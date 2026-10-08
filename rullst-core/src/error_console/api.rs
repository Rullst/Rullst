//! API endpoints for error explanation and developer diagnostics, and the
//! retired autofix endpoint that now points to `cargo rullst ai fix`.

use axum::{
    Json,
    extract::{ConnectInfo, Query},
    response::IntoResponse,
};
use serde::Deserialize;
use std::net::SocketAddr;

#[derive(Deserialize)]
/// Query parameters for requests to fetch an AI-based explanation of an error.
pub struct ExplainQuery {
    file: String,
    #[allow(dead_code)]
    line: u32,
    #[allow(dead_code)]
    err: String,
}

/// Loopback peers only. Dual-stack listeners report IPv4 clients as
/// IPv4-mapped IPv6 (`::ffff:127.0.0.1`), so the address is canonicalized
/// first, as the error-console middleware does.
fn is_local_peer(addr: SocketAddr) -> bool {
    addr.ip().to_canonical().is_loopback()
}

/// Asynchronous endpoint called by the browser to fetch the AI error explanation.
///
/// **Security:** Validates that the target file resides within the project's working
/// directory and is a `.rs` or `.toml` file to prevent path-traversal attacks.
#[cfg_attr(mutants, mutants::skip)]
pub async fn handle_explain(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Query(query): Query<ExplainQuery>,
) -> impl IntoResponse {
    if !is_local_peer(addr) {
        return "Access denied: endpoint only accessible from localhost.".to_string();
    }

    // H-3: path traversal guard for the inspected file.
    let project_root = match std::env::current_dir() {
        Ok(cwd) => cwd.canonicalize().unwrap_or(cwd),
        Err(_) => return "Unable to determine project root directory.".to_string(),
    };

    let target_path = std::path::Path::new(&query.file);
    if target_path
        .components()
        .any(|c| c == std::path::Component::ParentDir)
    {
        return "Access denied: Path traversal detected.".to_string();
    }

    let canonical_res = target_path.canonicalize();
    let canonical = match canonical_res {
        Ok(p) if p.starts_with(&project_root) => p,
        _ => return "File not found or access denied.".to_string(),
    };

    let extension = canonical.extension().and_then(|e| e.to_str()).unwrap_or("");
    if extension != "rs" && extension != "toml" {
        return "Access denied: only .rs and .toml files can be inspected.".to_string();
    }

    // Block sensitive files disclosure (e.g. .env*, Foundry.toml, Cargo.toml)
    if let Some(filename) = canonical.file_name().and_then(|f| f.to_str()) {
        if filename.starts_with(".env") || filename == "Foundry.toml" || filename == "Cargo.toml" {
            return "Access denied: sensitive configuration files cannot be inspected.".to_string();
        }
    }

    "AI Engine offline. AI features are now available via the `rullst-ai` crate.".to_string()
}

#[derive(Deserialize)]
/// Body of the retired `POST /_rullst/autofix` request. It is still accepted
/// so that an older console page receives the `410 Gone` pointer below.
pub struct AutoFixPayload {
    #[allow(dead_code)]
    file_path: String,
    #[allow(dead_code)]
    line: u32,
    #[allow(dead_code)]
    error_message: String,
}

/// Command that replaces the retired autofix endpoint.
pub(crate) const FIX_COMMAND: &str = "cargo rullst ai fix <error-id>";

/// Retired `POST /_rullst/autofix` endpoint.
///
/// The console never edits files. A loopback peer receives `410 Gone` with
/// `success: false` and a pointer to `cargo rullst ai fix <error-id>`, which
/// reviews every edit in the terminal (diff, confirmation, git checkpoint);
/// any other peer receives `403`. The payload is ignored.
#[cfg_attr(mutants, mutants::skip)]
pub async fn handle_autofix(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(_payload): Json<AutoFixPayload>,
) -> impl IntoResponse {
    if !is_local_peer(addr) {
        return (
            axum::http::StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "success": false,
                "error": "Access denied: endpoint only accessible from localhost"
            })),
        );
    }
    (
        axum::http::StatusCode::GONE,
        Json(serde_json::json!({
            "success": false,
            "error": format!(
                "The error console no longer edits files. Run `{FIX_COMMAND}` with the id shown on the error page; it previews each edit as a diff and applies it only after you confirm."
            ),
            "command": FIX_COMMAND,
        })),
    )
}

/// POST endpoint for the console's migration button.
///
/// The console has no access to the application's migration registry, so it
/// never runs migrations: it answers `501 Not Implemented` with
/// `success: false` and asks for `cargo rullst db:migrate`. A non-loopback
/// peer receives `403`.
#[cfg_attr(mutants, mutants::skip)]
pub async fn handle_run_migrations(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    if !is_local_peer(addr) {
        return (
            axum::http::StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "success": false,
                "error": "Access denied: endpoint only accessible from localhost"
            })),
        );
    }
    (
        axum::http::StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({
            "success": false,
            "error": "The error console cannot run migrations. Run `cargo rullst db:migrate` to apply pending SQL migrations."
        })),
    )
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::{
        AutoFixPayload, ExplainQuery, handle_autofix, handle_explain, handle_run_migrations,
    };
    use axum::{
        body::to_bytes,
        extract::{ConnectInfo, Query},
        response::IntoResponse,
    };
    use std::net::SocketAddr;

    async fn explain(peer: &str, file: impl Into<String>) -> String {
        let response = handle_explain(
            ConnectInfo(peer.parse::<SocketAddr>().expect("valid test peer")),
            Query(ExplainQuery {
                file: file.into(),
                line: 1,
                err: "test error".to_string(),
            }),
        )
        .await
        .into_response();
        let body = to_bytes(response.into_body(), 4096)
            .await
            .expect("bounded response body");
        String::from_utf8(body.to_vec()).expect("text response")
    }

    #[tokio::test]
    // TM-STUDIO-04: source inspection denies remote, traversal, sensitive and unsupported paths.
    async fn source_inspection_rejects_untrusted_file_requests() {
        let source = std::env::current_dir()
            .expect("workspace directory")
            .join("src/lib.rs");
        assert!(
            explain("192.0.2.30:43000", source.to_string_lossy())
                .await
                .contains("localhost")
        );
        assert!(
            explain("127.0.0.1:43000", "../Cargo.toml")
                .await
                .contains("Path traversal")
        );

        let manifest = std::env::current_dir()
            .expect("workspace directory")
            .join("Cargo.toml");
        assert!(
            explain("127.0.0.1:43000", manifest.to_string_lossy())
                .await
                .contains("sensitive configuration")
        );
        let unsupported_file = std::env::current_dir()
            .expect("workspace directory")
            .join("README.md");
        let unsupported = explain("127.0.0.1:43000", unsupported_file.to_string_lossy()).await;
        assert!(
            unsupported.contains("only .rs and .toml"),
            "unexpected response: {unsupported}"
        );
    }

    #[tokio::test]
    async fn retired_autofix_points_to_the_cli_and_never_edits() {
        let payload = || -> AutoFixPayload {
            serde_json::from_value(serde_json::json!({
                "file_path": "src/lib.rs",
                "line": 1,
                "error_message": "boom"
            }))
            .expect("payload")
        };
        for (peer, status) in [
            ("127.0.0.1:43000", 410),
            ("[::ffff:127.0.0.1]:43000", 410),
            ("192.0.2.30:43000", 403),
        ] {
            let response = handle_autofix(
                ConnectInfo(peer.parse::<SocketAddr>().expect("peer")),
                axum::Json(payload()),
            )
            .await
            .into_response();
            assert_eq!(response.status().as_u16(), status, "{peer}");
            let body = to_bytes(response.into_body(), 4096).await.expect("body");
            let json: serde_json::Value = serde_json::from_slice(&body).expect("JSON body");
            assert_eq!(json["success"], false, "{peer}");
            if status == 410 {
                assert_eq!(json["command"], "cargo rullst ai fix <error-id>");
            }
        }
    }

    #[tokio::test]
    async fn migration_button_never_reports_a_fabricated_success() {
        for (peer, status) in [
            ("127.0.0.1:43000", 501),
            ("[::ffff:127.0.0.1]:43000", 501),
            ("192.0.2.30:43000", 403),
            ("[::ffff:192.0.2.30]:43000", 403),
        ] {
            let response =
                handle_run_migrations(ConnectInfo(peer.parse::<SocketAddr>().expect("peer")))
                    .await
                    .into_response();
            assert_eq!(response.status().as_u16(), status, "{peer}");
            let body = to_bytes(response.into_body(), 4096).await.expect("body");
            let json: serde_json::Value = serde_json::from_slice(&body).expect("JSON body");
            assert_eq!(json["success"], false, "{peer}");
        }
    }
}
