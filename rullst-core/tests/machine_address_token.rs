//! `MachineEndpoint::address_token`: a per-resource token in the request
//! address authenticates a provider callback that the provider cannot sign.
use axum::{
    Router,
    body::Body,
    extract::Path,
    http::{Method, Request, StatusCode},
    routing::post,
};
use rullst_core::{
    config::{Environment, SecurityConfig},
    security::{
        MachineAuthentication, MachineEndpoint, MachineEndpointError, MachineEndpointPolicy,
        MachineRequestVerifier, apply_security_baseline_with_machine_endpoints, csrf_middleware,
    },
};
use subtle::ConstantTimeEq;
use tower::ServiceExt;

const KEY: &[u8] = b"application-key-for-callback-addresses-0123456789";

fn token(reference: &str) -> String {
    let key = ring::hmac::Key::new(ring::hmac::HMAC_SHA256, KEY);
    ring::hmac::sign(&key, reference.as_bytes())
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Accepts `/payments/{reference}/{token}` when the token is that
/// reference's HMAC; it never reads the unsigned body.
struct AddressToken;

#[async_trait::async_trait]
impl MachineRequestVerifier for AddressToken {
    async fn verify(
        &self,
        request: axum::extract::Request,
    ) -> Result<axum::extract::Request, MachineEndpointError> {
        let mut segments = request.uri().path().split('/').skip(2);
        let (Some(reference), Some(presented), None) =
            (segments.next(), segments.next(), segments.next())
        else {
            return Err(MachineEndpointError::Unauthorized);
        };
        if bool::from(token(reference).as_bytes().ct_eq(presented.as_bytes())) {
            Ok(request)
        } else {
            Err(MachineEndpointError::Unauthorized)
        }
    }
}

fn app() -> Router {
    let endpoint =
        MachineEndpoint::address_token(Method::POST, "/payments/{reference}/{token}", AddressToken)
            .unwrap();
    assert_eq!(
        endpoint.authentication(),
        MachineAuthentication::AddressTokenWithSourceConfirmation
    );
    // The handler would now confirm the payment with the provider by reference.
    let router = Router::new()
        .route(
            "/payments/{reference}/{token}",
            post(
                |Path((reference, _)): Path<(String, String)>, body: String| async move {
                    format!("{reference}:{body}")
                },
            ),
        )
        .layer(axum::middleware::from_fn(csrf_middleware));
    apply_security_baseline_with_machine_endpoints(
        router,
        SecurityConfig::default(),
        Environment::Production,
        MachineEndpointPolicy::new(vec![endpoint]).unwrap(),
    )
    .unwrap()
}

async fn send(path: &str, cookie: bool) -> (StatusCode, String) {
    let mut request = Request::post(path);
    if cookie {
        request = request.header("Cookie", "rullst_session=ambient");
    }
    let response = app()
        .oneshot(request.body(Body::from("paid")).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    (status, String::from_utf8_lossy(&body).into_owned())
}

#[tokio::test]
async fn the_resource_token_in_the_path_replaces_browser_csrf() {
    let valid = format!("/payments/order-42/{}", token("order-42"));
    assert_eq!(
        send(&valid, false).await,
        (StatusCode::OK, "order-42:paid".to_owned())
    );
    // Another resource's token, a wrong token and ambient cookies are refused.
    let swapped = format!("/payments/order-43/{}", token("order-42"));
    for (path, cookie) in [
        (swapped.as_str(), false),
        ("/payments/order-42/0000", false),
        (valid.as_str(), true),
    ] {
        assert_eq!(
            send(path, cookie).await.0,
            StatusCode::UNAUTHORIZED,
            "{path}"
        );
    }
}

#[test]
fn address_token_routes_follow_the_machine_route_rules() {
    for path in [
        "/payments/*",
        "payments",
        "/{reference}",
        "/a//b",
        "/a/../b",
    ] {
        assert!(matches!(
            MachineEndpoint::address_token(Method::POST, path, AddressToken),
            Err(MachineEndpointError::InvalidRoute)
        ));
    }
    assert!(matches!(
        MachineEndpoint::address_token(Method::GET, "/payments/{reference}", AddressToken),
        Err(MachineEndpointError::InvalidRoute)
    ));
    // It overlaps a signed webhook on the same route like any other category.
    assert!(matches!(
        MachineEndpointPolicy::new(vec![
            MachineEndpoint::address_token(Method::POST, "/payments/{reference}", AddressToken)
                .unwrap(),
            MachineEndpoint::signed_webhook(Method::POST, "/payments/{id}", AddressToken).unwrap(),
        ]),
        Err(MachineEndpointError::InvalidPolicy)
    ));
}
