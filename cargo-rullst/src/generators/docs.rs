use axum::{
    Router,
    body::Body,
    extract::{Path as AxumPath, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use notify::{RecursiveMode, Watcher};
use pulldown_cmark::{Event, Options, Parser, html};
use std::fs;
use std::io::{Error as IoError, ErrorKind};
use std::path::{Component, Path, PathBuf};
use std::sync::mpsc;
use walkdir::WalkDir;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocsBuildReport {
    pub markdown_pages: usize,
    pub copied_assets: usize,
    pub output_dir: PathBuf,
}

fn escape_html(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(character),
        }
    }
    escaped
}

fn page_title(markdown: &str, fallback: &str) -> String {
    markdown
        .lines()
        .find_map(|line| line.trim().strip_prefix("# "))
        .filter(|title| !title.trim().is_empty())
        .unwrap_or(fallback)
        .trim()
        .to_string()
}

fn render_markdown(markdown: &str, fallback_title: &str) -> String {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS;
    let safe_events = Parser::new_ext(markdown, options).map(|event| match event {
        Event::Html(raw) | Event::InlineHtml(raw) => Event::Text(raw),
        other => other,
    });
    let mut body = String::new();
    html::push_html(&mut body, safe_events);
    let title = escape_html(&page_title(markdown, fallback_title));

    format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <meta name="generator" content="RullstPress">
  <title>{title}</title>
  <style>
    :root {{ color-scheme: dark; font-family: ui-sans-serif, system-ui, sans-serif; }}
    body {{ margin: 0; background: #020617; color: #e2e8f0; line-height: 1.7; }}
    main {{ max-width: 860px; margin: 0 auto; padding: 3rem 1.5rem 5rem; }}
    article {{ background: rgba(15, 23, 42, .72); border: 1px solid #1e293b; border-radius: 1rem; padding: clamp(1.25rem, 4vw, 3rem); }}
    a {{ color: #38bdf8; }} code {{ background: #0f172a; padding: .15rem .35rem; border-radius: .3rem; }}
    pre {{ overflow-x: auto; background: #0f172a; padding: 1rem; border-radius: .6rem; }}
    table {{ width: 100%; border-collapse: collapse; }} th, td {{ border: 1px solid #334155; padding: .5rem; text-align: left; }}
  </style>
</head>
<body><main><article>{body}</article></main></body>
</html>
"#
    )
}

fn output_path(
    source_dir: &Path,
    output_dir: &Path,
    source_file: &Path,
) -> Result<PathBuf, IoError> {
    let relative = source_file.strip_prefix(source_dir).map_err(|_| {
        IoError::new(
            ErrorKind::InvalidInput,
            "documentation file escaped the docs source directory",
        )
    })?;
    let mut output = output_dir.join(relative);
    if source_file.extension().and_then(|ext| ext.to_str()) == Some("md") {
        output.set_extension("html");
    }
    Ok(output)
}

/// Builds `docs/**/*.md` and copies documentation assets into `docs/dist/`.
pub fn build_docs_site(project_root: &Path) -> Result<DocsBuildReport, Box<dyn std::error::Error>> {
    let source_dir = project_root.join("docs");
    let output_dir = source_dir.join("dist");
    if !source_dir.is_dir() {
        return Err(IoError::new(
            ErrorKind::NotFound,
            format!(
                "documentation source '{}' does not exist",
                source_dir.display()
            ),
        )
        .into());
    }
    fs::create_dir_all(&output_dir)?;

    let mut markdown_pages = 0usize;
    let mut copied_assets = 0usize;
    let mut has_index = false;
    let mut readme_source = None;

    for entry in WalkDir::new(&source_dir)
        .into_iter()
        .filter_entry(|entry| entry.path() != output_dir)
    {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let source_file = entry.path();
        let destination = output_path(&source_dir, &output_dir, source_file)?;
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }

        if source_file.extension().and_then(|ext| ext.to_str()) == Some("md") {
            let markdown = fs::read_to_string(source_file)?;
            let fallback = source_file
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("Rullst Documentation");
            fs::write(&destination, render_markdown(&markdown, fallback))?;
            markdown_pages = markdown_pages.saturating_add(1);
            has_index |= destination == output_dir.join("index.html");
            if source_file.file_name().and_then(|name| name.to_str()) == Some("README.md") {
                readme_source = Some((markdown, fallback.to_string()));
            }
        } else {
            fs::copy(source_file, destination)?;
            copied_assets = copied_assets.saturating_add(1);
        }
    }

    if !has_index {
        let (markdown, fallback) = readme_source.unwrap_or_else(|| {
            (
                "# Rullst Documentation\n\nAdd `docs/index.md` to customize this page.\n"
                    .to_string(),
                "Rullst Documentation".to_string(),
            )
        });
        fs::write(
            output_dir.join("index.html"),
            render_markdown(&markdown, &fallback),
        )?;
    }

    Ok(DocsBuildReport {
        markdown_pages,
        copied_assets,
        output_dir,
    })
}

fn safe_relative_path(requested: &str) -> Option<PathBuf> {
    let path = Path::new(requested);
    if path
        .components()
        .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
    {
        Some(path.to_path_buf())
    } else {
        None
    }
}

fn absolute_event_path(path: &Path, current_dir: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            current_dir.join(path)
        }
    })
}

fn event_requires_rebuild(paths: &[PathBuf], output_dir: &Path, current_dir: &Path) -> bool {
    paths
        .iter()
        .map(|path| absolute_event_path(path, current_dir))
        .any(|path| !path.starts_with(output_dir))
}

async fn serve_file(root: &Path, requested: &str) -> Response {
    let Some(relative) = safe_relative_path(requested) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let mut path = root.join(relative);
    if path.is_dir() {
        path = path.join("index.html");
    } else if path.extension().is_none() {
        path.set_extension("html");
    }

    let Ok(bytes) = tokio::fs::read(&path).await else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let content_type = match path.extension().and_then(|extension| extension.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("json") => "application/json",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        _ => "application/octet-stream",
    };

    match Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .body(Body::from(bytes))
    {
        Ok(response) => response,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

async fn serve_index(State(root): State<PathBuf>) -> Response {
    serve_file(&root, "index.html").await
}

async fn serve_asset(State(root): State<PathBuf>, AxumPath(path): AxumPath<String>) -> Response {
    serve_file(&root, &path).await
}

/// Builds, watches and serves RullstPress on loopback only.
pub async fn run_docs_dev(
    project_root: &Path,
    port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    let report = build_docs_site(project_root)?;
    let source_dir = project_root.join("docs");
    let output_dir = report.output_dir.canonicalize()?;
    let current_dir = std::env::current_dir()?;
    let rebuild_root = project_root.to_path_buf();
    let (sender, receiver) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        if let Ok(event) = event {
            if event_requires_rebuild(&event.paths, &output_dir, &current_dir) {
                let _ = sender.send(());
            }
        }
    })?;
    watcher.watch(&source_dir, RecursiveMode::Recursive)?;

    std::thread::Builder::new()
        .name("rullstpress-watcher".to_string())
        .spawn(move || {
            let _watcher = watcher;
            while receiver.recv().is_ok() {
                while receiver.try_recv().is_ok() {}
                if let Err(error) = build_docs_site(&rebuild_root) {
                    eprintln!("RullstPress rebuild failed: {error}");
                }
            }
        })?;

    let app = Router::new()
        .route("/", get(serve_index))
        .route("/{*path}", get(serve_asset))
        .with_state(report.output_dir);
    let address = format!("127.0.0.1:{port}");
    let listener = tokio::net::TcpListener::bind(&address).await?;
    println!("RullstPress is serving http://{address}");
    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_markdown_and_assets_without_executing_raw_html() {
        let root = std::env::temp_dir().join(format!("rullstpress-{}", rand::random::<u64>()));
        let docs = root.join("docs");
        fs::create_dir_all(&docs).expect("temporary docs directory");
        fs::write(
            docs.join("index.md"),
            "# Safe title\n\nHello **Rullst**.\n\n<script>alert(1)</script>\n",
        )
        .expect("markdown fixture");
        fs::write(docs.join("logo.txt"), "asset").expect("asset fixture");

        let report = build_docs_site(&root).expect("docs build");
        let html =
            fs::read_to_string(report.output_dir.join("index.html")).expect("generated index");
        assert_eq!(report.markdown_pages, 1);
        assert_eq!(report.copied_assets, 1);
        assert!(html.contains("<strong>Rullst</strong>"));
        assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
        assert!(!html.contains("<script>alert(1)</script>"));
        assert_eq!(
            fs::read_to_string(report.output_dir.join("logo.txt")).expect("copied asset"),
            "asset"
        );

        fs::remove_dir_all(root).expect("temporary docs cleanup");
    }

    #[test]
    fn static_paths_reject_parent_and_absolute_components() {
        assert!(safe_relative_path("guide/start.html").is_some());
        assert!(safe_relative_path("../secret").is_none());
        assert!(safe_relative_path("/etc/passwd").is_none());
    }

    #[test]
    fn watcher_ignores_generated_output_events() {
        let root = std::env::current_dir()
            .expect("current directory")
            .join("docs-watcher-fixture");
        let output = root.join("docs/dist");
        assert!(!event_requires_rebuild(
            &[output.join("index.html")],
            &output,
            &root,
        ));
        assert!(event_requires_rebuild(
            &[root.join("docs/index.md")],
            &output,
            &root,
        ));
    }
}
