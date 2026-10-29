//! Sleek, glowing dark-theme HTML console renderer for panic diagnosis.

use crate::error_console::assistant::{PANEL_TITLE, PromptSource};
use crate::error_console::capture::PanicCapture;
use crate::error_console::parser::{extract_source_context, find_source_location};

/// The panic's own location when it lies in a readable project file, else
/// the first project frame of the backtrace, else the panic location.
fn panic_source_location(capture: &PanicCapture, backtrace: &str) -> Option<(String, u32)> {
    let readable = |(file, line): &(String, u32)| extract_source_context(file, *line, 0).is_some();
    let location = capture.location.clone();
    location
        .clone()
        .filter(readable)
        .or_else(|| find_source_location(backtrace).filter(readable))
        .or(location)
}

/// The location shown on the page, with `/` separators on every OS: Windows
/// panic locations read `src\main.rs`. Scripts keep the native path.
fn display_path(file: &str) -> String {
    crate::html::escape_str(&file.replace('\\', "/")).into_owned()
}

/// Stylesheet inlined in a `<style>` element that carries the CSP nonce.
const CONSOLE_STYLE: &str = include_str!("console.css");

/// Renders the development panic console page.
///
/// `nonce` is the request's CSP nonce: the inline `<style>` and `<script>`
/// carry it so the default nonce-based policy allows them. The page loads no
/// external resource (fonts fall back to system faces) and makes no request.
/// `request` is the method and path (no query string) quoted in the
/// "Ask your AI assistant" prompt; `error_id` names the recorded context that
/// `GET /_rullst/errors/{id}` serves, when the server mounts that route.
#[cfg_attr(mutants, mutants::skip)]
pub(crate) async fn render_console_html(
    error_message: &str,
    capture: &PanicCapture,
    request: Option<(&str, &str)>,
    nonce: Option<&str>,
    error_id: Option<&str>,
) -> String {
    let nonce_attr = nonce.map_or_else(String::new, |nonce| {
        format!(" nonce=\"{}\"", crate::html::escape_str(nonce))
    });
    let bt_str = capture.backtrace.clone().unwrap_or_default();
    let source_loc = panic_source_location(capture, &bt_str);

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
    let trace_html = if bt_str.is_empty() {
        "<div class='trace-line'>No backtrace was captured. Set RUST_BACKTRACE=1 to record one.</div>".to_string()
    } else {
        bt_str.lines().enumerate().fold(String::with_capacity(1024), |mut trace_html, (i, line)| {
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
    })
    };

    let escaped_err = crate::html::escape_str(error_message);
    let file_html = display_path(&file_display);
    let prompt_file = file_display.replace('\\', "/");
    let assistant_panel = crate::error_console::assistant::panel(&PromptSource {
        message: error_message,
        location: source_loc
            .as_ref()
            .map(|(_, line)| (prompt_file.as_str(), *line)),
        backtrace: &bt_str,
        request,
        error_id,
    });

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Rullst Self-Healing Console 🩹</title>
    <style{nonce_attr}>
{style}
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
                        <span class="file-path">File: <span>{file_html}</span> (Line {line_display})</span>
                    </div>
                    <div class="code-body">
                        {code_frame_html}
                    </div>
                </div>
            </div>

            <div>
                <div class="section-title">
                    <span>🤖</span> {panel_title}
                </div>
                {assistant_panel}
            </div>
        </div>

        <div class="trace-card">
            <div class="trace-header">Stack Trace</div>
            <div class="trace-body">
                {trace_html}
            </div>
        </div>
    </div>

    <script{nonce_attr}>
        // Copies the prompt only; the page makes no network request. The
        // button stays hidden when a Content-Security-Policy blocks this script.
        const copyButton = document.getElementById('btn-copy');
        const promptBox = document.getElementById('ai-prompt');
        if (copyButton && promptBox) {{
            copyButton.hidden = false;
            copyButton.addEventListener('click', function () {{
                const done = function () {{ copyButton.textContent = 'Copied'; }};
                const select = function () {{
                    promptBox.focus();
                    promptBox.select();
                    copyButton.textContent = 'Selected: press Ctrl+C';
                }};
                if (navigator.clipboard && window.isSecureContext) {{
                    navigator.clipboard.writeText(promptBox.value).then(done, select);
                }} else {{
                    select();
                }}
            }});
        }}
    </script>
</body>
</html>"#,
        escaped_err = escaped_err,
        file_html = file_html,
        line_display = line_display,
        code_frame_html = code_frame_html,
        trace_html = trace_html,
        panel_title = PANEL_TITLE,
        assistant_panel = assistant_panel,
        nonce_attr = nonce_attr,
        style = CONSOLE_STYLE
    )
}
