use axum::{Router, routing::get_service};
use colored::*;
use pulldown_cmark::{Parser, Options, html};
use std::fs;
use std::path::Path;
use tower_http::services::ServeDir;
use walkdir::WalkDir;

pub fn run_build() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}",
        "📚 Building static site with RullstPress...".cyan().bold()
    );
    let docs_dir = Path::new("docs");
    let dist_dir = docs_dir.join("dist");

    if !docs_dir.exists() {
        println!(
            "{}",
            "❌ 'docs/' directory not found. Please create the folder and add Markdown files."
                .red()
        );
        std::process::exit(1);
    }

    if dist_dir.exists() {
        fs::remove_dir_all(&dist_dir)?;
    }
    fs::create_dir_all(&dist_dir)?;

    // Map all pages
    let mut pages = Vec::new();
    for entry in WalkDir::new(docs_dir) {
        let entry = entry?;
        let path = entry.path();
        if path.starts_with(&dist_dir) {
            continue;
        }

        if path.is_file() {
            if path.extension().and_then(|e| e.to_str()) == Some("md") {
                pages.push(path.to_path_buf());
            } else {
                // Copy static assets (images, etc.)
                if let Ok(rel_path) = path.strip_prefix(docs_dir) {
                    let out_path = dist_dir.join(rel_path);
                    if let Some(parent) = out_path.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                    let _ = fs::copy(path, &out_path);
                }
            }
        }
    }

    for page in &pages {
        let is_pt = page.components().any(|c| c.as_os_str() == "pt");
        let filtered_pages: Vec<std::path::PathBuf> = pages
            .iter()
            .filter(|p| {
                let p_is_pt = p.components().any(|c| c.as_os_str() == "pt");
                p_is_pt == is_pt
            })
            .cloned()
            .collect();
        let sidebar_html = generate_sidebar(&filtered_pages, docs_dir);

        let content = fs::read_to_string(page)?;
        let mut options = Options::empty();
        options.insert(Options::ENABLE_TABLES);
        options.insert(Options::ENABLE_STRIKETHROUGH);
        let parser = Parser::new_ext(&content, options);
        let mut html_output = String::new();
        html::push_html(&mut html_output, parser);

        let layout = if page.file_name().and_then(|n| n.to_str()) == Some("index.md") {
            render_home_layout(&html_output, &sidebar_html, page)
        } else {
            render_layout(&html_output, &sidebar_html)
        };

        let rel_path = page.strip_prefix(docs_dir)?;
        let mut out_path = dist_dir.join(rel_path);
        out_path.set_extension("html");

        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&out_path, layout)?;
        println!("{} {}", "✅ Generated:".green(), out_path.display());
    }

    println!(
        "{}",
        "🚀 Build completed successfully in docs/dist/!"
            .green()
            .bold()
    );
    Ok(())
}

fn generate_sidebar(pages: &[std::path::PathBuf], docs_dir: &Path) -> String {
    // Use relative paths — no leading slash — so GitHub Pages sub-path works correctly
    let mut html = String::from(
        "<ul class=\"sidebar-list\" id=\"sidebar-links\">\n<li><a href=\"index.html\">Home</a></li>\n",
    );

    let mut sorted_pages = pages.to_vec();
    sorted_pages.sort();

    for page in sorted_pages {
        if let Ok(rel) = page.strip_prefix(docs_dir) {
            let link = rel
                .with_extension("html")
                .display()
                .to_string()
                .replace("\\", "/");
            let name = rel
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            if name == "index" {
                continue;
            }

            let mut title = name.replace("-", " ");

            // Remove num prefix (e.g., "1 getting started" -> "getting started")
            if let Some(first_char) = title.chars().next() {
                if first_char.is_ascii_digit() {
                    let parts: Vec<&str> = title.splitn(2, ' ').collect();
                    if parts.len() == 2 {
                        title = parts[1].to_string();
                    }
                }
            }

            // Capitalize title
            let mut chars = title.chars();
            if let Some(first) = chars.next() {
                title = first.to_uppercase().collect::<String>() + chars.as_str();
            }

            // Custom capitalization overrides for premium branding
            title = title
                .replace("rullstpress", "RullstPress")
                .replace("Rullstpress", "RullstPress")
                .replace("rullst", "Rullst")
                .replace("blog", "Blog");

            // Relative link — no leading slash
            html.push_str(&format!("<li><a href=\"{}\">{}</a></li>\n", link, title));
        }
    }
    html.push_str("</ul>");
    html
}

/// JavaScript block for copy-to-clipboard on code blocks
fn copy_code_script() -> &'static str {
    r#"<script>
    document.addEventListener('DOMContentLoaded', function () {
        document.querySelectorAll('pre').forEach(function (pre) {
            // Wrap in relative container
            const wrapper = document.createElement('div');
            wrapper.style.position = 'relative';
            pre.parentNode.insertBefore(wrapper, pre);
            wrapper.appendChild(pre);

            // Create copy button
            const btn = document.createElement('button');
            btn.className = 'copy-btn';
            btn.textContent = 'Copy';
            btn.setAttribute('aria-label', 'Copy code to clipboard');
            wrapper.appendChild(btn);

            btn.addEventListener('click', function () {
                const code = pre.querySelector('code') ? pre.querySelector('code').innerText : pre.innerText;
                navigator.clipboard.writeText(code).then(function () {
                    btn.textContent = '✓ Copied!';
                    btn.classList.add('copied');
                    setTimeout(function () {
                        btn.textContent = 'Copy';
                        btn.classList.remove('copied');
                    }, 2000);
                }).catch(function () {
                    // Fallback for older browsers
                    const ta = document.createElement('textarea');
                    ta.value = code;
                    ta.style.position = 'fixed';
                    ta.style.opacity = '0';
                    document.body.appendChild(ta);
                    ta.select();
                    document.execCommand('copy');
                    document.body.removeChild(ta);
                    btn.textContent = '✓ Copied!';
                    btn.classList.add('copied');
                    setTimeout(function () {
                        btn.textContent = 'Copy';
                        btn.classList.remove('copied');
                    }, 2000);
                });
            });
        });
    });
</script>"#
}

fn sidebar_toggle_script() -> &'static str {
    r#"<script>
    document.addEventListener('DOMContentLoaded', function () {
        var sidebar = document.querySelector('.sidebar');
        var button = document.querySelector('.sidebar-toggle');
        if (!sidebar || !button) {
            return;
        }

        var mobileQuery = window.matchMedia('(max-width: 900px)');

        function syncState() {
            var shouldCollapse = mobileQuery.matches;
            sidebar.classList.toggle('is-collapsed', shouldCollapse);
            button.setAttribute('aria-expanded', shouldCollapse ? 'false' : 'true');
        }

        button.addEventListener('click', function () {
            var collapsed = sidebar.classList.toggle('is-collapsed');
            button.setAttribute('aria-expanded', collapsed ? 'false' : 'true');
        });

        if (mobileQuery.addEventListener) {
            mobileQuery.addEventListener('change', syncState);
        } else if (mobileQuery.addListener) {
            mobileQuery.addListener(syncState);
        }

        syncState();
    });
</script>"#
}

/// Shared CSS for the copy button
fn copy_btn_css() -> &'static str {
    r#"
        .copy-btn {
            position: absolute;
            top: 0.6rem;
            right: 0.6rem;
            background: rgba(249, 115, 22, 0.15);
            color: #f97316;
            border: 1px solid rgba(249, 115, 22, 0.35);
            border-radius: 0.375rem;
            padding: 0.25rem 0.65rem;
            font-size: 0.78rem;
            font-family: 'Inter', system-ui, sans-serif;
            font-weight: 600;
            cursor: pointer;
            transition: all 0.2s;
            letter-spacing: 0.02em;
            backdrop-filter: blur(4px);
        }
        .copy-btn:hover {
            background: rgba(249, 115, 22, 0.3);
            border-color: #f97316;
            color: #fff;
        }
        .copy-btn.copied {
            background: rgba(16, 185, 129, 0.2);
            color: #10b981;
            border-color: rgba(16, 185, 129, 0.4);
        }
        {}
"#
}

fn responsive_css() -> &'static str {
    r#"
        *, *::before, *::after {
            box-sizing: border-box;
        }
        img, video, iframe, svg {
            max-width: 100%;
            height: auto;
        }
        pre {
            -webkit-overflow-scrolling: touch;
        }
        .main-content {
            overflow-wrap: anywhere;
        }
        .sidebar-header {
            display: flex;
            align-items: center;
            justify-content: space-between;
            gap: 1rem;
        }
        .sidebar-panel {
            margin-top: 1.5rem;
        }
        .sidebar-toggle {
            display: none;
            appearance: none;
            border: 1px solid var(--border);
            background: rgba(249, 115, 22, 0.08);
            color: var(--text);
            border-radius: 9999px;
            padding: 0.5rem 0.9rem;
            font: inherit;
            font-size: 0.9rem;
            font-weight: 700;
            line-height: 1;
            cursor: pointer;
        }
        .sidebar-toggle:hover {
            border-color: var(--primary);
            color: var(--primary);
        }
        @media (max-width: 900px) {
            body {
                flex-direction: column;
            }
            .sidebar {
                position: static;
                width: 100%;
                height: auto;
                border-right: none;
                border-bottom: 1px solid var(--border);
                padding: 1rem;
            }
            .sidebar-header {
                align-items: flex-start;
            }
            .sidebar-toggle {
                display: inline-flex;
                align-items: center;
                justify-content: center;
            }
            .sidebar-panel {
                margin-top: 1rem;
                overflow: hidden;
                transition: max-height 0.25s ease, opacity 0.2s ease;
                max-height: 1000px;
                opacity: 1;
            }
            .sidebar.is-collapsed .sidebar-panel {
                max-height: 0;
                opacity: 0;
            }
            .main-content {
                margin-left: 0;
                max-width: 100%;
                padding: 1.5rem 1rem 2rem;
            }
            .navbar {
                padding: 1rem;
                flex-direction: column;
                align-items: flex-start;
                gap: 1rem;
            }
            .nav-links {
                width: 100%;
                flex-wrap: wrap;
                gap: 0.75rem 1rem;
            }
            .hero {
                padding: 3rem 1rem 1rem;
            }
            .hero h1 {
                font-size: clamp(2.25rem, 8vw, 4.5rem);
            }
            .hero p {
                font-size: 1.05rem;
            }
            .hero-buttons {
                flex-direction: column;
                align-items: center;
            }
            .btn {
                width: 100%;
                max-width: 320px;
            }
            .install-snippet {
                width: 100%;
                justify-content: center;
                flex-wrap: wrap;
                text-align: center;
            }
            .features {
                grid-template-columns: 1fr;
                padding: 1.5rem 1rem 4rem;
            }
            .feature-card {
                padding: 1.5rem;
            }
        }
    "#
}

fn render_layout(content: &str, sidebar: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Rullst Framework Docs</title>
    <meta name="description" content="The most productive Full-Stack web framework in Rust. Built for developer happiness.">
    <meta name="keywords" content="Rust, Web Framework, Full-Stack, Rullst, Tokio, Axum, WebAssembly">
    <meta name="author" content="Rullst Team">
    <link rel="icon" type="image/png" href="Rullst.png">
    <meta property="og:title" content="Rullst Framework Docs">
    <meta property="og:description" content="The most productive Full-Stack web framework in Rust. Built for developer happiness.">
    <meta property="og:image" content="Rullst.png">
    <meta name="twitter:card" content="summary_large_image">
    <meta name="twitter:image" content="Rullst.png">
    <style>
        :root {{
            --bg: #0f172a;
            --sidebar-bg: #1e293b;
            --text: #f8fafc;
            --text-muted: #94a3b8;
            --primary: #f97316;
            --primary-hover: #10b981;
            --border: #334155;
            --code-bg: #0b1120;
        }}
        body {{
            margin: 0;
            font-family: 'Inter', system-ui, sans-serif;
            background-color: var(--bg);
            color: var(--text);
            display: flex;
            min-height: 100vh;
        }}
        .sidebar {{
            width: 280px;
            background-color: var(--sidebar-bg);
            border-right: 1px solid var(--border);
            padding: 2rem 1.5rem;
            position: fixed;
            height: 100vh;
            overflow-y: auto;
            box-sizing: border-box;
        }}
        .main-content {{
            flex: 1;
            margin-left: 280px;
            padding: 3rem 4rem;
            max-width: 900px;
            line-height: 1.7;
        }}
        .sidebar-brand {{
            font-size: 1.5rem;
            font-weight: 800;
            margin-bottom: 2rem;
            display: flex;
            align-items: center;
            gap: 0.3rem;
            background: linear-gradient(to right, var(--primary), var(--primary-hover));
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
            text-decoration: none;
        }}
        .sidebar-brand img {{
            width: 32px;
            height: 32px;
            border-radius: 6px;
        }}
        .sidebar-list {{
            list-style: none;
            padding: 0;
            margin: 0;
        }}
        .sidebar-list li {{
            margin-bottom: 0.5rem;
        }}
        .sidebar-list a {{
            color: var(--text-muted);
            text-decoration: none;
            font-size: 0.95rem;
            display: block;
            padding: 0.5rem 0.75rem;
            border-radius: 0.375rem;
            transition: all 0.2s;
        }}
        .sidebar-list a:hover {{
            background-color: rgba(249, 115, 22, 0.1);
            color: var(--primary);
        }}
        h1, h2, h3, h4 {{
            color: var(--text);
            margin-top: 2rem;
            margin-bottom: 1rem;
        }}
        h1 {{ font-size: 2.5rem; font-weight: 800; border-bottom: 1px solid var(--border); padding-bottom: 0.5rem; }}
        h2 {{ font-size: 1.75rem; }}
        a {{ color: var(--primary); text-decoration: none; }}
        a:hover {{ text-decoration: underline; }}
        code {{
            background-color: var(--code-bg);
            padding: 0.2rem 0.4rem;
            border-radius: 0.25rem;
            font-family: 'Fira Code', monospace;
            font-size: 0.9em;
            color: #7dd3fc;
        }}
        pre {{
            background-color: var(--code-bg);
            padding: 1.25rem;
            border-radius: 0.5rem;
            overflow-x: auto;
            border: 1px solid var(--border);
        }}
        pre code {{
            background-color: transparent;
            padding: 0;
            color: #e2e8f0;
        }}
        blockquote {{
            border-left: 4px solid var(--primary);
            margin: 0;
            padding: 1rem;
            color: var(--text-muted);
            background: rgba(249,115,22, 0.05);
            border-radius: 0 0.5rem 0.5rem 0;
        }}
        {}
        {}
    </style>
</head>
<body>
    <aside class="sidebar">
        <div class="sidebar-header">
            <a class="sidebar-brand" href="index.html">
                <img src="Rullst.png" alt="Rullst Logo" style="width: 24px; height: 24px;" />
                Rullst
            </a>
            <button class="sidebar-toggle" type="button" aria-expanded="true" aria-controls="sidebar-links">Menu</button>
        </div>
        <div class="sidebar-panel">
            {}
        </div>
    </aside>
    <main class="main-content">
        {}
    </main>
    {}
    {}
</body>
</html>"#,
        copy_btn_css(),
    responsive_css(),
        sidebar,
        content,
        copy_code_script(),
        sidebar_toggle_script()
    )
}

fn render_home_layout(content: &str, _sidebar: &str, _page_path: &std::path::Path) -> String {
    let title = "Develop fast.<br>Scale forever.";
    let subtitle =
        "The most productive Full-Stack web framework in Rust. Built for developer happiness.";
    let btn_start = "Learn how to begin";
    // Relative path — no leading slash — works on GitHub Pages sub-path
    let btn_link = "1-getting-started.html";

    let f1_title = "Extremely Fast";
    let f1_desc = "Built on Tokio and Axum. Enjoy the insane speed of Rust without giving up a high-level API.";

    let f2_title = "Batteries Included";
    let f2_desc = "ORM, Authentication, Background Jobs, Mailer, Cache, and WebSocket all built-in and ready to use.";

    let f3_title = "AI-Native";
    let f3_desc = "Designed from the ground up to be manipulated by AI Agents. Predictable, clear, and strongly typed architecture.";

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Rullst — Full-Stack Web Framework for Rust</title>
    <meta name="description" content="The most productive Full-Stack web framework in Rust. Built for developer happiness.">
    <meta name="keywords" content="Rust, Web Framework, Full-Stack, Rullst, Tokio, Axum, WebAssembly">
    <meta name="author" content="Rullst Team">
    <link rel="icon" type="image/png" href="Rullst.png">
    <meta property="og:title" content="Rullst Framework">
    <meta property="og:description" content="The most productive Full-Stack web framework in Rust. Built for developer happiness.">
    <meta property="og:image" content="Rullst.png">
    <meta name="twitter:card" content="summary_large_image">
    <meta name="twitter:image" content="Rullst.png">
    <style>
        :root {{
            --bg: #0f172a;
            --sidebar-bg: #1e293b;
            --text: #f8fafc;
            --text-muted: #94a3b8;
            --primary: #f97316;
            --primary-hover: #10b981;
            --border: #334155;
            --card-bg: #1e293b;
        }}
        body {{
            margin: 0;
            font-family: 'Inter', system-ui, sans-serif;
            background-color: var(--bg);
            color: var(--text);
            display: flex;
            flex-direction: column;
            min-height: 100vh;
        }}
        .navbar {{
            display: flex;
            justify-content: space-between;
            align-items: center;
            padding: 1rem 4rem;
            border-bottom: 1px solid var(--border);
            background: rgba(15, 23, 42, 0.8);
            backdrop-filter: blur(12px);
            position: sticky;
            top: 0;
            z-index: 100;
        }}
        .brand {{
            font-size: 1.5rem;
            font-weight: 800;
            display: flex;
            align-items: center;
            gap: 0.5rem;
            background: linear-gradient(to right, var(--primary), var(--primary-hover));
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
            text-decoration: none;
        }}
        .brand img {{
            width: 40px;
            height: 40px;
            border-radius: 10px;
        }}
        .nav-links {{
            display: flex;
            gap: 2rem;
            align-items: center;
        }}
        .nav-links a {{
            color: var(--text-muted);
            font-weight: 500;
            text-decoration: none;
            transition: color 0.2s;
        }}
        .nav-links a:hover {{
            color: var(--primary);
        }}
        .hero {{
            text-align: center;
            max-width: 800px;
            margin: 0 auto;
            padding: 5rem 2rem 1rem;
        }}
        .hero-logo {{
            width: 140px;
            margin-bottom: 1.5rem;
            border-radius: 28px;
            box-shadow: 0 20px 60px rgba(249,115,22,0.25), 0 0 0 1px rgba(249,115,22,0.1);
            animation: float 4s ease-in-out infinite;
        }}
        @keyframes float {{
            0%, 100% {{ transform: translateY(0); }}
            50% {{ transform: translateY(-10px); }}
        }}
        .hero h1 {{
            font-size: 4.5rem;
            line-height: 1.1;
            margin-bottom: 1.5rem;
            border: none;
            background: linear-gradient(135deg, var(--primary), var(--primary-hover));
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
        }}
        .hero p {{
            font-size: 1.25rem;
            color: var(--text-muted);
            margin-bottom: 2.5rem;
        }}
        .hero-buttons {{
            display: flex;
            gap: 1rem;
            justify-content: center;
            flex-wrap: wrap;
            margin-bottom: 2rem;
        }}
        .btn {{
            padding: 0.85rem 2.2rem;
            border-radius: 9999px;
            font-size: 1.05rem;
            font-weight: 700;
            text-decoration: none;
            transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
            display: inline-block;
        }}
        .btn-primary {{
            background: linear-gradient(135deg, var(--primary), var(--primary-hover));
            color: white;
            box-shadow: 0 10px 25px -5px rgba(249, 115, 22, 0.4);
            animation: pulse-btn 2.5s infinite;
        }}
        .btn-primary:hover {{
            transform: translateY(-3px) scale(1.02);
            box-shadow: 0 20px 35px -5px rgba(249, 115, 22, 0.5);
            animation: none;
            text-decoration: none;
        }}
        .btn-secondary {{
            background: transparent;
            color: var(--text-muted);
            border: 1px solid var(--border);
        }}
        .btn-secondary:hover {{
            background: rgba(255,255,255,0.05);
            color: var(--text);
            border-color: var(--primary);
            text-decoration: none;
        }}
        @keyframes pulse-btn {{
            0% {{ box-shadow: 0 0 0 0 rgba(249, 115, 22, 0.6); }}
            70% {{ box-shadow: 0 0 0 15px rgba(249, 115, 22, 0); }}
            100% {{ box-shadow: 0 0 0 0 rgba(249, 115, 22, 0); }}
        }}
        .install-snippet {{
            display: inline-flex;
            align-items: center;
            gap: 0.75rem;
            background: rgba(11, 17, 32, 0.8);
            border: 1px solid var(--border);
            border-radius: 0.75rem;
            padding: 0.75rem 1.25rem;
            font-family: 'Fira Code', monospace;
            font-size: 0.95rem;
            color: #7dd3fc;
            margin-bottom: 3rem;
            cursor: pointer;
            transition: border-color 0.2s;
        }}
        .install-snippet:hover {{
            border-color: var(--primary);
        }}
        .install-snippet .copy-icon {{
            font-size: 0.85rem;
            color: var(--text-muted);
            transition: color 0.2s;
        }}
        .install-snippet:hover .copy-icon {{
            color: var(--primary);
        }}
        .features {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
            gap: 1.5rem;
            padding: 2rem 4rem 6rem;
            max-width: 1100px;
            margin: 0 auto;
            width: 100%;
            box-sizing: border-box;
        }}
        .feature-card {{
            background: var(--card-bg);
            padding: 2rem;
            border-radius: 1rem;
            border: 1px solid var(--border);
            transition: transform 0.2s, box-shadow 0.2s, border-color 0.2s;
        }}
        .feature-card:hover {{
            transform: translateY(-5px);
            box-shadow: 0 12px 30px rgba(0,0,0,0.3);
            border-color: rgba(249,115,22,0.3);
        }}
        .feature-icon {{
            font-size: 1.75rem;
            margin-bottom: 1rem;
            background: rgba(249, 115, 22, 0.1);
            width: 3rem;
            height: 3rem;
            display: flex;
            align-items: center;
            justify-content: center;
            border-radius: 0.75rem;
        }}
        .feature-card h3 {{
            margin: 0 0 0.75rem 0;
            font-size: 1.15rem;
            color: var(--text);
        }}
        .feature-card p {{
            color: var(--text-muted);
            margin: 0;
            line-height: 1.6;
            font-size: 0.95rem;
        }}
        .content-hidden {{ display: none; }}
        {}
        {}
    </style>
</head>
<body>
    <nav class="navbar">
        <a class="brand" href="index.html">
            <img src="Rullst.png" alt="Rullst Logo" />
            Rullst
        </a>
        <div class="nav-links">
            <a href="1-getting-started.html">Docs</a>
            <a href="spec.html">Spec</a>
            <a href="https://crates.io/crates/rullst" target="_blank">Crates.io</a>
            <a href="https://github.com/venelouis/Rullst" target="_blank">GitHub</a>
        </div>
    </nav>
    <main>
        <div class="hero">
            <img src="Rullst.png" alt="Rullst" class="hero-logo" />
            <h1>{}</h1>
            <p>{}</p>
            <div class="hero-buttons">
                <a href="{}" class="btn btn-primary">{}</a>
                <a href="https://crates.io/crates/rullst" target="_blank" class="btn btn-secondary">View on Crates.io ↗</a>
            </div>

        </div>

        <div class="features">
            <div class="feature-card">
                <div class="feature-icon">⚡</div>
                <h3>{}</h3>
                <p>{}</p>
            </div>
            <div class="feature-card">
                <div class="feature-icon">🔋</div>
                <h3>{}</h3>
                <p>{}</p>
            </div>
            <div class="feature-card">
                <div class="feature-icon">🧠</div>
                <h3>{}</h3>
                <p>{}</p>
            </div>
        </div>

        <!-- Markdown content hidden on homepage -->
        <div class="content-hidden">
            {}
        </div>
    </main>
    <script>

    </script>
    {}
</body>
</html>"#,
        copy_btn_css(),
    responsive_css(),
        title,
        subtitle,
        btn_link,
        btn_start,
        f1_title,
        f1_desc,
        f2_title,
        f2_desc,
        f3_title,
        f3_desc,
        content,
        copy_code_script()
    )
}

pub fn run_dev_server() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "🔄 Compiling initial static site...".yellow());
    run_build()?;

    let port = 4000;
    println!(
        "{}",
        format!("🚀 RullstPress Server running at http://localhost:{}", port)
            .cyan()
            .bold()
    );
    println!("{}", "ℹ️ Press Ctrl+C to stop.".bright_black());

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let app = Router::new().nest_service("/", get_service(ServeDir::new("docs/dist")));
        let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
            .await
            .unwrap();
        axum::serve(listener, app).await.unwrap();
    });

    Ok(())
}
