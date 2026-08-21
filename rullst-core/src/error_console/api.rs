//! API endpoints for error explanation, self-healing autofix, and developer diagnostics.

use axum::{
    Json,
    extract::{ConnectInfo, Query},
    response::IntoResponse,
};
use serde::Deserialize;
use std::net::SocketAddr;
use std::path::Path;

#[derive(Deserialize)]
/// Query parameters for requests to fetch an AI-based explanation of an error.
pub struct ExplainQuery {
    file: String,
    #[allow(dead_code)]
    line: u32,
    #[allow(dead_code)]
    err: String,
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
    if !addr.ip().is_loopback() {
        return "Access denied: endpoint only accessible from localhost.".to_string();
    }

    // H-3: Apply the same path traversal guard as handle_autofix
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
/// POST request body payload containing data needed to perform an AI autofix operation.
pub struct AutoFixPayload {
    file_path: String,
    line: u32,
    error_message: String,
}

/// POST endpoint that prompts the LLM to rewrite the file on disk to fix the panic.
///
/// **Security:** This endpoint validates that the target file resides within the
/// project's working directory to prevent path-traversal attacks.
#[cfg_attr(mutants, mutants::skip)]
pub async fn handle_autofix(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<AutoFixPayload>,
) -> impl IntoResponse {
    if !addr.ip().is_loopback() {
        return Json(serde_json::json!({
            "success": false,
            "error": "Access denied: endpoint only accessible from localhost"
        }));
    }

    // 1. Resolve the project root (current working directory)
    let project_root = match std::env::current_dir() {
        Ok(cwd) => match cwd.canonicalize() {
            Ok(canonical) => canonical,
            Err(_) => cwd,
        },
        Err(_) => {
            return Json(serde_json::json!({
                "success": false,
                "error": "Unable to determine project root directory"
            }));
        }
    };

    // 2. Resolve and verify the file is within the project root (prevents path traversal and existence oracles)
    let target_path = Path::new(&payload.file_path);
    if target_path
        .components()
        .any(|c| c == std::path::Component::ParentDir)
    {
        return Json(serde_json::json!({
            "success": false,
            "error": "Access denied: Path traversal detected"
        }));
    }

    let canonical_res = target_path.canonicalize();
    let canonical_target = match canonical_res {
        Ok(p) if p.starts_with(&project_root) => p,
        _ => {
            return Json(serde_json::json!({
                "success": false,
                "error": "File not found or access denied"
            }));
        }
    };

    // 4. Additionally restrict to Rust source files only
    let extension = canonical_target
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    if extension != "rs" && extension != "toml" {
        return Json(serde_json::json!({
            "success": false,
            "error": "Autofix is restricted to .rs and .toml files only"
        }));
    }

    // Block sensitive files disclosure (e.g. .env*, Foundry.toml, Cargo.toml)
    if let Some(filename) = canonical_target.file_name().and_then(|f| f.to_str()) {
        if filename.starts_with(".env") || filename == "Foundry.toml" || filename == "Cargo.toml" {
            return Json(serde_json::json!({
                "success": false,
                "error": "Access denied: sensitive configuration files cannot be modified"
            }));
        }
    }

    match perform_autofix(&payload.file_path, payload.line, &payload.error_message).await {
        Ok(_) => Json(serde_json::json!({ "success": true })),
        Err(e) => Json(serde_json::json!({ "success": false, "error": e.to_string() })),
    }
}

/// POST endpoint that triggers database migration execution from the Ignition Error Console.
#[cfg_attr(mutants, mutants::skip)]
pub async fn handle_run_migrations(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    if !addr.ip().is_loopback() {
        return Json(serde_json::json!({
            "success": false,
            "error": "Access denied: endpoint only accessible from localhost"
        }));
    }
    Json(serde_json::json!({
        "success": true,
        "message": "Database migration requested. Run `cargo rullst db:migrate` or use Rullst Studio to apply pending SQL migrations."
    }))
}

#[cfg_attr(mutants, mutants::skip)]
async fn perform_autofix(
    _file_path: &str,
    _line: u32,
    _error_message: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    Err("AI Engine offline. Auto-fix is now available via the `rullst-ai` crate.".into())
}
