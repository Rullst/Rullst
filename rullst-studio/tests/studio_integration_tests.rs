use axum::body::Body;
use axum::http::{Request, StatusCode};
use rullst_studio::Studio;
use rullst_studio::data_browser::IntoStudioPort;
use tower::ServiceExt;

#[tokio::test]
async fn test_studio_core_routes() {
    let app = Studio::new().into_router();

    let routes = [
        "/",
        "/studio",
        "/studio/radar",
        "/studio/capital",
        "/studio/security",
        "/studio/traces",
        "/studio/migrations",
        "/studio/ai",
        "/env",
        "/features",
        "/er",
        "/security/stats",
        "/requests",
        "/tools/migrations",
        "/tools/ai",
        "/tools/security",
        "/tools/radar",
        "/tools/capital",
        "/tools/revenue",
        "/tools/traces",
        "/api/radar",
        "/api/revenue",
        "/api/traces",
    ];

    for route in routes {
        let req = Request::builder()
            .uri(route)
            .body(Body::empty())
            .expect("valid request");

        let res = app.clone().oneshot(req).await.expect("handler executed");
        assert!(
            res.status().is_success() || res.status().is_redirection(),
            "Route {} returned status {:?}",
            route,
            res.status()
        );
    }
}

#[tokio::test]
async fn test_into_studio_port_conversion() {
    assert_eq!(5555u16.into_port(), 5555);
    assert_eq!("8080".into_port(), 8080);
    assert_eq!("".into_port(), 5555);
    assert_eq!("invalid".into_port(), 5555);
    assert_eq!(String::from("9000").into_port(), 9000);
    assert_eq!(Some(3000u16).into_port(), 3000);
    assert_eq!(None::<u16>.into_port(), 5555);
}

#[tokio::test]
async fn test_studio_features_endpoints() {
    let app = Studio::new().into_router();

    let req = Request::builder()
        .uri("/features")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8_lossy(&body);
    assert!(body_str.contains("Feature Flags") || body_str.contains("studio"));
}

#[tokio::test]
async fn test_studio_env_endpoints() {
    let app = Studio::new().into_router();

    let req = Request::builder().uri("/env").body(Body::empty()).unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_studio_er_diagram() {
    let app = Studio::new().into_router();

    let req = Request::builder().uri("/er").body(Body::empty()).unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}
