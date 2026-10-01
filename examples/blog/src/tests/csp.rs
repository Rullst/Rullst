//! The showcase pages must render under the production Content Security
//! Policy that `Server` applies in staging and production.

use super::{database, test_router};
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use tower::ServiceExt;

const PAGES: [&str; 10] = [
    "/",
    "/posts/repository",
    "/live-feed",
    "/pico-demo",
    "/templates-demo",
    "/pricing",
    "/checkout?provider=wise",
    "/security-demo?test=dlp",
    "/ai-assistant?q=rust",
    "/omni",
];

/// One start tag: its lowercase name and attribute names/values.
struct Tag {
    name: String,
    attributes: Vec<(String, String)>,
}

impl Tag {
    fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(attribute, _)| attribute == name)
            .map(|(_, value)| value.as_str())
    }
}

/// Parses the start tags of the rendered HTML, honoring quoted values.
fn start_tags(html: &str) -> Vec<Tag> {
    let mut tags = Vec::new();
    let mut rest = html;
    while let Some(open) = rest.find('<') {
        rest = &rest[open + 1..];
        if !rest.starts_with(|c: char| c.is_ascii_alphabetic()) {
            continue;
        }
        let name_end = rest
            .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
            .unwrap_or(rest.len());
        let name = rest[..name_end].to_ascii_lowercase();
        rest = &rest[name_end..];
        let mut attributes = Vec::new();
        loop {
            rest = rest.trim_start_matches(|c: char| c.is_whitespace() || c == '/');
            if rest.is_empty() || rest.starts_with('>') {
                break;
            }
            let end = rest
                .find(|c: char| c.is_whitespace() || c == '=' || c == '>')
                .unwrap_or(rest.len());
            let attribute = rest[..end].to_ascii_lowercase();
            rest = &rest[end..];
            let mut value = String::new();
            if let Some(after) = rest.strip_prefix('=') {
                let quote = after.chars().next().unwrap_or('"');
                let body = &after[quote.len_utf8()..];
                let close = body.find(quote).expect("closed attribute value");
                value = body[..close].to_string();
                rest = &body[close + quote.len_utf8()..];
            }
            attributes.push((attribute, value));
        }
        tags.push(Tag { name, attributes });
    }
    tags
}

async fn get(app: &axum::Router, path: &str) -> axum::response::Response {
    app.clone()
        .oneshot(Request::get(path).body(Body::empty()).expect("GET request"))
        .await
        .expect("GET response")
}

#[tokio::test]
async fn every_page_renders_under_the_production_content_security_policy() {
    database().await;
    let app = test_router().into_axum();
    let mut same_origin_assets = Vec::new();

    for path in PAGES {
        let response = get(&app, path).await;
        assert_eq!(response.status(), StatusCode::OK, "{path}");
        let headers = response.headers();
        let csp = headers
            .get(header::CONTENT_SECURITY_POLICY)
            .and_then(|value| value.to_str().ok())
            .expect("CSP header")
            .to_owned();
        let nonce = csp
            .split("'nonce-")
            .nth(1)
            .and_then(|rest| rest.split('\'').next())
            .expect("CSP nonce");
        assert_eq!(
            csp,
            rullst::security::DEFAULT_CSP_TEMPLATE.replace("{NONCE}", nonce),
            "{path} must be served with the unrelaxed production policy"
        );
        assert_eq!(headers.get(header::X_FRAME_OPTIONS).unwrap(), "DENY");

        let body = to_bytes(response.into_body(), 1024 * 1024)
            .await
            .expect("bounded HTML body");
        let html = std::str::from_utf8(&body).expect("UTF-8 HTML");
        for tag in start_tags(html) {
            for (attribute, value) in &tag.attributes {
                assert_ne!(attribute, "style", "{path}: inline style on <{}>", tag.name);
                assert!(
                    !attribute.starts_with("on"),
                    "{path}: inline {attribute} handler on <{}>",
                    tag.name
                );
                if matches!(attribute.as_str(), "src" | "href") && value.starts_with("/assets/") {
                    same_origin_assets.push(value.clone());
                }
            }
            let nonced = tag.attribute("nonce") == Some(nonce);
            match tag.name.as_str() {
                "script" => {
                    let src = tag.attribute("src").unwrap_or_default();
                    assert!(
                        src.starts_with('/') || nonced,
                        "{path}: script `{src}` is neither same-origin nor nonced"
                    );
                }
                "style" => assert!(nonced, "{path}: <style> without the response nonce"),
                "link" => {
                    let href = tag.attribute("href").unwrap_or_default();
                    assert!(href.starts_with('/'), "{path}: cross-origin <link> {href}");
                }
                "img" => {
                    let src = tag.attribute("src").unwrap_or_default();
                    assert!(src.starts_with('/'), "{path}: cross-origin image {src}");
                }
                "iframe" => panic!("{path}: frame-ancestors 'none' refuses embedded pages"),
                _ => {}
            }
        }
    }

    same_origin_assets.sort();
    same_origin_assets.dedup();
    assert!(same_origin_assets.len() >= 6, "{same_origin_assets:?}");
    for asset in same_origin_assets
        .iter()
        .map(String::as_str)
        .chain(["/favicon.ico"])
    {
        let response = get(&app, asset).await;
        assert_eq!(response.status(), StatusCode::OK, "{asset}");
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();
        let expected = match asset.rsplit('.').next() {
            Some("css") => "text/css",
            Some("js") => "text/javascript",
            _ => "image/png",
        };
        assert!(
            content_type.starts_with(expected),
            "{asset}: {content_type}"
        );
    }
}
