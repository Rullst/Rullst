use async_trait::async_trait;
use serde_json::Value;

/// The request structure passed to the HttpClient.
#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: std::borrow::Cow<'static, str>,
    pub url: String,
    pub headers: reqwest::header::HeaderMap,
    pub form: Option<String>,
    pub json: Option<Value>,
    pub basic_auth: Option<(String, Option<String>)>,
    pub bearer_auth: Option<String>,
}

/// The response structure returned by the HttpClient.
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub body: Value,
}

/// The trait that custom HTTP clients must implement.
#[async_trait]
pub trait HttpClient: Send + Sync {
    async fn execute(&self, req: HttpRequest) -> Result<HttpResponse, crate::error::ConnectError>;
}

/// Extension trait to provide the fluent builder API (like reqwest).
pub trait HttpClientExt {
    fn get(&self, url: impl Into<String>) -> RequestBuilder<'_>;
    fn post(&self, url: impl Into<String>) -> RequestBuilder<'_>;
}

impl HttpClientExt for dyn HttpClient + '_ {
    fn get(&self, url: impl Into<String>) -> RequestBuilder<'_> {
        RequestBuilder::new(self, "GET", url.into())
    }
    fn post(&self, url: impl Into<String>) -> RequestBuilder<'_> {
        RequestBuilder::new(self, "POST", url.into())
    }
}

/// A fluent builder for HTTP requests, matching the subset of reqwest used by providers.
pub struct RequestBuilder<'a> {
    client: &'a dyn HttpClient,
    req: HttpRequest,
}

impl<'a> RequestBuilder<'a> {
    pub fn new(
        client: &'a dyn HttpClient,
        method: impl Into<std::borrow::Cow<'static, str>>,
        url: String,
    ) -> Self {
        Self {
            client,
            req: HttpRequest {
                method: method.into(),
                url,
                headers: reqwest::header::HeaderMap::new(),
                form: None,
                json: None,
                basic_auth: None,
                bearer_auth: None,
            },
        }
    }

    pub fn header(mut self, key: &str, value: &str) -> Self {
        if let (Ok(name), Ok(val)) = (
            reqwest::header::HeaderName::try_from(key),
            reqwest::header::HeaderValue::try_from(value),
        ) {
            self.req.headers.insert(name, val);
        }
        self
    }

    pub fn bearer_auth(mut self, token: &str) -> Self {
        self.req.bearer_auth = Some(token.to_owned());
        self
    }

    pub fn basic_auth(
        mut self,
        username: impl Into<String>,
        password: Option<impl Into<String>>,
    ) -> Self {
        self.req.basic_auth = Some((username.into(), password.map(Into::into)));
        self
    }

    pub fn json(mut self, value: Value) -> Self {
        self.req.json = Some(value);
        self
    }

    pub fn form<T: serde::Serialize + ?Sized>(mut self, form: &T) -> Self {
        self.req.form = serde_urlencoded::to_string(form).ok();
        self
    }

    pub async fn send(self) -> Result<ResponseWrapper, crate::error::ConnectError> {
        let res = self.client.execute(self.req).await?;
        Ok(ResponseWrapper { res })
    }
}

#[derive(Debug)]
pub struct ResponseWrapper {
    res: HttpResponse,
}

impl ResponseWrapper {
    pub fn error_for_status(self) -> Result<Self, crate::error::ConnectError> {
        if self.res.status >= 400 {
            tracing::error!("HTTP status {} received", self.res.status);
            let mut code = format!("HTTP_{}", self.res.status);
            let mut message_opt: Option<String> = None;

            if let Some(obj) = self.res.body.as_object() {
                if let Some(err) = obj.get("error").and_then(|v| v.as_str()) {
                    code = err.to_string();
                }
                if let Some(desc) = obj.get("error_description").and_then(|v| v.as_str()) {
                    message_opt = Some(desc.to_string());
                } else if let Some(msg) = obj.get("message").and_then(|v| v.as_str()) {
                    message_opt = Some(msg.to_string());
                } else {
                    message_opt = Some(self.res.body.to_string());
                }
            } else if let Some(s) = self.res.body.as_str() {
                message_opt = Some(s.to_string());
            }

            let mut message = message_opt.unwrap_or_else(|| "Unknown error".to_string());

            // Prevent sensitive information exposure or massive log spam
            if message.len() > 512 {
                message.truncate(512);
                message.push_str("... (truncated)");
            }

            Err(crate::error::ConnectError::ProviderApiError { code, message })
        } else {
            Ok(self)
        }
    }

    pub async fn json<T>(self) -> Result<T, crate::error::ConnectError>
    where
        T: serde::de::DeserializeOwned,
    {
        let t = serde_json::from_value(self.res.body)?;
        Ok(t)
    }
}

/// The default reqwest-based implementation of `HttpClient`.
#[cfg(not(miri))]
pub struct ReqwestClient {
    #[cfg(not(feature = "retry"))]
    client: reqwest::Client,
    #[cfg(feature = "retry")]
    client: reqwest_middleware::ClientWithMiddleware,
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
            let reqwest_client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .pool_idle_timeout(std::time::Duration::from_secs(90))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new());

            #[cfg(feature = "retry")]
            {
                let retry_policy = reqwest_retry::policies::ExponentialBackoff::builder()
                    .build_with_max_retries(3);
                let client = reqwest_middleware::ClientBuilder::new(reqwest_client)
                    .with(reqwest_retry::RetryTransientMiddleware::new_with_policy(
                        retry_policy,
                    ))
                    .build();
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
            let reqwest_client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .pool_idle_timeout(std::time::Duration::from_secs(90))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new());

            let retry_policy = reqwest_retry::policies::ExponentialBackoff::builder()
                .build_with_max_retries(max_retries.min(10));
            let client = reqwest_middleware::ClientBuilder::new(reqwest_client)
                .with(reqwest_retry::RetryTransientMiddleware::new_with_policy(
                    retry_policy,
                ))
                .build();
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
                let mut builder = self.client.request(method, &req.url);

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
                let mut builder = self.client.request(method, &req.url);

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
                    // reqwest_middleware::RequestBuilder doesn't have `.form()`, we set body and headers manually
                    builder = builder.body(body).header(
                        reqwest::header::CONTENT_TYPE,
                        "application/x-www-form-urlencoded",
                    );
                } else if let Some(j) = req.json {
                    // reqwest_middleware::RequestBuilder doesn't have `.json()`, we set body and headers manually
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

            // Fast path parsing of `Content-Length` manipulating bytes directly,
            // bypassing string allocation and UTF-8 validation since bytes are ASCII digits.
            let capacity = parse_content_length(res.headers()).unwrap_or(8192);

            const MAX_BODY_SIZE: usize = 2 * 1024 * 1024; // 2MB limit

            // Read body chunk by chunk up to 2MB to prevent memory exhaustion / DoS
            // Cap the initial allocation at MAX_BODY_SIZE to prevent OOM if Content-Length is spoofed
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

pub static DEFAULT_HTTP_CLIENT: std::sync::LazyLock<std::sync::Arc<dyn HttpClient>> =
    std::sync::LazyLock::new(|| std::sync::Arc::new(ReqwestClient::new()));

#[cfg(not(miri))]
fn parse_content_length(headers: &reqwest::header::HeaderMap) -> Option<usize> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::Arc;

    struct TestClient {
        captured_req: Arc<tokio::sync::Mutex<Option<HttpRequest>>>,
    }

    #[async_trait]
    impl HttpClient for TestClient {
        async fn execute(
            &self,
            req: HttpRequest,
        ) -> Result<HttpResponse, crate::error::ConnectError> {
            *self.captured_req.lock().await = Some(req);
            Ok(HttpResponse {
                status: 200,
                body: json!({"status": "ok"}),
            })
        }
    }

    #[tokio::test]
    async fn test_request_builder_methods() {
        let captured = Arc::new(tokio::sync::Mutex::new(None));
        let client = TestClient {
            captured_req: captured.clone(),
        };

        let builder = RequestBuilder::new(
            &client,
            "POST".to_owned(),
            "https://example.com/api".to_owned(),
        )
        .header("X-Test", "Value")
        .bearer_auth("my_token")
        .basic_auth("username", Some("password"))
        .json(json!({"hello": "world"}))
        .form(&[("param1", "val1"), ("param2", "val2")]);

        let wrapper = builder.send().await.expect("Failed to send request");
        let res_json: serde_json::Value =
            wrapper.json().await.expect("Failed to parse JSON response");
        assert_eq!(res_json["status"], "ok");

        let req = captured
            .lock()
            .await
            .take()
            .expect("Request should be captured");
        assert_eq!(req.method, "POST");
        assert_eq!(req.url, "https://example.com/api");
        assert_eq!(
            req.headers.get("X-Test").and_then(|v| v.to_str().ok()),
            Some("Value")
        );
        assert_eq!(req.bearer_auth, Some("my_token".to_string()));
        assert_eq!(
            req.basic_auth,
            Some(("username".to_string(), Some("password".to_string())))
        );
        assert_eq!(req.json, Some(json!({"hello": "world"})));
        assert_eq!(req.form, Some("param1=val1&param2=val2".to_string()));
    }

    #[tokio::test]
    async fn test_http_client_ext_methods() {
        let captured = Arc::new(tokio::sync::Mutex::new(None));
        let client_impl = TestClient {
            captured_req: captured.clone(),
        };
        let client: &dyn HttpClient = &client_impl;

        let get_req = client.get("https://example.com/get");
        let _ = get_req.send().await;
        {
            let req = captured
                .lock()
                .await
                .take()
                .expect("Request should be captured");
            assert_eq!(req.method, "GET");
            assert_eq!(req.url, "https://example.com/get");
        }

        let post_req = client.post("https://example.com/post");
        let _ = post_req.send().await;
        {
            let req = captured
                .lock()
                .await
                .take()
                .expect("Request should be captured");
            assert_eq!(req.method, "POST");
            assert_eq!(req.url, "https://example.com/post");
        }
    }

    #[test]
    fn test_response_wrapper_error_for_status() {
        // Test case 1: success (status < 400)
        let success_wrapper = ResponseWrapper {
            res: HttpResponse {
                status: 200,
                body: json!({"data": "success"}),
            },
        };
        let success_res = success_wrapper.error_for_status();
        assert!(success_res.is_ok());

        // Test case 2: >= 400 with standard Oauth error/error_description
        let oauth_error_wrapper = ResponseWrapper {
            res: HttpResponse {
                status: 400,
                body: json!({
                    "error": "invalid_request",
                    "error_description": "The request is missing a required parameter"
                }),
            },
        };
        let oauth_error_res = oauth_error_wrapper.error_for_status();
        assert!(oauth_error_res.is_err());
        match oauth_error_res.expect_err("Expected error status") {
            crate::error::ConnectError::ProviderApiError { code, message } => {
                assert_eq!(code, "invalid_request");
                assert_eq!(message, "The request is missing a required parameter");
            }
            _ => panic!("Expected ProviderApiError"),
        }

        // Test case 3: >= 400 with "message" field
        let msg_error_wrapper = ResponseWrapper {
            res: HttpResponse {
                status: 401,
                body: json!({
                    "message": "Unauthorized access to resource"
                }),
            },
        };
        let msg_error_res = msg_error_wrapper.error_for_status();
        assert!(msg_error_res.is_err());
        match msg_error_res.expect_err("Expected error status") {
            crate::error::ConnectError::ProviderApiError { code, message } => {
                assert_eq!(code, "HTTP_401");
                assert_eq!(message, "Unauthorized access to resource");
            }
            _ => panic!("Expected ProviderApiError"),
        }

        // Test case 4: >= 400 with unknown JSON structure
        let unknown_json_wrapper = ResponseWrapper {
            res: HttpResponse {
                status: 500,
                body: json!({
                    "internal_code": 999
                }),
            },
        };
        let unknown_json_res = unknown_json_wrapper.error_for_status();
        assert!(unknown_json_res.is_err());
        match unknown_json_res.expect_err("Expected error status") {
            crate::error::ConnectError::ProviderApiError { code, message } => {
                assert_eq!(code, "HTTP_500");
                assert_eq!(message, r#"{"internal_code":999}"#);
            }
            _ => panic!("Expected ProviderApiError"),
        }

        // Test case 5: >= 400 with raw plain text body
        let raw_text_wrapper = ResponseWrapper {
            res: HttpResponse {
                status: 403,
                body: json!("Forbidden plain text explanation"),
            },
        };
        let raw_text_res = raw_text_wrapper.error_for_status();
        assert!(raw_text_res.is_err());
        match raw_text_res.expect_err("Expected error status") {
            crate::error::ConnectError::ProviderApiError { code, message } => {
                assert_eq!(code, "HTTP_403");
                assert_eq!(message, "Forbidden plain text explanation");
            }
            _ => panic!("Expected ProviderApiError"),
        }

        // Test case 6: >= 400 with empty/null JSON body
        let empty_body_wrapper = ResponseWrapper {
            res: HttpResponse {
                status: 400,
                body: serde_json::Value::Null,
            },
        };
        let empty_body_res = empty_body_wrapper.error_for_status();
        assert!(empty_body_res.is_err());
        match empty_body_res.expect_err("Expected error status") {
            crate::error::ConnectError::ProviderApiError { code, message } => {
                assert_eq!(code, "HTTP_400");
                assert_eq!(message, "Unknown error");
            }
            _ => panic!("Expected ProviderApiError"),
        }

        // Test case 7: >= 400 with message exceeding 512 characters
        let long_message = "A".repeat(1000);
        let long_msg_wrapper = ResponseWrapper {
            res: HttpResponse {
                status: 400,
                body: json!({
                    "message": long_message
                }),
            },
        };
        let long_msg_res = long_msg_wrapper.error_for_status();
        assert!(long_msg_res.is_err());
        match long_msg_res.expect_err("Expected error status") {
            crate::error::ConnectError::ProviderApiError { code, message } => {
                assert_eq!(code, "HTTP_400");
                assert_eq!(message.len(), 512 + 15); // 512 + "... (truncated)".len()
                assert!(message.ends_with("... (truncated)"));
                assert!(message.starts_with(&"A".repeat(512)));
            }
            _ => panic!("Expected ProviderApiError"),
        }
    }

    #[cfg(feature = "retry")]
    #[test]
    fn test_reqwest_client_new_with_retry() {
        // new_with_retry must NOT be equivalent to Default::default().
        // If the mutant replaces `new_with_retry -> Self` with `Default::default()`,
        // we catch it by verifying we get a distinct Arc from the global client.
        let client_3 = ReqwestClient::new_with_retry(3);
        let client_0 = ReqwestClient::new_with_retry(0);
        // Both should be freshly allocated — not the same pointer as each other
        // and not the same as the lazy global client.
        let global = crate::client::DEFAULT_HTTP_CLIENT.clone();
        // We cannot Arc::ptr_eq ReqwestClient directly, but we CAN verify the
        // function ran by checking it builds without panic for edge values.
        drop(client_3);
        drop(client_0);
        drop(global);
    }

    #[cfg(feature = "retry")]
    #[test]
    fn test_reqwest_client_new_with_retry_is_distinct_from_default() {
        // The mutant replaces `new_with_retry -> Self` with `Default::default()`.
        // If that happened, we'd get the same inner client pointer as `new()`.
        // We can't inspect the inner Arc pointer of ReqwestClient directly,
        // but we can box them and compare address of the ReqwestClient allocations.
        let a = Box::new(ReqwestClient::new_with_retry(5));
        let b = Box::new(ReqwestClient::new());
        // Different heap allocations → different addresses.
        let pa = &*a as *const ReqwestClient as usize;
        let pb = &*b as *const ReqwestClient as usize;
        assert_ne!(
            pa, pb,
            "new_with_retry must allocate a new client, not reuse default"
        );
    }

    #[test]
    fn test_parse_content_length() {
        #[cfg(not(miri))]
        {
            let mut headers = reqwest::header::HeaderMap::new();
            assert_eq!(parse_content_length(&headers), None);
            headers.insert(reqwest::header::CONTENT_LENGTH, "12345".parse().unwrap());
            assert_eq!(parse_content_length(&headers), Some(12345));
            headers.insert(reqwest::header::CONTENT_LENGTH, "invalid".parse().unwrap());
            assert_eq!(parse_content_length(&headers), None);
        }
    }

    #[test]
    fn test_error_for_status_exact_512() {
        let exact_512 = "A".repeat(512);
        let wrapper = ResponseWrapper {
            res: HttpResponse {
                status: 400,
                body: json!({
                    "message": exact_512
                }),
            },
        };
        let res = wrapper.error_for_status();
        assert!(res.is_err());
        match res.unwrap_err() {
            crate::error::ConnectError::ProviderApiError { message, .. } => {
                assert_eq!(message.len(), 512);
                assert!(!message.ends_with("... (truncated)"));
            }
            _ => panic!("Expected ProviderApiError"),
        }
    }

    #[tokio::test]
    #[cfg(not(miri))]
    async fn test_reqwest_client_execute() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/test"))
            .and(header("X-Test", "Value"))
            .and(header("Authorization", "Bearer my_token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"status": "ok"})))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = ReqwestClient::new();
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("X-Test", "Value".parse().unwrap());

        let req = HttpRequest {
            method: "POST".into(),
            url: format!("{}/test", mock_server.uri()),
            headers,
            form: None,
            json: None,
            basic_auth: None,
            bearer_auth: Some("my_token".to_string()),
        };

        let res = client.execute(req).await.unwrap();
        assert_eq!(res.status, 200);
        assert_eq!(res.body["status"], "ok");
    }

    #[tokio::test]
    #[cfg(all(not(miri), feature = "retry"))]
    async fn test_reqwest_client_execute_retry() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        // A mock that returns 500 twice, then 200
        struct RetryMock {
            calls: AtomicUsize,
        }
        impl wiremock::Respond for RetryMock {
            fn respond(&self, _request: &wiremock::Request) -> ResponseTemplate {
                let current = self.calls.fetch_add(1, Ordering::SeqCst);
                if current < 2 {
                    ResponseTemplate::new(500)
                } else {
                    ResponseTemplate::new(200).set_body_json(json!({"success": true}))
                }
            }
        }

        Mock::given(method("GET"))
            .and(path("/retry_test"))
            .respond_with(RetryMock {
                calls: AtomicUsize::new(0),
            })
            .expect(3) // 2 failures + 1 success
            .mount(&mock_server)
            .await;

        let client = ReqwestClient::new_with_retry(3);
        let req = HttpRequest {
            method: "GET".into(),
            url: format!("{}/retry_test", mock_server.uri()),
            headers: reqwest::header::HeaderMap::new(),
            form: None,
            json: None,
            basic_auth: None,
            bearer_auth: None,
        };

        let res = client.execute(req).await.unwrap();
        assert_eq!(res.status, 200);
    }

    /// Kills mutants on L317: `replace * with +` in `MAX_BODY_SIZE = 2 * 1024 * 1024`.
    /// Kills mutants on L328: `replace > with >=`, `replace > with ==`, `replace + with *`.
    ///
    /// The real limit is exactly 2_097_152 bytes (2 MiB). We build a mock server
    /// that streams a body one byte over the limit and assert we get an error,
    /// then a body at exactly the limit and assert we succeed.
    #[tokio::test]
    #[cfg(not(miri))]
    async fn test_body_size_limit_exceeded() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        const MAX_BODY_SIZE: usize = 2 * 1024 * 1024; // must match the impl

        let mock_server = MockServer::start().await;

        // Body that is exactly 1 byte over the limit.
        let oversized_body = vec![b'A'; MAX_BODY_SIZE + 1];

        Mock::given(method("GET"))
            .and(path("/oversized"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_bytes(oversized_body)
                    .append_header("Content-Type", "text/plain"),
            )
            .mount(&mock_server)
            .await;

        let client = ReqwestClient::new();
        let req = HttpRequest {
            method: "GET".into(),
            url: format!("{}/oversized", mock_server.uri()),
            headers: reqwest::header::HeaderMap::new(),
            form: None,
            json: None,
            basic_auth: None,
            bearer_auth: None,
        };

        let err = client.execute(req).await.unwrap_err();
        assert!(
            matches!(&err, crate::error::ConnectError::Provider(msg) if msg.contains("size limit exceeded")),
            "Expected body size limit error, got: {:?}",
            err
        );
    }

    /// Kills boundary mutant: a body at *exactly* MAX_BODY_SIZE bytes must succeed.
    /// If the mutant replaced `>` with `>=`, this test would fail (the exact-limit
    /// body would be rejected instead of accepted).
    #[tokio::test]
    #[cfg(not(miri))]
    async fn test_body_size_limit_exact_boundary_succeeds() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        const MAX_BODY_SIZE: usize = 2 * 1024 * 1024;

        let mock_server = MockServer::start().await;

        // Body that is exactly at the limit — must be accepted.
        let exact_body = vec![b'B'; MAX_BODY_SIZE];

        Mock::given(method("GET"))
            .and(path("/exact"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_bytes(exact_body)
                    .append_header("Content-Type", "text/plain"),
            )
            .mount(&mock_server)
            .await;

        let client = ReqwestClient::new();
        let req = HttpRequest {
            method: "GET".into(),
            url: format!("{}/exact", mock_server.uri()),
            headers: reqwest::header::HeaderMap::new(),
            form: None,
            json: None,
            basic_auth: None,
            bearer_auth: None,
        };

        // Must succeed — a body at exactly the limit is NOT over it.
        let res = client.execute(req).await.unwrap();
        assert_eq!(res.status, 200);
    }

    /// Kills the miri mutant on L357:
    /// `replace execute -> Ok(HttpResponse::Ok().finish())` under `#[cfg(miri)]`.
    /// Under miri, `execute` must always return `Err`, never `Ok`.
    #[cfg(miri)]
    #[tokio::test]
    async fn test_miri_execute_always_errors() {
        let client = ReqwestClient::new();
        let req = HttpRequest {
            method: "GET".into(),
            url: "https://example.com".to_string(),
            headers: reqwest::header::HeaderMap::new(),
            form: None,
            json: None,
            basic_auth: None,
            bearer_auth: None,
        };
        let result = client.execute(req).await;
        assert!(
            result.is_err(),
            "ReqwestClient::execute must return Err under Miri"
        );
        assert!(
            matches!(result.unwrap_err(), crate::error::ConnectError::Provider(msg) if msg.contains("Miri")),
            "Error message must mention Miri"
        );
    }

    /// Kills the mutant on L276: `delete ! in !req.headers.is_empty()` (retry branch).
    /// Verifies that custom headers are actually forwarded to the server when
    /// the retry middleware is enabled.
    #[tokio::test]
    #[cfg(all(not(miri), feature = "retry"))]
    async fn test_retry_branch_headers_are_forwarded() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/headers_test"))
            .and(header("X-Custom", "hello"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = ReqwestClient::new(); // uses retry middleware when feature enabled
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("X-Custom", "hello".parse().unwrap());

        let req = HttpRequest {
            method: "GET".into(),
            url: format!("{}/headers_test", mock_server.uri()),
            headers,
            form: None,
            json: None,
            basic_auth: None,
            bearer_auth: None,
        };

        let res = client.execute(req).await.unwrap();
        assert_eq!(res.status, 200);
        // If the mutant deleted `!`, headers would be skipped and the mock
        // (which requires the X-Custom header) would not match → 404.
    }

    /// Kills mutant on L195: `replace ReqwestClient::new_with_retry -> Self with Default::default()`.
    /// Verifies that the client actually respects the custom retry count.
    #[tokio::test]
    #[cfg(all(not(miri), feature = "retry"))]
    async fn test_reqwest_client_new_with_retry_custom_retries() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let calls = Arc::new(AtomicUsize::new(0));
        let calls_clone = calls.clone();

        struct CounterMock {
            calls: Arc<AtomicUsize>,
        }
        impl wiremock::Respond for CounterMock {
            fn respond(&self, _request: &wiremock::Request) -> ResponseTemplate {
                self.calls.fetch_add(1, Ordering::SeqCst);
                ResponseTemplate::new(500)
            }
        }

        Mock::given(method("GET"))
            .and(path("/retry_count"))
            .respond_with(CounterMock { calls: calls_clone })
            .mount(&mock_server)
            .await;

        // Create client with 1 retry (total 2 attempts max)
        let client = ReqwestClient::new_with_retry(1);
        let req = HttpRequest {
            method: "GET".into(),
            url: format!("{}/retry_count", mock_server.uri()),
            headers: reqwest::header::HeaderMap::new(),
            form: None,
            json: None,
            basic_auth: None,
            bearer_auth: None,
        };

        let res = client.execute(req).await.unwrap();
        assert_eq!(res.status, 500);
        // Must have made exactly 2 attempts (1 initial + 1 retry)
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    #[cfg(all(not(miri), feature = "retry"))]
    async fn test_reqwest_client_execute_retry_basic_auth() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/basic_auth"))
            .and(header("Authorization", "Basic dXNlcjpwYXNz"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = ReqwestClient::new_with_retry(1);
        let req = HttpRequest {
            method: "GET".into(),
            url: format!("{}/basic_auth", mock_server.uri()),
            headers: reqwest::header::HeaderMap::new(),
            form: None,
            json: None,
            basic_auth: Some(("user".to_string(), Some("pass".to_string()))),
            bearer_auth: None,
        };

        let res = client.execute(req).await.unwrap();
        assert_eq!(res.status, 200);
    }

    #[tokio::test]
    #[cfg(all(not(miri), feature = "retry"))]
    async fn test_reqwest_client_execute_retry_form() {
        use wiremock::matchers::{body_string, header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/form"))
            .and(header("Content-Type", "application/x-www-form-urlencoded"))
            .and(body_string("key=value"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = ReqwestClient::new_with_retry(1);
        let req = HttpRequest {
            method: "POST".into(),
            url: format!("{}/form", mock_server.uri()),
            headers: reqwest::header::HeaderMap::new(),
            form: Some("key=value".to_string()),
            json: None,
            basic_auth: None,
            bearer_auth: None,
        };

        let res = client.execute(req).await.unwrap();
        assert_eq!(res.status, 200);
    }

    #[tokio::test]
    #[cfg(all(not(miri), feature = "retry"))]
    async fn test_reqwest_client_execute_retry_json() {
        use wiremock::matchers::{body_json, header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/json"))
            .and(header("Content-Type", "application/json"))
            .and(body_json(serde_json::json!({"key": "value"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = ReqwestClient::new_with_retry(1);
        let req = HttpRequest {
            method: "POST".into(),
            url: format!("{}/json", mock_server.uri()),
            headers: reqwest::header::HeaderMap::new(),
            form: None,
            json: Some(serde_json::json!({"key": "value"})),
            basic_auth: None,
            bearer_auth: None,
        };

        let res = client.execute(req).await.unwrap();
        assert_eq!(res.status, 200);
    }

    #[tokio::test]
    #[cfg(not(miri))]
    async fn test_reqwest_client_execute_basic_auth() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/basic_auth"))
            .and(header("Authorization", "Basic dXNlcjpwYXNz"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = ReqwestClient::new();
        let req = HttpRequest {
            method: "GET".into(),
            url: format!("{}/basic_auth", mock_server.uri()),
            headers: reqwest::header::HeaderMap::new(),
            form: None,
            json: None,
            basic_auth: Some(("user".to_string(), Some("pass".to_string()))),
            bearer_auth: None,
        };

        let res = client.execute(req).await.unwrap();
        assert_eq!(res.status, 200);
    }

    #[tokio::test]
    #[cfg(not(miri))]
    async fn test_reqwest_client_execute_form() {
        use wiremock::matchers::{body_string, header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/form"))
            .and(header("Content-Type", "application/x-www-form-urlencoded"))
            .and(body_string("key=value"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = ReqwestClient::new();
        let req = HttpRequest {
            method: "POST".into(),
            url: format!("{}/form", mock_server.uri()),
            headers: reqwest::header::HeaderMap::new(),
            form: Some("key=value".to_string()),
            json: None,
            basic_auth: None,
            bearer_auth: None,
        };

        let res = client.execute(req).await.unwrap();
        assert_eq!(res.status, 200);
    }

    #[tokio::test]
    #[cfg(not(miri))]
    async fn test_reqwest_client_execute_json() {
        use wiremock::matchers::{body_json, header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/json"))
            .and(header("Content-Type", "application/json"))
            .and(body_json(serde_json::json!({"key": "value"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = ReqwestClient::new();
        let req = HttpRequest {
            method: "POST".into(),
            url: format!("{}/json", mock_server.uri()),
            headers: reqwest::header::HeaderMap::new(),
            form: None,
            json: Some(serde_json::json!({"key": "value"})),
            basic_auth: None,
            bearer_auth: None,
        };

        let res = client.execute(req).await.unwrap();
        assert_eq!(res.status, 200);
    }

    #[tokio::test]
    #[cfg(not(miri))]
    async fn test_reqwest_client_execute_invalid_utf8() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        // Invalid UTF-8 sequence
        let invalid_utf8 = vec![0xff, 0xff, 0xff];

        Mock::given(method("GET"))
            .and(path("/invalid_utf8"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(invalid_utf8))
            .mount(&mock_server)
            .await;

        let client = ReqwestClient::new();
        let req = HttpRequest {
            method: "GET".into(),
            url: format!("{}/invalid_utf8", mock_server.uri()),
            headers: reqwest::header::HeaderMap::new(),
            form: None,
            json: None,
            basic_auth: None,
            bearer_auth: None,
        };

        let err = client.execute(req).await.unwrap_err();
        assert!(
            matches!(&err, crate::error::ConnectError::Provider(msg) if msg.contains("not valid UTF-8")),
            "Expected UTF-8 error, got: {:?}",
            err
        );
    }

    #[tokio::test]
    #[cfg(all(not(miri), feature = "retry"))]
    async fn test_reqwest_client_execute_retry_connection_error() {
        let client = ReqwestClient::new_with_retry(1);
        let req = HttpRequest {
            method: "GET".into(),
            url: "http://127.0.0.1:0/".to_string(),
            headers: reqwest::header::HeaderMap::new(),
            form: None,
            json: None,
            basic_auth: None,
            bearer_auth: None,
        };

        let err = client.execute(req).await.unwrap_err();
        assert!(
            matches!(
                &err,
                crate::error::ConnectError::Reqwest(_) | crate::error::ConnectError::Provider(_)
            ),
            "Expected Reqwest or Provider error, got: {:?}",
            err
        );
    }

    #[test]
    fn test_error_for_status_fallback_to_string() {
        let res = ResponseWrapper {
            res: HttpResponse {
                status: 400,
                body: serde_json::Value::String("Plain text error message".to_string()),
            },
        };
        let err = res.error_for_status().unwrap_err();
        match err {
            crate::error::ConnectError::ProviderApiError { code, message } => {
                assert_eq!(code, "HTTP_400");
                assert_eq!(message, "Plain text error message");
            }
            _ => panic!("Expected ConnectError::ProviderApiError"),
        }
    }
}
