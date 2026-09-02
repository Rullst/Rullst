use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use rullst::client_contract::{
    CURRENT_CLIENT_CONTRACT_VERSION, ClientContractPolicy, ClientRequest, RequestId, ServerFailure,
};
use rullst::{Router, server_function};
use serde::{Deserialize, Serialize};
use tower::ServiceExt;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Sum {
    value: u32,
}

#[server_function(path = "/api/rpc/tests/sum")]
async fn sum(left: u32, right: u32) -> rullst::rpc::RpcResult<Sum> {
    Ok(Sum {
        value: left.saturating_add(right),
    })
}

#[server_function(path = "/api/rpc/tests/reject")]
async fn reject() -> rullst::rpc::RpcResult<()> {
    Err(
        rullst::rpc::RpcFailure::application("counter.denied", false)
            .expect("static test failure code"),
    )
}

fn encoded_request<T: Serialize>(payload: T, request_id: &str) -> Vec<u8> {
    let policy = ClientContractPolicy::default();
    let request = ClientRequest::new(
        CURRENT_CLIENT_CONTRACT_VERSION,
        RequestId::new(request_id).expect("valid test request id"),
        payload,
    );
    policy.encode_request(&request).expect("encoded request")
}

fn post(path: &str, body: Vec<u8>) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .expect("test request")
}

async fn body(response: axum::response::Response) -> Vec<u8> {
    to_bytes(response.into_body(), 512 * 1024)
        .await
        .expect("bounded response")
        .to_vec()
}

#[tokio::test]
async fn generated_route_round_trips_parameters_output_and_correlation() {
    let app = sum_rpc_router().into_axum();
    let response = app
        .oneshot(post(
            "/api/rpc/tests/sum",
            encoded_request((20_u32, 22_u32), "rpc_test_sum"),
        ))
        .await
        .expect("route response");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(header::CONTENT_TYPE).unwrap(),
        "application/json"
    );

    let decoded = ClientContractPolicy::default()
        .decode_response::<Sum>(&body(response).await)
        .expect("typed success envelope");
    assert_eq!(decoded.request_id().as_str(), "rpc_test_sum");
    assert_eq!(decoded.into_data(), Sum { value: 42 });
}

#[tokio::test]
async fn application_failures_are_redacted_versioned_and_machine_readable() {
    let response = reject_rpc_router()
        .into_axum()
        .oneshot(post(
            "/api/rpc/tests/reject",
            encoded_request((), "rpc_test_reject"),
        ))
        .await
        .expect("route response");
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let encoded = body(response).await;
    let decoded: ServerFailure = ClientContractPolicy::default()
        .decode_failure(&encoded)
        .expect("typed failure envelope");
    assert_eq!(decoded.request_id().as_str(), "rpc_test_reject");
    assert_eq!(decoded.error().code().as_str(), "counter.denied");
    assert!(!decoded.error().retryable());
    assert!(
        !String::from_utf8(encoded)
            .expect("JSON UTF-8")
            .contains("message")
    );
}

#[tokio::test]
async fn route_rejects_wrong_media_type_malformed_and_oversized_bodies() {
    let app = sum_rpc_router().into_axum();
    let wrong_media = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/rpc/tests/sum")
                .header(header::CONTENT_TYPE, "text/plain")
                .body(Body::from("{}"))
                .expect("test request"),
        )
        .await
        .expect("route response");
    assert_eq!(wrong_media.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);

    let malformed = app
        .clone()
        .oneshot(post("/api/rpc/tests/sum", br#"{"payload":[]}"#.to_vec()))
        .await
        .expect("route response");
    assert_eq!(malformed.status(), StatusCode::BAD_REQUEST);

    let oversized = app
        .oneshot(post("/api/rpc/tests/sum", vec![b' '; 256 * 1024 + 1]))
        .await
        .expect("route response");
    assert_eq!(oversized.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn production_baseline_requires_matching_csrf_cookie_and_header() {
    let app: axum::Router = sum_rpc_router().into_axum();
    let app = rullst::apply_security_baseline(
        app,
        rullst::SecurityConfig::default(),
        rullst::config::Environment::Production,
    )
    .expect("valid production baseline");

    let get = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/rpc/tests/sum")
                .body(Body::empty())
                .expect("test request"),
        )
        .await
        .expect("GET response");
    let cookie = get
        .headers()
        .get(header::SET_COOKIE)
        .expect("CSRF cookie")
        .to_str()
        .expect("ASCII cookie")
        .split(';')
        .next()
        .expect("cookie pair")
        .to_owned();
    let csrf = cookie
        .strip_prefix("rullst_csrf=")
        .expect("CSRF cookie name")
        .to_owned();
    let request_body = encoded_request((20_u32, 22_u32), "rpc_test_csrf");

    let missing_header = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/rpc/tests/sum")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie.clone())
                .body(Body::from(request_body.clone()))
                .expect("test request"),
        )
        .await
        .expect("CSRF rejection");
    assert_eq!(missing_header.status(), StatusCode::FORBIDDEN);

    let accepted = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/rpc/tests/sum")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie)
                .header("x-csrf-token", csrf)
                .body(Body::from(request_body))
                .expect("test request"),
        )
        .await
        .expect("CSRF-protected response");
    assert_eq!(accepted.status(), StatusCode::OK);
}

#[test]
fn generated_router_is_a_regular_rullst_router() {
    let _: Router = sum_rpc_router();
}
