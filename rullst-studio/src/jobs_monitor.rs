//! Bounded queue inspection routes for a queue explicitly supplied to Studio.

use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
};
use rullst_core::{Queue, QueuedJobDetail, queue::QueueError};
use std::{fmt::Write, sync::Arc};

struct HorizonState {
    queue: Queue,
}

pub fn router(queue: Queue) -> Router {
    let state = Arc::new(HorizonState { queue });

    Router::new()
        .route("/", get(dashboard_home))
        .route("/jobs-table", get(jobs_table))
        // rullst-access: admin — composed behind LocalStudioAccess::protect_router.
        .route("/retry/{id}", post(retry_job))
        .route("/purge-failed", post(purge_failed_jobs))
        .route("/purge", post(purge_failed_jobs))
        .with_state(state)
}

async fn dashboard_home(State(state): State<Arc<HorizonState>>) -> Response {
    match load_snapshot(&state.queue).await {
        Ok((jobs, pending)) => {
            let failed = jobs.iter().filter(|job| job.status == "failed").count();
            let processing = jobs.iter().filter(|job| job.status == "processing").count();
            Html(render_dashboard_layout(
                pending,
                failed,
                processing,
                render_table_rows(&jobs),
            ))
            .into_response()
        }
        Err(error) => queue_error_response(error),
    }
}

async fn jobs_table(State(state): State<Arc<HorizonState>>) -> Response {
    match state.queue.list_all_jobs(50).await {
        Ok(jobs) => Html(render_table_rows(&jobs)).into_response(),
        Err(error) => queue_error_response(error),
    }
}

async fn retry_job(State(state): State<Arc<HorizonState>>, Path(id): Path<String>) -> Response {
    match state.queue.retry_failed_job(&id).await {
        Ok(()) => Redirect::to("/jobs").into_response(),
        Err(error) => queue_error_response(error),
    }
}

async fn purge_failed_jobs(State(state): State<Arc<HorizonState>>) -> Response {
    match state.queue.purge_failed_jobs().await {
        Ok(()) => Redirect::to("/jobs").into_response(),
        Err(error) => queue_error_response(error),
    }
}

async fn load_snapshot(queue: &Queue) -> Result<(Vec<QueuedJobDetail>, u64), QueueError> {
    let jobs = queue.list_all_jobs(50).await?;
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
    <dt>Pending jobs</dt><dd>{pending}</dd>
    <dt>Jobs marked processing</dt><dd>{processing}</dd>
    <dt>Jobs marked failed</dt><dd>{failed}</dd>
  </dl>
  <form method="post" action="/jobs/purge-failed"><button type="submit">Purge failed jobs</button></form>
  <p><a href="/jobs">Refresh snapshot</a> · <a href="/studio">Back to Studio</a></p>
  <table>
    <caption>Up to 50 recent queue records</caption>
    <thead><tr><th>ID / type</th><th>Payload preview</th><th>Status</th><th>Attempts</th><th>Created</th><th>Action</th></tr></thead>
    <tbody>{table_rows}</tbody>
  </table>
</main>
</body>
</html>"#
    )
}

fn bounded_preview(value: &str, maximum_chars: usize) -> String {
    let mut preview = value.chars().take(maximum_chars).collect::<String>();
    if value.chars().count() > maximum_chars {
        preview.push('…');
    }
    preview
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
                "<form method=\"post\" action=\"/jobs/retry/{}\"><button type=\"submit\">Retry job</button></form>",
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
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

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
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/purge-failed")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(purge.status().is_redirection());

        let legacy_purge = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/purge")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(legacy_purge.status().is_redirection());
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
}
