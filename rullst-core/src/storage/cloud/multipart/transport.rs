use super::super::{CloudClient, CloudError, client::Backend, signing};
use super::{MultipartError, MultipartLimits};
use reqwest::{Method, header::HeaderMap};
use ring::digest::{Context, SHA256};
use std::time::SystemTime;

pub(crate) struct Wire {
    pub status: u16,
    pub headers: HeaderMap,
    pub body: Vec<u8>,
}

impl CloudClient {
    pub(crate) fn multipart_binding(&self, key: &str, limits: &MultipartLimits) -> [u8; 32] {
        let mut digest = Context::new(&SHA256);
        let incarnation = match &self.backend {
            Backend::Mock(store) => store.incarnation().to_string(),
            Backend::Live(_) => String::new(),
        };
        for value in [
            "rullst.multipart.v1",
            self.endpoint.as_str(),
            &self.bucket,
            &self.region,
            key,
            if self.is_mock() { "mock" } else { "live" },
            &incarnation,
            &limits.max_total.to_string(),
            &limits.part_bytes.to_string(),
            &limits.lifetime.as_secs().to_string(),
        ] {
            digest.update(&(value.len() as u64).to_be_bytes());
            digest.update(value.as_bytes());
        }
        let mut output = [0; 32];
        output.copy_from_slice(digest.finish().as_ref());
        output
    }

    pub(crate) async fn multipart_request(
        &self,
        method: Method,
        key: &str,
        query: &[(&str, String)],
        headers: HeaderMap,
        body: &[u8],
    ) -> Result<Wire, MultipartError> {
        signing::validate_key(key)?;
        if let Backend::Mock(store) = &self.backend {
            return store.multipart_request(method, &self.bucket, key, query, headers, body);
        }
        let Backend::Live(client) = &self.backend else {
            return Err(CloudError::MockUnavailable.into());
        };
        let mut url = signing::object_url(&self.endpoint, &self.bucket, key)?;
        url.query_pairs_mut()
            .extend_pairs(query.iter().map(|(k, v)| (*k, v.as_str())));
        // An empty query has no signature-relevant delimiters for HEAD.
        if query.is_empty() {
            url.set_query(None);
        }
        let headers = signing::headers_with_extra(
            &self.config,
            &self.region,
            &method,
            &url,
            body,
            SystemTime::now(),
            headers,
        )?;
        tokio::time::timeout(self.config.timeout, async {
            let mut request = client.request(method.clone(), url).headers(headers);
            if method == Method::PUT || method == Method::POST {
                request = request.body(body.to_vec());
            }
            let mut response = request.send().await.map_err(transport)?;
            let status = response.status().as_u16();
            let headers = response.headers().clone();
            let mut body = Vec::new();
            while let Some(chunk) = response.chunk().await.map_err(transport)? {
                if chunk.len() > (256 * 1024usize).saturating_sub(body.len()) {
                    return Err(MultipartError::InvalidResponse);
                }
                body.extend_from_slice(&chunk);
            }
            Ok(Wire {
                status,
                headers,
                body,
            })
        })
        .await
        .map_err(|_| CloudError::Timeout)?
    }
}

fn transport(error: reqwest::Error) -> MultipartError {
    if error.is_timeout() {
        CloudError::Timeout.into()
    } else {
        CloudError::Transport.into()
    }
}

impl Wire {
    pub(super) fn expect(&self, expected: u16) -> Result<(), MultipartError> {
        if self.status == expected {
            Ok(())
        } else if self.status == 404 {
            Err(CloudError::NotFound.into())
        } else {
            Err(CloudError::Rejected(self.status).into())
        }
    }
}
