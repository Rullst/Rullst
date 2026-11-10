//! Application stylesheets linked after Nexus's own (`Nexus::with_stylesheet`).
//!
//! The panel's handlers render the page shell through the public
//! [`super::ui::render_shell`], whose signature (and that of the handlers)
//! carries no configuration, so the validated links reach it through a
//! task-local value that a layer of the Nexus router sets for the duration
//! of each request. Outside a Nexus request no link is added.

use super::NexusBuildError;
use axum::{extract::Request, middleware::Next, response::Response};
use std::sync::Arc;

/// At most this many application stylesheets are linked.
const MAX_STYLESHEETS: usize = 8;
/// Longest accepted stylesheet path, in bytes.
const MAX_STYLESHEET_BYTES: usize = 512;

tokio::task_local! {
    static APPLICATION_STYLESHEETS: Arc<[String]>;
}

/// Checks every stylesheet added with `Nexus::with_stylesheet`: at most
/// [`MAX_STYLESHEETS`], each a same-origin path.
pub(super) fn validate(stylesheets: &[String]) -> Result<(), NexusBuildError> {
    if stylesheets.len() <= MAX_STYLESHEETS && stylesheets.iter().all(|path| is_same_origin(path)) {
        Ok(())
    } else {
        Err(NexusBuildError::InvalidStylesheet)
    }
}

/// A path the browser resolves against the panel's own origin: one leading
/// `/` (a second `/` or a `\` would make it protocol-relative), visible ASCII
/// only and no character that could leave the `href` attribute.
fn is_same_origin(path: &str) -> bool {
    path.len() <= MAX_STYLESHEET_BYTES
        && path.starts_with('/')
        && !path.starts_with("//")
        && path.bytes().all(|byte| {
            byte.is_ascii_graphic() && !matches!(byte, b'"' | b'\'' | b'<' | b'>' | b'`' | b'\\')
        })
}

/// Makes `stylesheets` visible to [`links`] while the inner service handles
/// the request.
pub(super) async fn scope(
    axum::extract::State(stylesheets): axum::extract::State<Arc<[String]>>,
    request: Request,
    next: Next,
) -> Response {
    APPLICATION_STYLESHEETS
        .scope(stylesheets, next.run(request))
        .await
}

/// The `<link rel="stylesheet">` elements of the current Nexus request's
/// application stylesheets, in the order they were added.
pub(super) fn links() -> String {
    APPLICATION_STYLESHEETS
        .try_with(|stylesheets| render(stylesheets))
        .unwrap_or_default()
}

fn render(stylesheets: &[String]) -> String {
    let mut out = String::new();
    for path in stylesheets {
        out.push_str("<link rel=\"stylesheet\" href=\"");
        out.push_str(&rullst_core::html::escape_str(path));
        out.push_str("\" />\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_same_origin_paths_are_accepted() {
        for path in [
            "/static/admin.css",
            "/assets/nexus-overrides.css?v=3&theme=dark",
            "/",
        ] {
            assert!(is_same_origin(path), "{path}");
        }
        for path in [
            "",
            "static/admin.css",
            "//cdn.example.com/admin.css",
            "/\\cdn.example.com/admin.css",
            "https://cdn.example.com/admin.css",
            "/admin.css\" onload=\"alert(1)",
            "/admin.css'",
            "/admin.css>",
            "/<admin>.css",
            "/admin`.css",
            "/admin .css",
            "/admin\t.css",
            "/administração.css",
        ] {
            assert!(!is_same_origin(path), "{path:?}");
        }
        let longest = format!("/{}", "a".repeat(MAX_STYLESHEET_BYTES - 1));
        assert!(is_same_origin(&longest));
        assert!(!is_same_origin(&format!("{longest}a")));
    }

    #[test]
    fn at_most_eight_stylesheets_are_accepted() {
        let paths = |count: usize| {
            (0..count)
                .map(|index| format!("/static/{index}.css"))
                .collect::<Vec<_>>()
        };
        assert_eq!(validate(&[]), Ok(()));
        assert_eq!(validate(&paths(MAX_STYLESHEETS)), Ok(()));
        assert_eq!(
            validate(&paths(MAX_STYLESHEETS + 1)),
            Err(NexusBuildError::InvalidStylesheet)
        );
        let mut one_invalid = paths(2);
        one_invalid.push("//cdn.example.com/x.css".to_string());
        assert_eq!(
            validate(&one_invalid),
            Err(NexusBuildError::InvalidStylesheet)
        );
    }

    #[tokio::test]
    async fn links_exist_only_inside_the_request_scope() {
        assert_eq!(links(), "");
        let stylesheets: Arc<[String]> =
            Arc::from(vec!["/a.css".to_string(), "/b.css?x=1&y=2".to_string()]);
        let rendered = APPLICATION_STYLESHEETS
            .scope(stylesheets, async { links() })
            .await;
        assert_eq!(
            rendered,
            "<link rel=\"stylesheet\" href=\"/a.css\" />\n\
             <link rel=\"stylesheet\" href=\"/b.css?x=1&amp;y=2\" />\n"
        );
        assert_eq!(links(), "");
    }
}
