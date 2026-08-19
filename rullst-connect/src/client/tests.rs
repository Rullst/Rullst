use super::request_builder::{RequestBuilder, ResponseWrapper};
use super::reqwest_client::{parse_content_length, ReqwestClient};
use super::traits::{HttpClient, HttpClientExt, HttpRequest, HttpResponse};
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

struct TestClient {
    captured_req: Arc<tokio::sync::Mutex<Option<HttpRequest>>>,
}

#[async_trait]
impl HttpClient for TestClient {
    async fn execute(&self, req: HttpRequest) -> Result<HttpResponse, crate::error::ConnectError> {
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
    let res_json: serde_json::Value = wrapper.json().await.expect("Failed to parse JSON response");
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
    let client_3 = ReqwestClient::new_with_retry(3);
    let client_0 = ReqwestClient::new_with_retry(0);
    let global = crate::client::DEFAULT_HTTP_CLIENT.clone();
    drop(client_3);
    drop(client_0);
    drop(global);
}

#[cfg(feature = "retry")]
#[test]
fn test_reqwest_client_new_with_retry_is_distinct_from_default() {
    let a = Box::new(ReqwestClient::new_with_retry(5));
    let b = Box::new(ReqwestClient::new());
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

    let res = client.execute(req).await.unwrap();
    assert_eq!(res.status, 200);
}

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

    let client = ReqwestClient::new();
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
}

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
