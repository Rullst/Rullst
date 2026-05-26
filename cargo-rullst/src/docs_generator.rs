use axum::{Router, routing::get_service};
use colored::*;
use pulldown_cmark::{Parser, html};
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
                // Copy static assets
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
        let parser = Parser::new(&content);
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
    let mut html = String::from("<ul class=\"sidebar-list\">\n<li><a href=\"/\">Home</a></li>\n");

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

            html.push_str(&format!("<li><a href=\"/{}\">{}</a></li>\n", link, title));
        }
    }
    html.push_str("</ul>");
    html
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
    <link rel="icon" type="image/png" href="/Rullst.png">
    <meta property="og:title" content="Rullst Framework Docs">
    <meta property="og:description" content="The most productive Full-Stack web framework in Rust. Built for developer happiness.">
    <meta property="og:image" content="/Rullst.png">
    <meta name="twitter:card" content="summary_large_image">
    <meta name="twitter:image" content="/Rullst.png">
    <style>
        :root {{
            --bg: #0f172a;
            --sidebar-bg: #1e293b;
            --text: #f8fafc;
            --text-muted: #94a3b8;
            --primary: #f97316; /* Logo orange */
            --primary-hover: #10b981; /* Logo emerald green */
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
            background-color: rgba(56, 189, 248, 0.1);
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
            padding-left: 1rem;
            color: var(--text-muted);
            background: rgba(56,189,248, 0.05);
            padding: 1rem;
            border-radius: 0 0.5rem 0.5rem 0;
        }}
    </style>
</head>
<body>
    <aside class="sidebar">
        <div class="sidebar-brand">
            <img src="/Rullst.png" alt="Rullst Logo" style="width: 24px; height: 24px;" />
            Rullst
        </div>
        {}
    </aside>
    <main class="main-content">
        {}
    </main>
</body>
</html>"#,
        sidebar, content
    )
}

fn render_home_layout(content: &str, _sidebar: &str, _page_path: &std::path::Path) -> String {
    let title = "Develop fast.<br>Scale forever.";
    let subtitle =
        "The most productive Full-Stack web framework in Rust. Built for developer happiness.";
    let btn_start = "Learn how to begin";
    let btn_link = "/1-getting-started.html";

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
    <title>Rullst Framework Docs</title>
    <meta name="description" content="The most productive Full-Stack web framework in Rust. Built for developer happiness.">
    <meta name="keywords" content="Rust, Web Framework, Full-Stack, Rullst, Tokio, Axum, WebAssembly">
    <meta name="author" content="Rullst Team">
    <link rel="icon" type="image/png" href="/Rullst.png">
    <meta property="og:title" content="Rullst Framework Docs">
    <meta property="og:description" content="The most productive Full-Stack web framework in Rust. Built for developer happiness.">
    <meta property="og:image" content="/Rullst.png">
    <meta name="twitter:card" content="summary_large_image">
    <meta name="twitter:image" content="/Rullst.png">
    <style>
        :root {{
            --bg: #0f172a;
            --sidebar-bg: #1e293b;
            --text: #f8fafc;
            --text-muted: #94a3b8;
            --primary: #f97316; /* Orange */
            --primary-hover: #10b981; /* Emerald */
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
            gap: 0.3rem;
            background: linear-gradient(to right, var(--primary), var(--primary-hover));
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
        }}
        .nav-links {{
            display: flex;
            gap: 2rem;
            align-items: center;
        }}
        .nav-links a {{
            color: var(--text-muted);
            font-weight: 500;
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
        .hero h1 {{
            font-size: 4.5rem;
            line-height: 1.1;
            margin-bottom: 1.5rem;
            border: none;
            background: linear-gradient(to right, var(--primary), var(--primary-hover));
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
            margin-bottom: 1.5rem;
        }}
        .btn {{
            padding: 1rem 2.5rem;
            border-radius: 9999px;
            font-size: 1.25rem;
            font-weight: 700;
            text-decoration: none;
            transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
        }}
        .btn-primary {{
            background: linear-gradient(135deg, var(--primary), var(--primary-hover));
            color: white;
            box-shadow: 0 10px 25px -5px rgba(249, 115, 22, 0.4), 0 8px 10px -6px rgba(249, 115, 22, 0.1);
            animation: pulse-btn 2.5s infinite;
        }}
        .btn-primary:hover {{
            background: linear-gradient(135deg, #fb923c, #34d399);
            transform: translateY(-3px) scale(1.02);
            box-shadow: 0 20px 25px -5px rgba(249, 115, 22, 0.5), 0 8px 10px -6px rgba(249, 115, 22, 0.2);
            animation: none;
        }}
        
        @keyframes pulse-btn {{
            0% {{ box-shadow: 0 0 0 0 rgba(249, 115, 22, 0.6); }}
            70% {{ box-shadow: 0 0 0 15px rgba(249, 115, 22, 0); }}
            100% {{ box-shadow: 0 0 0 0 rgba(249, 115, 22, 0); }}
        }}

        /* Features */
        .features {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
            gap: 2rem;
            padding: 0 4rem 6rem;
            max-width: 1200px;
            margin: 0 auto;
        }}
        .feature-card {{
            background: var(--card-bg);
            padding: 2rem;
            border-radius: 1rem;
            border: 1px solid var(--border);
            transition: transform 0.2s;
        }}
        .feature-card:hover {{
            transform: translateY(-5px);
        }}
        .feature-icon {{
            font-size: 2rem;
            margin-bottom: 1rem;
            background: rgba(249, 115, 22, 0.1);
            color: var(--primary);
            width: 3rem;
            height: 3rem;
            display: flex;
            align-items: center;
            justify-content: center;
            border-radius: 0.75rem;
        }}
        .feature-card h3 {{
            margin: 0 0 0.75rem 0;
            font-size: 1.25rem;
        }}
        .feature-card p {{
            color: var(--text-muted);
            margin: 0;
            line-height: 1.6;
        }}
        
        /* Markdown overrides for home */
        .content-hidden {{ display: none; }}
    </style>
</head>
<body>
    <nav class="navbar" style="padding: 1rem 4rem;">
        <div class="brand">
            <img src="/Rullst.png" alt="Rullst Logo" style="width: 40px; height: 40px; margin-right: 0.5rem;" />
            Rullst
        </div>
        <div class="nav-links">
            <a href="/1-getting-started.html">Docs</a>
            <a href="https://github.com/venelouis/Rullst" target="_blank">GitHub</a>
        </div>
    </nav>
    <main>
        <div class="hero" style="padding: 4rem 2rem 1rem;">
            <img src="/Rullst.png" alt="Rullst" style="width: 200px; margin-bottom: 0.5rem; border-radius: 32px; box-shadow: 0 20px 40px rgba(249,115,22,0.2);" />
            <h1>{}</h1>
            <p>{}</p>
            <div class="hero-buttons">
                <a href="{}" class="btn btn-primary">{}</a>
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

        <!-- Render hidden markdown content if present, or ignore it on the homepage -->
        <div class="content-hidden">
            {}
        </div>
    </main>
</body>
</html>"#,
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
        content
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
