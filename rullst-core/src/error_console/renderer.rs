//! Sleek, glowing dark-theme HTML console renderer for panic diagnosis.

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

/// Stylesheet inlined in a `<style>` element that carries the CSP nonce.
const CONSOLE_STYLE: &str = include_str!("console.css");

/// Renders the development panic console page.
///
/// `nonce` is the request's CSP nonce: the inline `<style>` and `<script>`
/// carry it so the default nonce-based policy allows them. The page loads no
/// external resource (fonts fall back to system faces).
#[cfg_attr(mutants, mutants::skip)]
pub(crate) async fn render_console_html(
    error_message: &str,
    capture: &PanicCapture,
    nonce: Option<&str>,
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

    <script{nonce_attr}>
        const file_path = "{file_display}";
        const line_num = parseInt("{line_display}");
        const err_msg = `{escaped_err}`;

        // 1. Fetch explanation asynchronously to avoid blocking render
        async function loadSolution() {{
            const solutionBox = document.getElementById('ai-solution-box');
            const autofixBtn = document.getElementById('btn-autofix');

            if (file_path === "Unknown File") {{
                solutionBox.innerHTML = "<div class='empty-state'>Cannot generate solution without file location.<br><br><small class='tip'>💡 <b>Tip:</b> If the Rullst AI Assistant is not activated yet, set your <code>GEMINI_API_KEY</code>, <code>OPENAI_API_KEY</code>, or <code>ANTHROPIC_API_KEY</code> environment variable to enable self-healing.</small></div>";
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
        trace_html = trace_html,
        nonce_attr = nonce_attr,
        style = CONSOLE_STYLE
    )
}
