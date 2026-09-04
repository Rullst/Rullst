use async_trait::async_trait;
use secrecy::{ExposeSecret, SecretString};
#[cfg(not(miri))]
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
    /// Creates the default client with redirects disabled and a bounded request timeout.
    pub fn new() -> Self {
        #[cfg(miri)]
        {
            Self {}
        }
        #[cfg(not(miri))]
        {
            Self::from_client_result(build_reqwest_client(None), 3)
        }
    }

    /// Creates a client that sends HTTP and HTTPS provider traffic through one explicit proxy.
    ///
    /// Only authority-only HTTP(S) endpoints are accepted. Credentials embedded in the URL are
    /// rejected; use [`Self::try_with_proxy_basic_auth`] when proxy authentication is required.
    pub fn try_with_proxy(proxy_url: impl Into<String>) -> Result<Self, crate::ConnectError> {
        let proxy_url = validate_proxy_url(proxy_url.into(), false)?;

        #[cfg(miri)]
        {
            let _ = proxy_url;
            Ok(Self {})
        }
        #[cfg(not(miri))]
        {
            let proxy = reqwest::Proxy::all(proxy_url.as_str()).map_err(|_| proxy_build_error())?;
            Self::try_from_proxy(proxy)
        }
    }

    /// Creates an explicitly authenticated corporate-proxy client.
    ///
    /// Authenticated proxies must use HTTPS, except for exact loopback hosts used by local tests
    /// and development. User information in `proxy_url` is rejected so configuration errors do
    /// not echo credentials from a URL.
    pub fn try_with_proxy_basic_auth(
        proxy_url: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Result<Self, crate::ConnectError> {
        let proxy_url = validate_proxy_url(proxy_url.into(), true)?;
        let username = username.into();
        let password = SecretString::from(password.into());
        validate_proxy_credentials(&username, password.expose_secret())?;

        #[cfg(miri)]
        {
            let _ = (proxy_url, username, password);
            Ok(Self {})
        }
        #[cfg(not(miri))]
        {
            let proxy = reqwest::Proxy::all(proxy_url.as_str())
                .map_err(|_| proxy_build_error())?
                .basic_auth(&username, password.expose_secret());
            Self::try_from_proxy(proxy)
        }
    }

    #[cfg(feature = "retry")]
    /// Creates a client with an explicit bounded transient-retry policy.
    pub fn new_with_retry(max_retries: u32) -> Self {
        #[cfg(miri)]
        {
            let _ = max_retries;
            Self {}
        }
        #[cfg(not(miri))]
        {
            Self::from_client_result(build_reqwest_client(None), max_retries)
        }
    }

    #[cfg(not(miri))]
    fn try_from_proxy(proxy: reqwest::Proxy) -> Result<Self, crate::ConnectError> {
        let client = build_reqwest_client(Some(proxy)).map_err(|_| proxy_build_error())?;
        Ok(Self::from_client_result(Ok(client), 3))
    }

    #[cfg(not(miri))]
    fn from_client_result(
        reqwest_client: Result<reqwest::Client, String>,
        max_retries: u32,
    ) -> Self {
        #[cfg(feature = "retry")]
        {
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

        #[cfg(not(feature = "retry"))]
        {
            let _ = max_retries;
            Self {
                client: reqwest_client,
            }
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
            let method = reqwest::Method::from_bytes(req.method.as_bytes()).map_err(|_| {
                crate::error::ConnectError::InvalidConfiguration {
                    field: "http_method",
                    reason: "HTTP method is invalid".to_string(),
                }
            })?;

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
fn build_reqwest_client(proxy: Option<reqwest::Proxy>) -> Result<reqwest::Client, String> {
    let mut builder = reqwest::Client::builder()
        // Discovery and JWKS URLs are validated before dispatch. Following a redirect here
        // would allow a validated public URL to pivot into an unvalidated private endpoint.
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(10))
        .pool_idle_timeout(std::time::Duration::from_secs(90));
    if let Some(proxy) = proxy {
        // Supplying one explicit proxy also disables reqwest's ambient system-proxy lookup.
        builder = builder.proxy(proxy);
    }
    builder.build().map_err(|error| error.to_string())
}

fn validate_proxy_url(
    proxy_url: String,
    authenticated: bool,
) -> Result<url::Url, crate::ConnectError> {
    const MAX_PROXY_URL_LEN: usize = 2_048;
    if proxy_url.is_empty() || proxy_url.len() > MAX_PROXY_URL_LEN {
        return Err(proxy_config_error("proxy URL must contain 1 to 2048 bytes"));
    }

    let parsed =
        url::Url::parse(&proxy_url).map_err(|_| proxy_config_error("proxy URL is invalid"))?;
    if !matches!(parsed.scheme(), "http" | "https") || parsed.host().is_none() {
        return Err(proxy_config_error(
            "proxy URL must use HTTP or HTTPS and include a host",
        ));
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(proxy_config_error(
            "proxy URL must not contain embedded credentials",
        ));
    }
    if parsed.path() != "/" || parsed.query().is_some() || parsed.fragment().is_some() {
        return Err(proxy_config_error(
            "proxy URL must contain only scheme and authority",
        ));
    }
    if authenticated && parsed.scheme() != "https" && !is_loopback_host(&parsed) {
        return Err(proxy_config_error(
            "authenticated proxy must use HTTPS outside exact loopback hosts",
        ));
    }
    Ok(parsed)
}

fn validate_proxy_credentials(username: &str, password: &str) -> Result<(), crate::ConnectError> {
    if username.is_empty() || username.len() > 256 || password.len() > 1_024 {
        return Err(proxy_config_error(
            "proxy credentials exceed the accepted size boundary",
        ));
    }
    if username.contains(['\r', '\n']) || password.contains(['\r', '\n']) {
        return Err(proxy_config_error(
            "proxy credentials contain forbidden CR/LF characters",
        ));
    }
    Ok(())
}

fn is_loopback_host(url: &url::Url) -> bool {
    match url.host() {
        Some(url::Host::Domain(host)) => host.eq_ignore_ascii_case("localhost"),
        Some(url::Host::Ipv4(address)) => address.is_loopback(),
        Some(url::Host::Ipv6(address)) => address.is_loopback(),
        None => false,
    }
}

#[cfg(not(miri))]
fn proxy_build_error() -> crate::ConnectError {
    proxy_config_error("proxy client could not be constructed")
}

fn proxy_config_error(reason: impl Into<String>) -> crate::ConnectError {
    crate::ConnectError::InvalidConfiguration {
        field: "proxy_url",
        reason: reason.into(),
    }
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
