//! `GET /_rullst/errors/{id}`: the panic context that
//! `cargo rullst ai fix <error-id>` reads.
//!
//! Mounted with the development error console only (debug build running in
//! Development). It answers only a loopback peer that names a loopback `Host`
//! without proxy forwarding headers (the development telemetry rule); every
//! other request, a malformed id and an unknown or expired id receive `404`.
//! The id is a random 128-bit value printed only on the loopback error page.

use axum::{
    body::Body,
    extract::{Path, Request},
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};

pub(super) async fn serve(Path(id): Path<String>, request: Request) -> Response {
    if !super::dev_telemetry::is_local(&request) || !crate::error_console::store::valid_id(&id) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let Some(context) = crate::error_console::store::lookup(&id) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Ok(body) = serde_json::to_vec(&context) else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    let mut response = Response::new(Body::from(body));
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response
}

#[cfg(test)]
#[path = "dev_errors_tests.rs"]
mod tests;
