use axum::{
    Json,
    body::Body,
    extract::{ConnectInfo, Query},
    http::{Request, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::net::SocketAddr;
use std::path::Path;

// ─── Stack Frame & Backtrace Parsing ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Represents a single frame in the panic stack trace.
pub struct StackFrame {
    /// The source file path containing the frame.
    pub file: String,
    /// The line number in the source file.
    pub line: u32,
    /// The name of the function where the panic occurred.
    pub function: String,
}

/// Parses the stack trace to find the developer's source file and line.
#[cfg_attr(mutants, mutants::skip)]
pub fn find_source_location(bt_str: &str) -> Option<(String, u32)> {
    for line in bt_str.lines() {
        let trimmed = line.trim();
        if trimmed.contains("at ")
            && (trimmed.contains("/src/")
                || trimmed.contains("\\src\\")
                || trimmed.contains("/examples/")
                || trimmed.contains("\\examples\\")
                || trimmed.contains("/tests/")
                || trimmed.contains("\\tests\\"))
        {
            // Find the location after "at "
            if let Some(pos) = trimmed.find("at ") {
                let path_part = &trimmed[pos + 3..];
                if let Some((file, line_str)) = path_part.rsplit_once(':')
                    && let Ok(line_num) = line_str.trim().parse::<u32>()
                {
                    return Some((file.trim().to_string(), line_num));
                }
            }
        }
    }
    None
}

// ─── Source Snippet Extractor ───────────────────────────────────────────────

/// Reads a file and extracts surrounding context lines.
#[cfg_attr(mutants, mutants::skip)]
pub fn extract_source_context(
    file_path: &str,
    target_line: u32,
    range: u32,
) -> Option<Vec<(u32, String, bool)>> {
    let project_root = std::env::current_dir()
        .map(|cwd| cwd.canonicalize().unwrap_or(cwd))
        .ok()?;

    let target_path = Path::new(file_path);
    if target_path
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return None;
    }

    let absolute_path = project_root.join(target_path);

    let canonical = absolute_path.canonicalize().ok()?;
    if !canonical.starts_with(&project_root) {
        return None;
    }

    let content = fs::read_to_string(&canonical).ok()?;
    let lines: Vec<&str> = content.lines().collect();

    let total_lines = lines.len() as u32;
    let start = if target_line > range {
        target_line - range
    } else {
        1
    };
    let end = std::cmp::min(target_line + range, total_lines);

    let mut context = Vec::new();
    for i in start..=end {
        if i >= 1 && i <= total_lines {
            let line_content = lines[(i - 1) as usize].to_string();
            let is_target = i == target_line;
            context.push((i, line_content, is_target));
        }
    }
    Some(context)
}

// ─── Axum Middleware for Dev Mode ──────────────────────────────────────────

/// Middleware that catches panic unwinds in dev mode and presents the Self-Healing Console.
#[cfg_attr(mutants, mutants::skip)]
pub async fn catch_panic_middleware(req: Request<Body>, next: Next) -> Response {
    let handle = tokio::spawn(async move { next.run(req).await });

    match handle.await {
        Ok(response) => response,
        Err(join_err) => {
            if join_err.is_panic() {
                let panic_payload = join_err.into_panic();
                let message = if let Some(s) = panic_payload.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = panic_payload.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "Unhandled application panic".to_string()
                };

                let backtrace = std::backtrace::Backtrace::capture();
                let html_content = render_console_html(&message, &backtrace).await;

                match Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
                    .body(Body::from(html_content))
                {
                    Ok(res) => res,
                    Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
                }
            } else {
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

// ─── Explanations & Auto-Fix API Endpoints ────────────────────────────────────

#[derive(Deserialize)]
/// Query parameters for requests to fetch an AI-based explanation of an error.
pub struct ExplainQuery {
    file: String,
    line: u32,
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

    let file_content = fs::read_to_string(&canonical).unwrap_or_default();

    let client = match crate::ai::AiClient::auto() {
        Ok(c) => c,
        Err(_) => return "AI Engine offline. Please configure GEMINI_API_KEY, OPENAI_API_KEY, or ANTHROPIC_API_KEY to enable solutions.".to_string(),
    };

    let prompt = format!(
        "You are an expert Rust coding assistant specialized in the Rullst full-stack framework.\n\
        A panic occurred in the file '{}' at line {} with the following error:\n\
        \"{}\"\n\n\
        Here is the complete source file context:\n\
        ```rust\n\
        {}\n\
        ```\n\n\
        Please provide a highly polished, professional, and concise explanation detailing:\n\
        1. **Why** this specific error occurred.\n\
        2. **How** the developer can fix it step-by-step.\n\
        Make the output clear, actionable, and formatted in clean markdown.",
        query.file, query.line, query.err, file_content
    );

    match client.prompt(&prompt).await {
        Ok(explanation) => explanation,
        Err(e) => format!("Failed to generate solution: {}", e),
    }
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

#[cfg_attr(mutants, mutants::skip)]
async fn perform_autofix(
    file_path: &str,
    line: u32,
    error_message: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let file_content = fs::read_to_string(file_path)?;

    let client = crate::ai::AiClient::auto()?;

    let prompt = format!(
        "You are the core AI auto-fix healing engine for the Rullst Rust framework.\n\
        An application panic occurred at line {} in '{}' with the error:\n\
        \"{}\"\n\n\
        Here is the current content of the file:\n\
        ```rust\n\
        {}\n\
        ```\n\n\
        Return the EXACT, COMPLETE modified file content that fixes the compilation or runtime issue.\n\
        CRITICAL RULES:\n\
        1. Return ONLY the raw file contents. Do not explain anything.\n\
        2. Do NOT wrap the file inside markdown code fences (do NOT include ```rust or ```).\n\
        3. Do NOT add conversational text, notes, or explanations before or after the code.\n\
        4. Preserve all unrelated imports, struct definitions, and business logic intact.",
        line, file_path, error_message, file_content
    );

    let fixed_content = client.prompt(&prompt).await?;

    // Clean up accidental markdown code fence wrapping if the LLM hallucinated them
    let mut clean = fixed_content.trim().to_string();
    if let Some(start) = clean.find("```rust") {
        if let Some(end) = clean.rfind("```") {
            if end > start + 7 {
                clean = clean[start + 7..end].trim().to_string();
            }
        }
    } else if let Some(start) = clean.find("```") {
        if let Some(end) = clean.rfind("```") {
            if end > start + 3 {
                clean = clean[start + 3..end].trim().to_string();
            }
        }
    }
    let clean = clean.trim().to_string();

    if clean.is_empty() {
        return Err("AI generated an empty file patch.".into());
    }

    fs::write(file_path, clean)?;
    Ok(())
}

// ─── Sleek, Glowing Dark-Theme HTML Console Renderer ──────────────────────────

#[cfg_attr(mutants, mutants::skip)]
pub(crate) async fn render_console_html(
    error_message: &str,
    backtrace: &std::backtrace::Backtrace,
) -> String {
    let bt_str = format!("{:#?}", backtrace);
    let source_loc = find_source_location(&bt_str);

    let (file_display, line_display, code_frame_html) = if let Some((file, line)) =
        source_loc.clone()
    {
        let code_snippet = if let Some(context) = extract_source_context(&file, line, 5) {
            context.into_iter().fold(String::with_capacity(512), |mut html, (idx, content, is_target)| {
                let escaped = crate::html::escape_str(&content);
                if is_target {
                    let _ = std::fmt::Write::write_fmt(&mut html, format_args!(
                        "<div class='code-line active'><span class='line-num'>{}</span><span class='line-content'>{}</span></div>",
                        idx, escaped
                    ));
                } else {
                    let _ = std::fmt::Write::write_fmt(&mut html, format_args!(
                        "<div class='code-line'><span class='line-num'>{}</span><span class='line-content'>{}</span></div>",
                        idx, escaped
                    ));
                }
                html
            })
        } else {
            "<div class='empty-state'>Failed to read source file context.</div>".to_string()
        };

        (file, line.to_string(), code_snippet)
    } else {
        (
            "Unknown File".to_string(),
            "Unknown Line".to_string(),
            "<div class='empty-state'>Could not pinpoint developer's frame in stack trace.</div>"
                .to_string(),
        )
    };

    // Filter and clean stack trace lines for presentation
    let trace_html = bt_str.lines().enumerate().fold(String::with_capacity(1024), |mut trace_html, (i, line)| {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return trace_html;
        }
        let is_dev_frame = trimmed.contains("src/")
            || trimmed.contains("src\\")
            || trimmed.contains("examples/")
            || trimmed.contains("examples\\");
        let class = if is_dev_frame {
            "trace-line dev-frame"
        } else {
            "trace-line"
        };
        let _ = std::fmt::Write::write_fmt(&mut trace_html, format_args!(
            "<div class='{}'><span class='trace-idx'>#{}</span><span class='trace-val'>{}</span></div>",
            class, i, crate::html::escape_str(trimmed)
        ));
        trace_html
    });

    let escaped_err = crate::html::escape_str(error_message);

    let escaped_err_js = escaped_err
        .replace('\\', "\\\\")
        .replace('`', "\\`")
        .replace('$', "\\$");

    let file_display_js = file_display.replace('\\', "\\\\").replace('"', "\\\"");

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Rullst Self-Healing Console 🩹</title>
    <link href="https://fonts.googleapis.com/css2?family=Outfit:wght@300;400;500;600;700;800&family=JetBrains+Mono:wght@400;500&display=swap" rel="stylesheet">
    <style>
        :root {{
            --bg: #070913;
            --surface: #0f1324;
            --surface-hover: #171d36;
            --border: rgba(168, 85, 247, 0.15);
            --primary: #a855f7;
            --primary-glow: rgba(168, 85, 247, 0.4);
            --secondary: #ec4899;
            --text: #e2e8f0;
            --text-muted: #64748b;
            --danger: #ef4444;
            --danger-bg: rgba(239, 68, 68, 0.1);
            --success: #10b981;
            --code-bg: #0b0d18;
        }}
        body {{
            background-color: var(--bg);
            color: var(--text);
            font-family: 'Outfit', sans-serif;
            margin: 0;
            padding: 0;
            display: flex;
            justify-content: center;
            min-height: 100vh;
        }}
        .container {{
            width: 100%;
            max-width: 1100px;
            padding: 3rem 1.5rem;
            box-sizing: border-box;
        }}
        header {{
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 2.5rem;
        }}
        .logo-section {{
            display: flex;
            align-items: center;
            gap: 0.75rem;
        }}
        .logo-icon {{
            font-size: 2.2rem;
            filter: drop-shadow(0 0 10px var(--primary-glow));
        }}
        .logo-text {{
            font-size: 1.8rem;
            font-weight: 800;
            background: linear-gradient(135deg, var(--primary), var(--secondary));
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
        }}
        .badge {{
            background: rgba(255, 255, 255, 0.05);
            border: 1px solid rgba(255, 255, 255, 0.1);
            padding: 0.4rem 0.8rem;
            border-radius: 9999px;
            font-size: 0.85rem;
            font-weight: 500;
            color: var(--text-muted);
        }}
        .error-card {{
            background: var(--surface);
            border: 1px solid var(--border);
            box-shadow: 0 8px 32px 0 rgba(0, 0, 0, 0.5), 0 0 0 1px rgba(168, 85, 247, 0.05);
            border-radius: 1rem;
            padding: 2rem;
            margin-bottom: 2rem;
            position: relative;
            overflow: hidden;
        }}
        .error-card::before {{
            content: '';
            position: absolute;
            top: 0;
            left: 0;
            width: 4px;
            height: 100%%;
            background: linear-gradient(to bottom, var(--danger), var(--secondary));
        }}
        .error-label {{
            font-size: 0.8rem;
            font-weight: 700;
            text-transform: uppercase;
            letter-spacing: 0.1rem;
            color: var(--danger);
            margin-bottom: 0.5rem;
        }}
        .error-message {{
            font-size: 1.6rem;
            font-weight: 700;
            line-height: 1.4;
            margin: 0;
        }}
        .panel-grid {{
            display: grid;
            grid-template-columns: 1.5fr 1fr;
            gap: 2rem;
        }}
        @media (max-width: 900px) {{
            .panel-grid {{
                grid-template-columns: 1fr;
            }}
        }}
        .section-title {{
            font-size: 1.15rem;
            font-weight: 600;
            margin-top: 0;
            margin-bottom: 1rem;
            color: #f8fafc;
            display: flex;
            align-items: center;
            gap: 0.5rem;
        }}
        .code-container {{
            background: var(--surface);
            border: 1px solid var(--border);
            border-radius: 1rem;
            overflow: hidden;
            box-shadow: 0 4px 20px rgba(0,0,0,0.3);
        }}
        .code-header {{
            background: rgba(255,255,255,0.02);
            padding: 0.75rem 1.25rem;
            border-bottom: 1px solid var(--border);
            display: flex;
            justify-content: space-between;
            align-items: center;
        }}
        .file-path {{
            font-family: 'JetBrains Mono', monospace;
            font-size: 0.85rem;
            color: var(--text-muted);
        }}
        .file-path span {{
            color: var(--text);
            font-weight: 500;
        }}
        .code-body {{
            background: var(--code-bg);
            padding: 1.25rem 0;
            font-family: 'JetBrains Mono', monospace;
            font-size: 0.9rem;
            overflow-x: auto;
            line-height: 1.6;
        }}
        .code-line {{
            display: flex;
            padding: 0 1.25rem;
            border-left: 3px solid transparent;
        }}
        .code-line.active {{
            background: rgba(239, 68, 68, 0.08);
            border-left-color: var(--danger);
        }}
        .line-num {{
            width: 2.5rem;
            min-width: 2.5rem;
            color: var(--text-muted);
            text-align: right;
            padding-right: 1.25rem;
            user-select: none;
        }}
        .line-content {{
            white-space: pre;
            color: #cbd5e1;
        }}
        .code-line.active .line-content {{
            color: #fca5a5;
            font-weight: 500;
        }}
        .ai-panel {{
            background: var(--surface);
            border: 1px solid var(--border);
            border-radius: 1rem;
            padding: 1.5rem;
            box-shadow: 0 4px 20px rgba(0,0,0,0.3);
            display: flex;
            flex-direction: column;
            gap: 1.25rem;
            position: relative;
        }}
        .ai-header-badge {{
            display: inline-flex;
            align-items: center;
            gap: 0.5rem;
            background: linear-gradient(135deg, rgba(168, 85, 247, 0.1), rgba(236, 72, 153, 0.1));
            border: 1px solid rgba(168, 85, 247, 0.3);
            padding: 0.4rem 0.8rem;
            border-radius: 9999px;
            font-size: 0.85rem;
            font-weight: 600;
            color: #c084fc;
            align-self: flex-start;
        }}
        .ai-explanation-box {{
            color: #94a3b8;
            font-size: 0.95rem;
            line-height: 1.6;
            min-height: 150px;
        }}
        .ai-explanation-box strong {{
            color: #cbd5e1;
        }}
        .ai-explanation-box code {{
            font-family: 'JetBrains Mono', monospace;
            background: rgba(255,255,255,0.05);
            padding: 0.2rem 0.4rem;
            border-radius: 0.25rem;
            color: #f472b6;
            font-size: 0.85rem;
        }}
        .ai-explanation-box pre {{
            background: var(--code-bg);
            border: 1px solid rgba(255,255,255,0.05);
            border-radius: 0.5rem;
            padding: 1rem;
            overflow-x: auto;
        }}
        .ai-explanation-box pre code {{
            background: none;
            padding: 0;
            color: #cbd5e1;
        }}
        .btn-autofix {{
            background: linear-gradient(135deg, var(--primary), var(--secondary));
            color: #fff;
            border: none;
            border-radius: 0.75rem;
            padding: 0.9rem 1.5rem;
            font-size: 1rem;
            font-weight: 700;
            cursor: pointer;
            display: flex;
            align-items: center;
            justify-content: center;
            gap: 0.5rem;
            transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
            box-shadow: 0 4px 15px var(--primary-glow);
            width: 100%%;
            box-sizing: border-box;
        }}
        .btn-autofix:hover {{
            transform: translateY(-2px);
            box-shadow: 0 6px 20px rgba(168, 85, 247, 0.6);
        }}
        .btn-autofix:active {{
            transform: translateY(0);
        }}
        .btn-autofix:disabled {{
            background: var(--text-muted);
            box-shadow: none;
            cursor: not-allowed;
            opacity: 0.6;
        }}
        .trace-card {{
            background: var(--surface);
            border: 1px solid var(--border);
            border-radius: 1rem;
            margin-top: 2rem;
            overflow: hidden;
            box-shadow: 0 4px 20px rgba(0,0,0,0.3);
        }}
        .trace-header {{
            background: rgba(255,255,255,0.02);
            padding: 0.75rem 1.25rem;
            border-bottom: 1px solid var(--border);
            font-weight: 600;
        }}
        .trace-body {{
            background: var(--code-bg);
            max-height: 350px;
            overflow-y: auto;
            font-family: 'JetBrains Mono', monospace;
            font-size: 0.85rem;
            line-height: 1.5;
            padding: 1rem 0;
        }}
        .trace-line {{
            display: flex;
            padding: 0.35rem 1.25rem;
            color: #64748b;
        }}
        .trace-line.dev-frame {{
            color: #c084fc;
            background: rgba(168, 85, 247, 0.05);
            font-weight: 500;
        }}
        .trace-idx {{
            width: 3rem;
            min-width: 3rem;
            user-select: none;
        }}
        .trace-val {{
            white-space: pre-wrap;
        }}
        .pulse-loader {{
            display: flex;
            flex-direction: column;
            gap: 0.75rem;
            padding: 1rem 0;
        }}
        .pulse-bar {{
            height: 1rem;
            background: linear-gradient(90deg, #1e293b 25%%, #334155 50%%, #1e293b 75%%);
            background-size: 200%% 100%%;
            animation: pulse-shimmer 1.5s infinite;
            border-radius: 0.25rem;
            width: 100%%;
        }}
        .pulse-bar:nth-child(2) {{ width: 85%%; }}
        .pulse-bar:nth-child(3) {{ width: 60%%; }}
        @keyframes pulse-shimmer {{
            0%% {{ background-position: 200%% 0; }}
            100%% {{ background-position: -200%% 0; }}
        }}
        .spinner {{
            width: 1.25rem;
            height: 1.25rem;
            border: 2.5px solid rgba(255,255,255,0.3);
            border-top-color: #fff;
            border-radius: 50%%;
            animation: spin 0.8s linear infinite;
        }}
        @keyframes spin {{
            to {{ transform: rotate(360deg); }}
        }}
    </style>
</head>
<body>
    <div class="container">
        <header>
            <div class="logo-section">
                <span class="logo-icon">🩹</span>
                <span class="logo-text">Rullst Self-Healing Console</span>
            </div>
            <span class="badge">Development Mode</span>
        </header>

        <div class="error-card">
            <div class="error-label">Application Panicked</div>
            <h1 class="error-message">"{escaped_err}"</h1>
        </div>

        <div class="panel-grid">
            <div>
                <div class="section-title">
                    <span>📝</span> Source Code Snippet
                </div>
                <div class="code-container">
                    <div class="code-header">
                        <span class="file-path">File: <span>{file_display}</span> (Line {line_display})</span>
                    </div>
                    <div class="code-body">
                        {code_frame_html}
                    </div>
                </div>
            </div>

            <div>
                <div class="section-title">
                    <span>🤖</span> Rullst AI Assistant
                </div>
                <div class="ai-panel">
                    <div class="ai-header-badge">
                        <span>✨</span> Rullst AI Solution
                    </div>
                    
                    <div id="ai-solution-box" class="ai-explanation-box">
                        <div class="pulse-loader">
                            <div class="pulse-bar"></div>
                            <div class="pulse-bar"></div>
                            <div class="pulse-bar"></div>
                        </div>
                    </div>

                    <button id="btn-autofix" class="btn-autofix" disabled="disabled">
                        <span>🩹</span> Auto-Fix with Rullst AI
                    </button>
                </div>
            </div>
        </div>

        <div class="trace-card">
            <div class="trace-header">Stack Trace</div>
            <div class="trace-body">
                {trace_html}
            </div>
        </div>
    </div>

    <script>
        const file_path = "{file_display}";
        const line_num = parseInt("{line_display}");
        const err_msg = `{escaped_err}`;

        // 1. Fetch explanation asynchronously to avoid blocking render
        async function loadSolution() {{
            const solutionBox = document.getElementById('ai-solution-box');
            const autofixBtn = document.getElementById('btn-autofix');

            if (file_path === "Unknown File") {{
                solutionBox.innerHTML = "<div class='empty-state'>Cannot generate solution without file location.<br><br><small style='color: var(--text-muted); line-height: 1.5;'>💡 <b>Tip:</b> If the Rullst AI Assistant is not activated yet, set your <code>GEMINI_API_KEY</code>, <code>OPENAI_API_KEY</code>, or <code>ANTHROPIC_API_KEY</code> environment variable to enable self-healing.</small></div>";
                return;
            }}

            try {{
                const url = `/_rullst/explain?file=${{encodeURIComponent(file_path)}}&line=${{line_num}}&err=${{encodeURIComponent(err_msg)}}`;
                const response = await fetch(url);
                const text = await response.text();
                
                // Format code and formatting nicely
                solutionBox.innerHTML = formatMarkdown(text);
                
                // Enable Auto-Fix button if we successfully fetched the AI Solution
                if (!text.includes("AI Engine offline")) {{
                    autofixBtn.removeAttribute('disabled');
                }}
            }} catch (err) {{
                solutionBox.innerHTML = "<div class='empty-state'>Failed to fetch AI explanation.</div>";
            }}
        }}

        // 2. Handle Auto-Fix action
        document.getElementById('btn-autofix').addEventListener('click', async function() {{
            const btn = this;
            btn.setAttribute('disabled', 'disabled');
            btn.innerHTML = "<div class='spinner'></div> Healing file...";

            try {{
                const response = await fetch('/_rullst/autofix', {{
                    method: 'POST',
                    headers: {{ 'Content-Type': 'application/json' }},
                    body: JSON.stringify({{
                        file_path: file_path,
                        line: line_num,
                        error_message: err_msg
                    }})
                }});
                const result = await response.json();

                if (result.success) {{
                    btn.innerHTML = "✅ Repaired! Reloading...";
                    btn.style.background = "var(--success)";
                    setTimeout(() => {{
                        window.location.reload();
                    }}, 1200);
                }} else {{
                    btn.removeAttribute('disabled');
                    btn.innerHTML = "❌ Failed to heal. Try again.";
                    btn.style.background = "var(--danger)";
                    alert("Self-healing failed: " + result.error);
                }}
            }} catch (err) {{
                btn.removeAttribute('disabled');
                btn.innerHTML = "🩹 Auto-Fix with Rullst AI";
                alert("Request error: " + err.message);
            }}
        }});

        // Simple markdown parsing function for basic preview
        function formatMarkdown(text) {{
            let formatted = text
                .replace(/&/g, '&amp;')
                .replace(/</g, '&lt;')
                .replace(/>/g, '&gt;')
                // Bold
                .replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>')
                // Code block
                .replace(/```rust([\s\S]*?)```/g, '<pre><code>$1</code></pre>')
                .replace(/```([\s\S]*?)```/g, '<pre><code>$1</code></pre>')
                // Inline code
                .replace(/`(.*?)`/g, '<code>$1</code>')
                // Newlines to breaks
                .replace(/\n/g, '<br>');
            return formatted;
        }}

        // Trigger load
        window.addEventListener('load', loadSolution);
    </script>
</body>
</html>"#,
        escaped_err = escaped_err_js,
        file_display = file_display_js,
        line_display = line_display,
        code_frame_html = code_frame_html,
        trace_html = trace_html
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_find_source_location_linux() {
        let bt = "   0: rullst::error_console::tests::test_panic\n             at /home/user/project/src/error_console.rs:42";
        let res = find_source_location(bt);
        assert_eq!(
            res,
            Some(("/home/user/project/src/error_console.rs".to_string(), 42))
        );
    }

    #[test]
    fn test_find_source_location_windows() {
        let bt = "   0: rullst::error_console::tests::test_panic\n             at C:\\Users\\user\\project\\src\\error_console.rs:55";
        let res = find_source_location(bt);
        assert_eq!(
            res,
            Some((
                "C:\\Users\\user\\project\\src\\error_console.rs".to_string(),
                55
            ))
        );
    }

    #[test]
    fn test_find_source_location_none() {
        let bt = "   0: rust_panic\n             at /home/user/project/main.rs:100";
        let res = find_source_location(bt);
        assert_eq!(res, None);
    }

    #[test]
    fn test_extract_source_context_bounds() {
        use std::io::Write;
        let cwd = std::env::current_dir().unwrap();
        let test_file = cwd.join("test_extract_source_context.rs");
        let mut file = std::fs::File::create(&test_file).unwrap();
        writeln!(file, "line 1").unwrap();
        writeln!(file, "line 2").unwrap();
        writeln!(file, "line 3").unwrap();
        file.sync_all().unwrap();

        let path_str = test_file.to_str().unwrap();

        // Testing line 1 (boundary)
        let ctx = extract_source_context(path_str, 1, 1).unwrap();
        assert_eq!(ctx.len(), 2);
        assert_eq!(ctx[0].1, "line 1");
        assert!(ctx[0].2); // is_target

        // Testing end of file
        let ctx = extract_source_context(path_str, 3, 1).unwrap();
        assert_eq!(ctx.len(), 2);
        assert_eq!(ctx[1].1, "line 3");
        assert!(ctx[1].2);

        let _ = std::fs::remove_file(test_file);
    }
}
