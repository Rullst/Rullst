use super::{
    CloudError, CloudStorageConfig, ObjectMetadata, SignedDownload, mock::MockStore, signing,
};
use crate::storage::StorageDriver;
use reqwest::{Method, Response, Url, header};
use std::{
    fmt,
    time::{Duration, SystemTime},
};

pub(crate) struct CloudClient {
    pub(super) config: CloudStorageConfig,
    pub(super) endpoint: Url,
    pub(super) bucket: String,
    pub(super) region: String,
    pub(super) backend: Backend,
}

pub(super) enum Backend {
    Live(reqwest::Client),
    Mock(MockStore),
}

impl fmt::Debug for CloudClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CloudClient")
            .field("mock", &self.is_mock())
            .finish_non_exhaustive()
    }
}

impl CloudClient {
    pub(crate) fn new(
        driver: &StorageDriver,
        config: CloudStorageConfig,
    ) -> Result<Self, CloudError> {
        let (bucket, region, endpoint) = match driver {
            StorageDriver::S3 { bucket, region } => {
                if region.len() < 3
                    || region.len() > 64
                    || !region
                        .bytes()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
                {
                    return Err(CloudError::Configuration);
                }
                let suffix = if region.starts_with("cn-") {
                    "amazonaws.com.cn"
                } else {
                    "amazonaws.com"
                };
                (
                    bucket.clone(),
                    region.clone(),
                    format!("https://s3.{region}.{suffix}/"),
                )
            }
            StorageDriver::R2 { bucket, account_id } => {
                if account_id.len() != 32
                    || !account_id
                        .bytes()
                        .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
                {
                    return Err(CloudError::Configuration);
                }
                (
                    bucket.clone(),
                    "auto".into(),
                    format!("https://{account_id}.r2.cloudflarestorage.com/"),
                )
            }
            StorageDriver::Local { .. } => return Err(CloudError::Configuration),
        };
        validate_bucket(&bucket)?;
        let endpoint = match &config.loopback_endpoint {
            Some(endpoint) => endpoint.clone(),
            None => Url::parse(&endpoint).map_err(|_| CloudError::Configuration)?,
        };
        let backend = if config.credentials.mock {
            Backend::Mock(MockStore::default())
        } else {
            let client = reqwest::Client::builder()
                .no_proxy()
                .no_gzip()
                .no_brotli()
                .no_deflate()
                .no_zstd()
                .redirect(reqwest::redirect::Policy::none())
                .https_only(config.loopback_endpoint.is_none())
                .timeout(config.timeout)
                .connect_timeout(config.timeout.min(Duration::from_secs(5)))
                .build()
                .map_err(|_| CloudError::Configuration)?;
            Backend::Live(client)
        };
        Ok(Self {
            config,
            endpoint,
            bucket,
            region,
            backend,
        })
    }

    pub(crate) fn is_mock(&self) -> bool {
        matches!(self.backend, Backend::Mock(_))
    }

    pub(crate) async fn put(&self, key: &str, bytes: &[u8]) -> Result<(), CloudError> {
        signing::validate_key(key)?;
        if bytes.len() > self.config.max_object_bytes {
            return Err(CloudError::SizeLimit);
        }
        if let Backend::Mock(store) = &self.backend {
            return store.put(key, bytes);
        }
        let response = self.request(Method::PUT, key, bytes).await?;
        status(&response, 200)?;
        Ok(())
    }

    pub(crate) async fn get(&self, key: &str) -> Result<Vec<u8>, CloudError> {
        signing::validate_key(key)?;
        if let Backend::Mock(store) = &self.backend {
            return store.get(key, self.config.max_object_bytes);
        }
        tokio::time::timeout(self.config.timeout, async {
            let mut response = self.request(Method::GET, key, &[]).await?;
            status(&response, 200)?;
            let expected = content_length(&response)?;
            if expected.is_some_and(|n| n > self.config.max_object_bytes as u64) {
                return Err(CloudError::SizeLimit);
            }
            let mut body = Vec::new();
            while let Some(chunk) = response.chunk().await.map_err(transport)? {
                if chunk.len() > self.config.max_object_bytes.saturating_sub(body.len()) {
                    return Err(CloudError::SizeLimit);
                }
                body.extend_from_slice(&chunk);
            }
            if expected.is_some_and(|n| n != body.len() as u64) {
                return Err(CloudError::InvalidResponse);
            }
            Ok(body)
        })
        .await
        .map_err(|_| CloudError::Timeout)?
    }

    pub(crate) async fn metadata(&self, key: &str) -> Result<ObjectMetadata, CloudError> {
        signing::validate_key(key)?;
        if let Backend::Mock(store) = &self.backend {
            return store.metadata(key);
        }
        let response = self.request(Method::HEAD, key, &[]).await?;
        status(&response, 200)?;
        let size_bytes = content_length(&response)?.ok_or(CloudError::InvalidResponse)?;
        let etag = response
            .headers()
            .get(header::ETAG)
            .map(|value| {
                let value = value.to_str().map_err(|_| CloudError::InvalidResponse)?;
                if value.len() > 256 || value.bytes().any(|b| b.is_ascii_control()) {
                    return Err(CloudError::InvalidResponse);
                }
                Ok(value.to_string())
            })
            .transpose()?;
        Ok(ObjectMetadata { size_bytes, etag })
    }

    pub(crate) async fn delete(&self, key: &str) -> Result<(), CloudError> {
        signing::validate_key(key)?;
        if let Backend::Mock(store) = &self.backend {
            return store.delete(key);
        }
        let response = self.request(Method::DELETE, key, &[]).await?;
        status(&response, 204)
    }

    pub(crate) fn signed_download(
        &self,
        key: &str,
        lifetime: Duration,
    ) -> Result<SignedDownload, CloudError> {
        let url = signing::object_url(&self.endpoint, &self.bucket, key)?;
        if self.is_mock() {
            return Err(CloudError::MockGrantUnsupported);
        }
        signing::signed_download(&self.config, &self.region, url, lifetime, SystemTime::now())
    }

    async fn request(
        &self,
        method: Method,
        key: &str,
        body: &[u8],
    ) -> Result<Response, CloudError> {
        let Backend::Live(client) = &self.backend else {
            return Err(CloudError::MockUnavailable);
        };
        let url = signing::object_url(&self.endpoint, &self.bucket, key)?;
        let headers = signing::headers(
            &self.config,
            &self.region,
            &method,
            &url,
            body,
            SystemTime::now(),
        )?;
        let mut request = client.request(method.clone(), url).headers(headers);
        if method == Method::PUT {
            request = request.body(body.to_vec());
        }
        request.send().await.map_err(transport)
    }
}

fn validate_bucket(bucket: &str) -> Result<(), CloudError> {
    if !(3..=63).contains(&bucket.len())
        || bucket.contains("..")
        || bucket.contains(".-")
        || bucket.contains("-.")
        || !bucket
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, b'-' | b'.'))
        || !bucket
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        || !bucket
            .as_bytes()
            .last()
            .is_some_and(u8::is_ascii_alphanumeric)
        || bucket.parse::<std::net::Ipv4Addr>().is_ok()
    {
        return Err(CloudError::Configuration);
    }
    Ok(())
}

fn status(response: &Response, expected: u16) -> Result<(), CloudError> {
    match response.status().as_u16() {
        actual if actual == expected => Ok(()),
        404 => Err(CloudError::NotFound),
        other => Err(CloudError::Rejected(other)),
    }
}

fn content_length(response: &Response) -> Result<Option<u64>, CloudError> {
    response
        .headers()
        .get(header::CONTENT_LENGTH)
        .map(|value| {
            value
                .to_str()
                .map_err(|_| CloudError::InvalidResponse)?
                .parse()
                .map_err(|_| CloudError::InvalidResponse)
        })
        .transpose()
}

fn transport(error: reqwest::Error) -> CloudError {
    if error.is_timeout() {
        CloudError::Timeout
    } else {
        CloudError::Transport
    }
}
