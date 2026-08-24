#![allow(clippy::unwrap_used, clippy::expect_used)]

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use rullst_studio::Studio;
use tower::ServiceExt;

#[tokio::test]
async fn test_studio_all_views_and_interactive_actions() {
    let app = Studio::new().into_router();

    // 1. Feature flags view & toggles
    let req = Request::builder()
        .uri("/features")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let toggle_req = Request::builder()
        .method("POST")
        .uri("/features/toggle/beta_feature")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(toggle_req).await.unwrap();
    assert!(res.status() == StatusCode::OK || res.status() == StatusCode::SEE_OTHER);

    // 2. Env viewer
    let req = Request::builder().uri("/env").body(Body::empty()).unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 3. ER Diagram
    let req = Request::builder().uri("/er").body(Body::empty()).unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 4. Requests / Logger view
    let req = Request::builder()
        .uri("/requests")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 5. Core Studio visualizers
    for path in [
        "/radar",
        "/capital",
        "/traces",
        "/security",
        "/ai",
        "/migrations",
    ] {
        let req = Request::builder().uri(path).body(Body::empty()).unwrap();
        let res = app.clone().oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK, "Failed route: {}", path);
    }
}
