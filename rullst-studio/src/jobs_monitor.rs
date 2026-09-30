//! Bounded queue inspection routes for a queue explicitly supplied to Studio.

use crate::access::{VerifiedLocalStudioAccess, verified_local_access_required};
use axum::{
    Router,
    extract::{Extension, Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
};
use rullst_core::{Queue, QueuedJobDetail, queue::QueueError};
use std::{fmt::Write, sync::Arc};

struct HorizonState {
    queue: Queue,
}

/// Number of most recent queue records in one snapshot. Status counts derived
/// from it describe this window, not the whole queue.
const SNAPSHOT_RECORDS: u32 = 50;

/// Raw queue routes, without an access boundary.
///
/// [`crate::Studio::with_horizon`] mounts them behind the verified local
/// boundary. Retry and purge additionally require the crate-private marker
/// installed by that boundary, so mounting this router elsewhere exposes only
/// the read-only snapshot and those writes return `403`.
pub fn router(queue: Queue) -> Router {
    let state = Arc::new(HorizonState { queue });

    Router::new()
        .route("/", get(dashboard_home))
        .route("/jobs-table", get(jobs_table))
        // rullst-access: admin — composed behind LocalStudioAccess::protect_router.
        .route("/retry/{id}", post(retry_job))
        .route("/purge-failed", post(purge_failed_jobs))
        .route("/purge-completed", post(purge_completed_history))
        .route("/purge", post(purge_failed_jobs))
        .with_state(state)
}

async fn dashboard_home(State(state): State<Arc<HorizonState>>) -> Response {
    match load_snapshot(&state.queue).await {
        Ok((jobs, pending)) => {
            let failed = jobs.iter().filter(|job| job.status == "failed").count();
            let processing = jobs.iter().filter(|job| job.status == "processing").count();
            let completed = jobs.iter().filter(|job| job.status == "completed").count();
            Html(render_dashboard_layout(
                pending,
                failed,
                processing,
                completed,
                render_table_rows(&jobs),
            ))
            .into_response()
        }
        Err(error) => queue_error_response(error),
    }
}

async fn jobs_table(State(state): State<Arc<HorizonState>>) -> Response {
    match state.queue.list_all_jobs(SNAPSHOT_RECORDS).await {
        Ok(jobs) => Html(render_table_rows(&jobs)).into_response(),
        Err(error) => queue_error_response(error),
    }
}

async fn retry_job(
    State(state): State<Arc<HorizonState>>,
    verified: Option<Extension<VerifiedLocalStudioAccess>>,
    Path(id): Path<String>,
) -> Response {
    if verified.is_none() {
        return verified_local_access_required();
    }
    match state.queue.retry_failed_job(&id).await {
        Ok(()) => Redirect::to("/studio/jobs").into_response(),
        Err(error) => queue_error_response(error),
    }
}

async fn purge_failed_jobs(
    State(state): State<Arc<HorizonState>>,
    verified: Option<Extension<VerifiedLocalStudioAccess>>,
) -> Response {
    if verified.is_none() {
        return verified_local_access_required();
    }
    match state.queue.purge_failed_jobs().await {
        Ok(()) => Redirect::to("/studio/jobs").into_response(),
        Err(error) => queue_error_response(error),
    }
}

async fn purge_completed_history(
    State(state): State<Arc<HorizonState>>,
    verified: Option<Extension<VerifiedLocalStudioAccess>>,
) -> Response {
    if verified.is_none() {
        return verified_local_access_required();
    }
    match state.queue.purge_completed_history().await {
        Ok(()) => Redirect::to("/studio/jobs").into_response(),
        Err(error) => queue_error_response(error),
    }
}

async fn load_snapshot(queue: &Queue) -> Result<(Vec<QueuedJobDetail>, u64), QueueError> {
    let jobs = queue.list_all_jobs(SNAPSHOT_RECORDS).await?;
    let pending = queue.pending_count().await?;
    Ok((jobs, pending))
}

fn queue_error_response(error: QueueError) -> Response {
    let status = if matches!(error, QueueError::StateTransition { .. }) {
        StatusCode::CONFLICT
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    };
    let error = error.to_string();
    let message = rullst_core::html::escape_str(&error);
    (
        status,
        Html(format!(
            "<h1>Queue snapshot unavailable</h1><p>{message}</p>"
        )),
    )
        .into_response()
}

fn render_dashboard_layout(
    pending: u64,
    failed: usize,
    processing: usize,
    completed: usize,
    table_rows: String,
) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head><meta charset="utf-8"><meta name="viewport" content="width=device-width"><title>Rullst queue snapshot</title></head>
<body>
<main>
  <h1>Rullst queue snapshot</h1>
  <p>Current values from the queue supplied to this local Studio instance. They do not prove that a worker is running.</p>
  <dl>
    <dt>Pending jobs in the queue</dt><dd>{pending}</dd>
  </dl>
  <p>The status counts below cover only the {SNAPSHOT_RECORDS} most recent records, not the whole queue.</p>
  <dl>
    <dt>Marked processing among the {SNAPSHOT_RECORDS} most recent records</dt><dd>{processing}</dd>
    <dt>Marked failed among the {SNAPSHOT_RECORDS} most recent records</dt><dd>{failed}</dd>
    <dt>Completed among the {SNAPSHOT_RECORDS} most recent records</dt><dd>{completed}</dd>
  </dl>
  <form method="post" action="/studio/jobs/purge-failed"><button type="submit">Purge every failed job</button></form>
  <form method="post" action="/studio/jobs/purge-completed"><button type="submit">Purge all completed history</button></form>
  <p><a href="/studio/jobs">Refresh snapshot</a> · <a href="/studio">Back to Studio</a></p>
  <table>
    <caption>Up to {SNAPSHOT_RECORDS} most recent queue records</caption>
    <thead><tr><th>ID / type</th><th>Payload preview</th><th>Status</th><th>Attempts</th><th>Created</th><th>Action</th></tr></thead>
    <tbody>{table_rows}</tbody>
  </table>
</main>
</body>
</html>"#
    )
}

/// Cuts a value to `maximum_chars` characters without reading past them, so a
/// large payload is never scanned in full just to render its preview.
fn bounded_preview(value: &str, maximum_chars: usize) -> String {
    match value.char_indices().nth(maximum_chars) {
        Some((cut, _)) => format!("{}…", &value[..cut]),
        None => value.to_string(),
    }
}

fn render_table_rows(jobs: &[QueuedJobDetail]) -> String {
    if jobs.is_empty() {
        return "<tr><td colspan=\"6\">No queue records in this snapshot.</td></tr>".to_string();
    }

    jobs.iter().fold(String::new(), |mut rows, job| {
        let id_preview = bounded_preview(&job.id, 8);
        let payload = bounded_preview(&job.payload, 256);
        let error = job
            .error
            .as_deref()
            .map(|error| bounded_preview(error, 512));
        let action = if job.status == "failed" {
            format!(
                "<form method=\"post\" action=\"/studio/jobs/retry/{}\"><button type=\"submit\">Retry job</button></form>",
                urlencoding::encode(&job.id)
            )
        } else {
            "No action".to_string()
        };
        let error_markup = error.map_or_else(String::new, |error| {
            format!(
                "<div>{}</div>",
                rullst_core::html::escape_str(&error)
            )
        });

        let _ = write!(
            rows,
            "<tr><td><code>{}</code><div>{}</div></td><td><code>{}</code></td><td>{}{}</td><td>{}</td><td>{}</td><td>{action}</td></tr>",
            rullst_core::html::escape_str(&id_preview),
            rullst_core::html::escape_str(&job.name),
            rullst_core::html::escape_str(&payload),
            rullst_core::html::escape_str(&job.status),
            error_markup,
            job.attempts,
            rullst_core::html::escape_str(&job.created_at),
        );
        rows
    })
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
#[cfg(not(miri))]
mod tests {
    use super::*;
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
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
    }

    #[test]
    fn job_rows_escape_untrusted_values_and_accept_short_identifiers() {
        let html = render_table_rows(&[QueuedJobDetail {
            id: "é".to_string(),
            name: "<script>".to_string(),
            payload: "{\"value\":\"<img>\"}".to_string(),
            status: "failed".to_string(),
            error: Some("<b>failure</b>".to_string()),
            attempts: 1,
            created_at: "now".to_string(),
            updated_at: "now".to_string(),
        }]);

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
        assert!(
            html.contains("<dt>Marked processing among the 50 most recent records</dt><dd>3</dd>")
        );
        assert!(html.contains("<dt>Completed among the 50 most recent records</dt><dd>4</dd>"));
        assert!(html.contains("Purge every failed job"));
        assert!(!html.contains("<dt>Jobs marked failed</dt>"));
    }
}
