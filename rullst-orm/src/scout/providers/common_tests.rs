use super::{client, endpoint, json_response, parse_positive_id, validate_credential};
use reqwest::StatusCode;
use serde_json::{Value, json};

#[test]
fn credentials_are_bounded_and_printable() {
    assert!(validate_credential("api key", &"k".repeat(2_048)).is_ok());
    assert!(validate_credential("api key", &"k".repeat(2_049)).is_err());
    assert!(validate_credential("api key", "line\nbreak").is_err());
}

#[test]
fn endpoints_are_bare_origins_over_https_or_loopback_http() {
    assert!(endpoint("https://search.example.com").is_ok());
    assert!(endpoint("http://127.0.0.1:7700/").is_ok());
    assert!(endpoint("http://search.example.com").is_err());
    let origin = "Validation error: Scout endpoint must be an origin URL without credentials, path, query or fragment";
    for invalid in [
        "https://user@search.example.com",
        "https://:pw@search.example.com",
        "https://search.example.com?x=1",
        "https://search.example.com#x",
        "https://search.example.com/indexes",
    ] {
        assert_eq!(
            endpoint(invalid).unwrap_err().to_string(),
            origin,
            "{invalid}"
        );
    }
}

#[test]
fn document_ids_must_be_positive() {
    assert_eq!(parse_positive_id(&json!(1), "Test").unwrap(), 1);
    assert_eq!(parse_positive_id(&json!("7"), "Test").unwrap(), 7);
    for invalid in [json!(0), json!("0"), json!(-1), json!(i64::MAX), json!("x")] {
        assert!(parse_positive_id(&invalid, "Test").is_err(), "{invalid}");
    }
}

fn response(body: Vec<u8>) -> reqwest::Response {
    reqwest::Response::from(axum::http::Response::new(body))
}

#[tokio::test]
async fn responses_are_read_up_to_exactly_four_mebibytes() {
    let limit = 4 * 1_048_576;
    let exact = format!("\"{}\"", "a".repeat(limit - 2)).into_bytes();
    let value = json_response("Test", response(exact), &[StatusCode::OK])
        .await
        .expect("a 4 MiB body is accepted");
    assert_eq!(value.as_str().map(str::len), Some(limit - 2));
    let over = format!("\"{}\"", "a".repeat(limit - 1)).into_bytes();
    assert!(
        json_response("Test", response(over), &[StatusCode::OK])
            .await
            .is_err()
    );
    assert_eq!(
        json_response("Test", response(Vec::new()), &[StatusCode::OK])
            .await
            .unwrap(),
        Value::Null
    );
}

#[tokio::test]
async fn the_client_never_follows_redirects() {
    use axum::{Router, http::header, response::IntoResponse, routing::get};
    async fn moved() -> impl IntoResponse {
        (StatusCode::FOUND, [(header::LOCATION, "/target")])
    }
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let router = Router::new()
        .route("/moved", get(moved))
        .route("/target", get(|| async { "followed" }));
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let status = client()
        .unwrap()
        .get(format!("http://{address}/moved"))
        .send()
        .await
        .unwrap()
        .status();
    server.abort();
    assert_eq!(status, StatusCode::FOUND);
}
