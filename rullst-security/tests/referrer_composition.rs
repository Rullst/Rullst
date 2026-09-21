//! A private endpoint's referrer restriction survives every supported layer.
use axum::{
    Router,
    body::Body,
    http::{HeaderValue, Request, Response, header},
    middleware::from_fn,
    routing::get,
};
use rullst_core::security::headers_middleware;
use rullst_security::{CspSecurityLayer, SecureHeadersConfig, SecureHeadersLayer};
use tower::ServiceExt;

#[tokio::test]
async fn no_referrer_survives_composition_without_trusting_weaker_handler_policies() {
    for policies in [
        vec![],
        vec!["unsafe-url"],
        vec!["unrecognized"],
        vec!["no-referrer"],
        vec!["unsafe-url", "no-referrer"],
        vec!["no-referrer", "unsafe-url"],
    ] {
        for layers in 0..6 {
            let values = policies.clone();
            let mut app = Router::new().route(
                "/private",
                get(move || {
                    let values = values.clone();
                    async move {
                        let mut response = Response::new(Body::from("private"));
                        for value in values {
                            response
                                .headers_mut()
                                .append(header::REFERRER_POLICY, HeaderValue::from_static(value));
                        }
                        response
                    }
                }),
            );
            app = match layers {
                0 => app.layer(from_fn(headers_middleware)),
                1 => app.layer(SecureHeadersLayer::default()),
                2 => app.layer(CspSecurityLayer),
                3 => app
                    .layer(CspSecurityLayer)
                    .layer(SecureHeadersLayer::default())
                    .layer(from_fn(headers_middleware)),
                4 => app
                    .layer(from_fn(headers_middleware))
                    .layer(SecureHeadersLayer::default())
                    .layer(CspSecurityLayer),
                _ => {
                    let mut config = SecureHeadersConfig::default();
                    config.referrer_policy = Some("origin".into());
                    app.layer(SecureHeadersLayer::with_config(config))
                }
            };
            let response = app
                .oneshot(Request::get("/private").body(Body::empty()).unwrap())
                .await
                .unwrap();
            let expected = if policies.contains(&"no-referrer") {
                "no-referrer"
            } else if layers == 5 {
                "origin"
            } else {
                "strict-origin-when-cross-origin"
            };
            assert_eq!(
                response.headers()[header::REFERRER_POLICY],
                expected,
                "layers={layers}, input={policies:?}"
            );
            assert_eq!(
                response
                    .headers()
                    .get_all(header::REFERRER_POLICY)
                    .iter()
                    .count(),
                1
            );
            assert_eq!(
                response.headers()[header::X_CONTENT_TYPE_OPTIONS],
                "nosniff"
            );
        }
    }
}
