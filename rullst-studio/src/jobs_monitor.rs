//! Bounded queue inspection routes for a queue explicitly supplied to Studio.

use crate::access::{VerifiedLocalStudioAccess, verified_local_access_required};
use axum::{
    Router,
    extract::{Extension, Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
};
use rullst_core::{
    Queue,
    queue::{QueueError, QueuedJobPreview},
};
use std::{fmt::Write, sync::Arc};

struct HorizonState {
    queue: Queue,
}

/// Number of most recent queue records in one snapshot. Status counts derived
/// from it describe this window, not the whole queue.
const SNAPSHOT_RECORDS: u32 = 50;
/// Characters of a payload and of an error rendered in a row.
const PAYLOAD_PREVIEW_CHARS: usize = 256;
const ERROR_PREVIEW_CHARS: usize = 512;
/// Bytes of each payload and error the queue returns: the longest preview in
/// four-byte characters. The SQLite and Redis drivers cut the values in the
/// store, so a large payload never reaches Studio in full.
const PREVIEW_BYTES: u32 = 2_048;
const _: () = assert!(PREVIEW_BYTES as usize >= ERROR_PREVIEW_CHARS * 4);

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
    match list_previews(&state.queue).await {
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

async fn list_previews(queue: &Queue) -> Result<Vec<QueuedJobPreview>, QueueError> {
    queue
        .list_job_previews(SNAPSHOT_RECORDS, PREVIEW_BYTES)
        .await
}

async fn load_snapshot(queue: &Queue) -> Result<(Vec<QueuedJobPreview>, u64), QueueError> {
    let jobs = list_previews(queue).await?;
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
    field_preview(value, maximum_chars, false)
}

/// [`bounded_preview`] that also marks a value the queue already cut.
fn field_preview(value: &str, maximum_chars: usize, truncated: bool) -> String {
    match value.char_indices().nth(maximum_chars) {
        Some((cut, _)) => format!("{}…", &value[..cut]),
        None if truncated => format!("{value}…"),
        None => value.to_string(),
    }
}

fn render_table_rows(jobs: &[QueuedJobPreview]) -> String {
    if jobs.is_empty() {
        return "<tr><td colspan=\"6\">No queue records in this snapshot.</td></tr>".to_string();
    }

    jobs.iter().fold(String::new(), |mut rows, job| {
        let id_preview = bounded_preview(&job.id, 8);
        let payload = field_preview(&job.payload, PAYLOAD_PREVIEW_CHARS, job.payload_truncated);
        let error = job
            .error
            .as_deref()
            .map(|error| field_preview(error, ERROR_PREVIEW_CHARS, job.error_truncated));
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
#[cfg(not(miri))]
mod tests;
