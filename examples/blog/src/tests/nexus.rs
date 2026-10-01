//! Nexus administers the tenant-owned `Post` model only inside the request's
//! membership-checked tenant.

use super::{database, test_router};
use crate::app::{Post, create_post};
use axum::body::{Body, to_bytes};
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode, header};
use rullst_orm::with_tenant;
use std::net::SocketAddr;
use tower::ServiceExt;

/// Base64 of the test router's fixture operator and credential.
const BASIC_AUTHORIZATION: &str =
    "Basic aW50ZWdyYXRpb24tZml4dHVyZS1vcGVyYXRvcjppbnRlZ3JhdGlvbi1maXh0dXJlLWNyZWRlbnRpYWw=";

/// An authenticated Nexus request over verified TLS for `tenant`.
fn admin_request(method: &str, path: &str, tenant: &str, form: Option<String>) -> Request<Body> {
    let token = rullst::security::generate_csrf_token();
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header("X-Tenant-ID", tenant)
        .header(header::AUTHORIZATION, BASIC_AUTHORIZATION);
    let body = match form {
        Some(form) => {
            builder = builder
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header(header::COOKIE, format!("rullst_csrf={token}"));
            Body::from(format!("{form}&_token={token}"))
        }
        None => Body::empty(),
    };
    let mut request = builder.body(body).expect("Nexus request");
    request.extensions_mut().insert(ConnectInfo(
        "192.0.2.80:443"
            .parse::<SocketAddr>()
            .expect("test peer address"),
    ));
    request
        .extensions_mut()
        .insert(rullst_nexus::NexusVerifiedTls::from_trusted_tls_termination());
    request
}

async fn send(app: &axum::Router, request: Request<Body>) -> (StatusCode, String) {
    let response = app.clone().oneshot(request).await.expect("Nexus response");
    let status = response.status();
    let body = to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("bounded Nexus body");
    (status, String::from_utf8_lossy(&body).into_owned())
}

async fn stored_title(tenant: &str, id: i32) -> String {
    with_tenant(tenant.to_string(), async { Post::find(id).await })
        .await
        .expect("post lookup")
        .expect("stored post")
        .title
}

#[tokio::test]
async fn nexus_lists_and_changes_only_the_request_tenants_posts() {
    if !database().await {
        return;
    }
    let app = test_router().into_axum();
    let marker = format!("Nexus scope {}", std::process::id());
    let own = create_post("tenant-startup", &format!("{marker} startup"), "Own story")
        .await
        .expect("startup fixture post");
    let other = create_post(
        "tenant-enterprise",
        &format!("{marker} enterprise"),
        "Another tenant's story",
    )
    .await
    .expect("enterprise fixture post");

    let (status, list) = send(
        &app,
        admin_request("GET", "/nexus/table/posts", "tenant-startup", None),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        list.contains(&own.title),
        "Nexus must list the tenant's post"
    );
    assert!(!list.contains(&other.title), "Nexus leaked another tenant");
    assert!(!list.contains("tenant-enterprise"));

    // A search matching both titles still returns only the request tenant's.
    let search = format!("/nexus/table/posts?q={}", marker.replace(' ', "+"));
    let (status, found) = send(&app, admin_request("GET", &search, "tenant-startup", None)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(found.contains(&own.title));
    assert!(
        !found.contains(&other.title),
        "search leaked another tenant"
    );

    let other_record = format!("/nexus/table/posts/record/{}", other.id);
    let (status, _) = send(
        &app,
        admin_request(
            "GET",
            &format!("{other_record}/edit"),
            "tenant-startup",
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = send(
        &app,
        admin_request(
            "POST",
            &other_record,
            "tenant-startup",
            Some("title=Rewritten+across+tenants".to_string()),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(
        stored_title("tenant-enterprise", other.id).await,
        other.title
    );

    // The tenant column is Nexus-controlled: a submitted value is refused.
    let (status, _) = send(
        &app,
        admin_request(
            "POST",
            &format!("/nexus/table/posts/record/{}", own.id),
            "tenant-startup",
            Some("title=Moved&tenant_id=tenant-enterprise".to_string()),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(stored_title("tenant-startup", own.id).await, own.title);
}
