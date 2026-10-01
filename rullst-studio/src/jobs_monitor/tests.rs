#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use axum::{
    body::{Body, to_bytes},
    http::Request,
};
use rullst_core::QueuedJobDetail;
use rullst_core::queue::{QueueDriver, SqliteDriver};
use tower::ServiceExt;

fn verified_post(uri: &str) -> Request<Body> {
    let mut request = Request::builder()
        .method("POST")
        .uri(uri)
        .body(Body::empty())
        .unwrap();
    request
        .extensions_mut()
        .insert(crate::access::VerifiedLocalStudioAccess);
    request
}

async fn snapshot_html(app: &Router) -> String {
    let response = app
        .clone()
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 128 * 1024).await.unwrap();
    String::from_utf8(body.to_vec()).unwrap()
}

#[tokio::test]
async fn queue_dashboard_routes_return_real_snapshots() {
    let queue = Queue::sqlite("sqlite::memory:").await.unwrap();
    let app = router(queue);

    for uri in ["/", "/jobs-table"] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    let purge = app
        .clone()
        .oneshot(verified_post("/purge-failed"))
        .await
        .unwrap();
    assert!(purge.status().is_redirection());

    let legacy_purge = app.oneshot(verified_post("/purge")).await.unwrap();
    assert!(legacy_purge.status().is_redirection());
}

#[tokio::test]
// TM-STUDIO-06: importing the raw queue router cannot turn retry or purge
// into an unprotected write API.
async fn raw_queue_router_denies_writes_without_verified_local_access() {
    let driver = SqliteDriver::new("sqlite::memory:")
        .await
        .unwrap()
        .try_with_completed_history_limit(10)
        .unwrap();
    driver
        .push("completed-job", "report", r#"{"scope":"daily"}"#)
        .await
        .unwrap();
    let completed = driver.pop().await.unwrap().unwrap();
    driver.mark_complete(&completed.id).await.unwrap();
    driver
        .push("failed-job", "report", r#"{"scope":"weekly"}"#)
        .await
        .unwrap();
    let failed = driver.pop().await.unwrap().unwrap();
    driver.mark_failed(&failed.id, "boom").await.unwrap();
    let app = router(Queue::custom(Box::new(driver)));

    for uri in [
        "/retry/failed-job",
        "/purge-failed",
        "/purge-completed",
        "/purge",
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(uri)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN, "{uri}");
    }
    let html = snapshot_html(&app).await;
    assert!(html.contains("<code>complete…</code>"));
    assert!(html.contains("<code>failed-j…</code>"));
    assert!(html.contains("Retry job"));

    let retry = app
        .clone()
        .oneshot(verified_post("/retry/failed-job"))
        .await
        .unwrap();
    assert!(retry.status().is_redirection());
    assert!(!snapshot_html(&app).await.contains("Retry job"));
}

#[tokio::test]
async fn dashboard_reads_and_purges_real_opt_in_completed_history() {
    let driver = SqliteDriver::new("sqlite::memory:")
        .await
        .unwrap()
        .try_with_completed_history_limit(10)
        .unwrap();
    driver
        .push("completed-job", "report", r#"{"scope":"daily"}"#)
        .await
        .unwrap();
    let claimed = driver.pop().await.unwrap().unwrap();
    driver.mark_complete(&claimed.id).await.unwrap();
    let app = router(Queue::custom(Box::new(driver)));

    let response = app
        .clone()
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 128 * 1024).await.unwrap();
    let html = String::from_utf8(body.to_vec()).unwrap();
    assert!(html.contains("Completed among the 50 most recent records"));
    assert!(html.contains("<code>complete…</code>"));
    assert!(html.contains("<dd>1</dd>"));
    assert!(html.contains("Purge all completed history"));

    let purge = app
        .clone()
        .oneshot(verified_post("/purge-completed"))
        .await
        .unwrap();
    assert!(purge.status().is_redirection());

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let body = to_bytes(response.into_body(), 128 * 1024).await.unwrap();
    let html = String::from_utf8(body.to_vec()).unwrap();
    assert!(!html.contains("<code>complete…</code>"));
}

#[test]
fn previews_are_cut_on_character_boundaries() {
    assert_eq!(bounded_preview("abc", 3), "abc");
    assert_eq!(bounded_preview("abcd", 3), "abc…");
    assert_eq!(bounded_preview("ééé", 2), "éé…");
    assert_eq!(bounded_preview("", 0), "");
    assert_eq!(bounded_preview("a", 0), "…");
    // A value the queue already cut is marked even when it fits.
    assert_eq!(field_preview("ab", 3, true), "ab…");
    assert_eq!(field_preview("abcd", 3, true), "abc…");
}

#[test]
fn job_rows_escape_untrusted_values_and_accept_short_identifiers() {
    let html = render_table_rows(&[QueuedJobPreview::from_detail(
        QueuedJobDetail {
            id: "é".to_string(),
            name: "<script>".to_string(),
            payload: "{\"value\":\"<img>\"}".to_string(),
            status: "failed".to_string(),
            error: Some("<b>failure</b>".to_string()),
            attempts: 1,
            created_at: "now".to_string(),
            updated_at: "now".to_string(),
        },
        PREVIEW_BYTES,
    )]);

    assert!(html.contains("Retry job"));
    assert!(html.contains("&lt;script&gt;"));
    assert!(html.contains("&lt;img&gt;"));
    assert!(html.contains("&lt;b&gt;failure&lt;/b&gt;"));
    assert!(!html.contains("<script>"));
}

#[test]
fn dashboard_labels_windowed_status_counts_as_such() {
    let html = render_dashboard_layout(2, 1, 3, 4, String::new());

    // Only the pending count covers the whole queue; the status counts
    // come from the snapshot window, while purges remove every match.
    assert!(html.contains("<dt>Pending jobs in the queue</dt><dd>2</dd>"));
    assert!(html.contains("<dt>Marked failed among the 50 most recent records</dt><dd>1</dd>"));
    assert!(html.contains("<dt>Marked processing among the 50 most recent records</dt><dd>3</dd>"));
    assert!(html.contains("<dt>Completed among the 50 most recent records</dt><dd>4</dd>"));
    assert!(html.contains("Purge every failed job"));
    assert!(!html.contains("<dt>Jobs marked failed</dt>"));
}

#[tokio::test]
async fn snapshots_use_the_bounded_queue_projection() {
    let driver = SqliteDriver::new("sqlite::memory:").await.unwrap();
    // Text that is not UTF-8 cannot be listed as complete `String` records, but
    // the bounded projection decodes the leading bytes it reads.
    sqlx::query(
        "INSERT INTO rullst_jobs (id, name, payload) VALUES ('raw-job', 'raw', CAST(X'7BFF7D' AS TEXT))",
    )
    .execute(driver.get_pool())
    .await
    .unwrap();
    assert!(driver.list_all_jobs(SNAPSHOT_RECORDS).await.is_err());
    let app = router(Queue::custom(Box::new(driver)));

    let html = snapshot_html(&app).await;
    assert!(html.contains("<code>{\u{fffd}}</code>"), "{html}");
    let table = app
        .oneshot(
            Request::builder()
                .uri("/jobs-table")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(table.status(), StatusCode::OK);
}

#[tokio::test]
async fn large_payloads_render_bounded_previews() {
    let driver = SqliteDriver::new("sqlite::memory:").await.unwrap();
    let large = format!("{{\"blob\":\"{}\"}}", "é".repeat(1_000_000));
    driver.push("large-job", "report", &large).await.unwrap();
    let claimed = driver.pop().await.unwrap().unwrap();
    driver
        .mark_failed(&claimed.id, &"ü".repeat(500_000))
        .await
        .unwrap();
    let app = router(Queue::custom(Box::new(driver)));

    let html = snapshot_html(&app).await;
    assert!(html.len() < 16 * 1024, "{}", html.len());
    let payload = format!("<code>{{&quot;blob&quot;:&quot;{}…</code>", "é".repeat(247));
    assert!(html.contains(&payload));
    assert!(html.contains(&format!("<div>{}…</div>", "ü".repeat(512))));
}
