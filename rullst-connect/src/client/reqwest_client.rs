use async_trait::async_trait;
use serde_json::Value;

use super::traits::{HttpClient, HttpRequest, HttpResponse};

/// The default reqwest-based implementation of `HttpClient`.
#[cfg(not(miri))]
pub struct ReqwestClient {
    #[cfg(not(feature = "retry"))]
    client: Result<reqwest::Client, String>,
    #[cfg(feature = "retry")]
    client: Result<reqwest_middleware::ClientWithMiddleware, String>,
}

#[cfg(miri)]
pub struct ReqwestClient {}

impl ReqwestClient {
    pub fn new() -> Self {
        #[cfg(miri)]
        {
            Self {}
        }
        #[cfg(not(miri))]
        {
            let reqwest_client = build_reqwest_client();

            #[cfg(feature = "retry")]
            {
                let retry_policy = reqwest_retry::policies::ExponentialBackoff::builder()
                    .build_with_max_retries(3);
                let client = reqwest_client.map(|reqwest_client| {
                    reqwest_middleware::ClientBuilder::new(reqwest_client)
                        .with(reqwest_retry::RetryTransientMiddleware::new_with_policy(
                            retry_policy,
                        ))
                        .build()
                });
                Self { client }
            }

            #[cfg(not(feature = "retry"))]
            Self {
                client: reqwest_client,
            }
        }
    }

    #[cfg(feature = "retry")]
    pub fn new_with_retry(max_retries: u32) -> Self {
        #[cfg(miri)]
        {
            let _ = max_retries;
            Self {}
        }
        #[cfg(not(miri))]
        {
            let reqwest_client = build_reqwest_client();

            let retry_policy = reqwest_retry::policies::ExponentialBackoff::builder()
                .build_with_max_retries(max_retries.min(10));
            let client = reqwest_client.map(|reqwest_client| {
                reqwest_middleware::ClientBuilder::new(reqwest_client)
                    .with(reqwest_retry::RetryTransientMiddleware::new_with_policy(
                        retry_policy,
                    ))
                    .build()
            });
            Self { client }
        }
    }
}

impl Default for ReqwestClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl HttpClient for ReqwestClient {
    #[tracing::instrument(skip(self, req), fields(method = %req.method, url = %req.url))]
    async fn execute(&self, req: HttpRequest) -> Result<HttpResponse, crate::error::ConnectError> {
        #[cfg(miri)]
        {
            let _ = &req;
            return Err(crate::error::ConnectError::Provider(
                "Network requests are not supported under Miri".to_string(),
            ));
        }

        #[cfg(not(miri))]
        {
            tracing::debug!("Executing HTTP request");
            let method = match req.method.as_ref() {
                "POST" => reqwest::Method::POST,
                _ => reqwest::Method::GET,
            };

            #[cfg(not(feature = "retry"))]
            let mut res = {
                let client = self.client.as_ref().map_err(|reason| {
                    crate::error::ConnectError::InvalidConfiguration {
                        field: "http_client",
                        reason: reason.clone(),
                    }
                })?;
                let mut builder = client.request(method, &req.url);

                builder = builder.headers(req.headers);

                if let Some(token) = &req.bearer_auth {
                    builder = builder.bearer_auth(token);
                }

                if let Some((user, pass)) = &req.basic_auth {
                    builder = builder.basic_auth(user, pass.as_deref());
                }

                if let Some(f) = req.form {
                    builder = builder.body(f).header(
                        reqwest::header::CONTENT_TYPE,
                        "application/x-www-form-urlencoded",
                    );
                } else if let Some(j) = req.json {
                    builder = builder.json(&j);
                }

                builder
                    .send()
                    .await
                    .map_err(crate::error::ConnectError::from)?
            };

            #[cfg(feature = "retry")]
            let mut res = {
                let client = self.client.as_ref().map_err(|reason| {
                    crate::error::ConnectError::InvalidConfiguration {
                        field: "http_client",
                        reason: reason.clone(),
                    }
                })?;
                let mut builder = client.request(method, &req.url);

                if !req.headers.is_empty() {
                    builder = builder.headers(req.headers);
                }

                if let Some(token) = &req.bearer_auth {
                    builder = builder.bearer_auth(token);
                }

                if let Some((user, pass)) = &req.basic_auth {
                    builder = builder.basic_auth(user, pass.as_deref());
                }

                if let Some(body) = req.form {
                    builder = builder.body(body).header(
                        reqwest::header::CONTENT_TYPE,
                        "application/x-www-form-urlencoded",
                    );
                } else if let Some(j) = req.json {
                    let body = serde_json::to_string(&j)
                        .map_err(|e| crate::error::ConnectError::Json(e.to_string()))?;
                    builder = builder
                        .body(body)
                        .header(reqwest::header::CONTENT_TYPE, "application/json");
                }

                builder.send().await.map_err(|e| {
                    if let reqwest_middleware::Error::Reqwest(err) = e {
                        crate::error::ConnectError::Reqwest(err.to_string())
                    } else {
                        crate::error::ConnectError::Provider(e.to_string())
                    }
                })?
            };
            let status = res.status().as_u16();
            tracing::debug!(status = %status, "Received HTTP response");

            let capacity = parse_content_length(res.headers()).unwrap_or(8192);

            const MAX_BODY_SIZE: usize = 2 * 1024 * 1024; // 2MB limit

            let mut body_bytes = Vec::with_capacity(capacity.min(MAX_BODY_SIZE));

            while let Some(chunk) = res
                .chunk()
                .await
                .map_err(crate::error::ConnectError::from)?
            {
                if body_bytes.len() + chunk.len() > MAX_BODY_SIZE {
                    return Err(crate::error::ConnectError::Provider(
                        "Response body size limit exceeded".to_string(),
                    ));
                }
                body_bytes.extend_from_slice(&chunk);
            }

            let body = match serde_json::from_slice(&body_bytes) {
                Ok(v) => v,
                Err(_) => {
                    let text = String::from_utf8(body_bytes).map_err(|e| {
                        crate::error::ConnectError::Provider(format!(
                            "Response body is not valid UTF-8: {}",
                            e
                        ))
                    })?;
                    Value::String(text)
                }
            };

            Ok(HttpResponse { status, body })
        }
    }
}

#[cfg(not(miri))]
fn build_reqwest_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        // Discovery and JWKS URLs are validated before dispatch. Following a redirect here
        // would allow a validated public URL to pivot into an unvalidated private endpoint.
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(10))
        .pool_idle_timeout(std::time::Duration::from_secs(90))
        .build()
        .map_err(|error| error.to_string())
}

#[cfg(not(miri))]
pub(crate) fn parse_content_length(headers: &reqwest::header::HeaderMap) -> Option<usize> {
    headers
        .get(reqwest::header::CONTENT_LENGTH)
        .map(|h| h.as_bytes())
        .and_then(|bytes| {
            bytes.iter().try_fold(0usize, |acc, &b| {
                if b.is_ascii_digit() {
                    Some(acc.saturating_mul(10).saturating_add((b - b'0') as usize))
                } else {
                    None
                }
            })
        })
}
