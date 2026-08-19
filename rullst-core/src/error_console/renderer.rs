//! Sleek, glowing dark-theme HTML console renderer for panic diagnosis.

use crate::error_console::parser::{extract_source_context, find_source_location};

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
            --bg: #030712;
            --surface: #0f172a;
            --surface-hover: #1e293b;
            --border: #1e293b;
            --border-glow: rgba(56, 189, 248, 0.2);
            --primary: #38bdf8;
            --primary-glow: rgba(56, 189, 248, 0.4);
            --secondary: #a855f7;
            --accent: #f43f5e;
            --text-main: #f8fafc;
            --text-muted: #94a3b8;
            --code-bg: #090d16;
            --line-active: rgba(244, 63, 94, 0.15);
            --line-active-border: #f43f5e;
            --success: #10b981;
            --danger: #ef4444;
        }}
        * {{
            box-sizing: border-box;
            margin: 0;
            padding: 0;
        }}
        body {{
            background-color: var(--bg);
            color: var(--text-main);
            font-family: 'Outfit', sans-serif;
            min-height: 100vh;
            display: flex;
            flex-direction: column;
            padding: 2rem;
            position: relative;
            overflow-x: hidden;
        }}
        body::before {{
            content: '';
            position: absolute;
            top: -200px;
            left: 50%;
            transform: translateX(-50%);
            width: 800px;
            height: 400px;
            background: radial-gradient(circle, rgba(168, 85, 247, 0.15) 0%, rgba(56, 189, 248, 0.05) 50%, transparent 100%);
            z-index: 0;
            pointer-events: none;
        }}
        .container {{
            max-width: 1200px;
            width: 100%;
            margin: 0 auto;
            position: relative;
            z-index: 1;
        }}
        header {{
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 2rem;
            padding-bottom: 1.5rem;
            border-bottom: 1px solid var(--border);
        }}
        .logo-section {{
            display: flex;
            align-items: center;
            gap: 0.75rem;
        }}
        .logo-icon {{
            font-size: 1.75rem;
            filter: drop-shadow(0 0 10px var(--primary-glow));
        }}
        .logo-text {{
            font-size: 1.25rem;
            font-weight: 700;
            letter-spacing: -0.02em;
            background: linear-gradient(135deg, #fff, var(--text-muted));
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
        }}
        .badge {{
            background: rgba(244, 63, 94, 0.1);
            color: var(--accent);
            border: 1px solid rgba(244, 63, 94, 0.3);
            padding: 0.35rem 0.8rem;
            border-radius: 9999px;
            font-size: 0.8rem;
            font-weight: 600;
            text-transform: uppercase;
            letter-spacing: 0.05em;
        }}
        .error-card {{
            background: var(--surface);
            border: 1px solid var(--border);
            border-radius: 1rem;
            padding: 2rem;
            margin-bottom: 2rem;
            box-shadow: 0 4px 20px rgba(0, 0, 0, 0.5), 0 0 0 1px rgba(255,255,255,0.02);
            position: relative;
            overflow: hidden;
        }}
        .error-card::after {{
            content: '';
            position: absolute;
            left: 0;
            top: 0;
            bottom: 0;
            width: 4px;
            background: linear-gradient(to bottom, var(--accent), var(--secondary));
        }}
        .error-label {{
            font-size: 0.85rem;
            text-transform: uppercase;
            font-weight: 700;
            letter-spacing: 0.05em;
            color: var(--accent);
            margin-bottom: 0.5rem;
        }}
        .error-message {{
            font-size: 1.75rem;
            font-weight: 700;
            line-height: 1.3;
            color: #fff;
            font-family: 'JetBrains Mono', monospace;
            word-break: break-word;
        }}
        .panel-grid {{
            display: grid;
            grid-template-columns: 1fr 1fr;
            gap: 2rem;
        }}
        @media (max-width: 900px) {{
            .panel-grid {{
                grid-template-columns: 1fr;
            }}
        }}
        .section-title {{
            font-size: 1.1rem;
            font-weight: 600;
            color: var(--text-main);
            margin-bottom: 1rem;
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
            display: flex;
            flex-direction: column;
            height: 100%;
        }}
        .code-header {{
            background: rgba(255,255,255,0.02);
            padding: 0.75rem 1.25rem;
            border-bottom: 1px solid var(--border);
            font-family: 'JetBrains Mono', monospace;
            font-size: 0.85rem;
            color: var(--text-muted);
            display: flex;
            justify-content: space-between;
        }}
        .code-header span.file-path {{
            color: var(--primary);
        }}
        .code-body {{
            background: var(--code-bg);
            padding: 1rem 0;
            font-family: 'JetBrains Mono', monospace;
            font-size: 0.85rem;
            line-height: 1.7;
            overflow-x: auto;
            flex-grow: 1;
        }}
        .code-line {{
            display: flex;
            padding: 0 1.25rem;
            transition: background 0.2s;
        }}
        .code-line.active {{
            background: var(--line-active);
            border-left: 3px solid var(--line-active-border);
            padding-left: calc(1.25rem - 3px);
        }}
        .code-line.active .line-num {{
            color: var(--accent);
            font-weight: 700;
        }}
        .line-num {{
            width: 2.5rem;
            min-width: 2.5rem;
            color: #475569;
            user-select: none;
            text-align: right;
            margin-right: 1.25rem;
        }}
        .line-content {{
            white-space: pre;
            color: #e2e8f0;
        }}
        .ai-panel {{
            background: var(--surface);
            border: 1px solid var(--border);
            border-radius: 1rem;
            padding: 1.5rem;
            display: flex;
            flex-direction: column;
            height: 100%;
            box-shadow: 0 4px 20px rgba(0,0,0,0.3);
            position: relative;
        }}
        .ai-panel::before {{
            content: '';
            position: absolute;
            top: 0;
            right: 0;
            width: 150px;
            height: 150px;
            background: radial-gradient(circle, rgba(168, 85, 247, 0.1) 0%, transparent 70%);
            pointer-events: none;
        }}
        .ai-header-badge {{
            display: inline-flex;
            align-items: center;
            gap: 0.4rem;
            color: var(--secondary);
            font-size: 0.85rem;
            font-weight: 700;
            margin-bottom: 0.75rem;
            text-transform: uppercase;
            letter-spacing: 0.05em;
        }}
        .ai-explanation-box {{
            background: var(--code-bg);
            border: 1px solid var(--border);
            border-radius: 0.75rem;
            padding: 1.25rem;
            color: var(--text-muted);
            font-size: 0.95rem;
            margin-bottom: 1.5rem;
            flex-grow: 1;
            overflow-y: auto;
            max-height: 250px;
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
            width: 100%;
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
            background: linear-gradient(90deg, #1e293b 25%, #334155 50%, #1e293b 75%);
            background-size: 200% 100%;
            animation: pulse-shimmer 1.5s infinite;
            border-radius: 0.25rem;
            width: 100%;
        }}
        .pulse-bar:nth-child(2) {{ width: 85%; }}
        .pulse-bar:nth-child(3) {{ width: 60%; }}
        @keyframes pulse-shimmer {{
            0% {{ background-position: 200% 0; }}
            100% {{ background-position: -200% 0; }}
        }}
        .spinner {{
            width: 1.25rem;
            height: 1.25rem;
            border: 2.5px solid rgba(255,255,255,0.3);
            border-top-color: #fff;
            border-radius: 50%;
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
