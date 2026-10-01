//! The Blog starter's controller template.

/// Reads posts through bounded, ordered queries: the index pages through the
/// newest posts and a post is looked up by its slug, so neither depends on
/// the ORM's default row cap. Database failures return `503`, not `404`.
/// `robots.txt` and `sitemap.xml` use absolute URLs from `RULLST_PUBLIC_ORIGIN`.
pub(super) const BLOG_CONTROLLER: &str = r##"use rullst::server::{Extension, IntoResponse, Path, Query, Response, StatusCode};
use rullst::response::Html;
use crate::models::post::Post;
use crate::pages::blog;
use serde::Deserialize;

/// Posts listed on each index page, newest first.
const POSTS_PER_PAGE: usize = 20;

/// The production security headers allow only nonce-bound inline styles; the
/// nonce is absent (and unneeded) when no CSP is sent, as in development.
fn nonce(csp_nonce: &Option<Extension<rullst::security::CspNonce>>) -> &str {
    csp_nonce
        .as_ref()
        .map(|Extension(nonce)| nonce.as_str())
        .unwrap_or_default()
}

fn unavailable(error: rullst_orm::Error) -> Response {
    eprintln!("Blog query failed: {error}");
    StatusCode::SERVICE_UNAVAILABLE.into_response()
}

#[derive(Debug, Deserialize)]
pub struct IndexQuery {
    pub page: Option<usize>,
}

pub async fn index(
    Query(query): Query<IndexQuery>,
    csp_nonce: Option<Extension<rullst::security::CspNonce>>,
) -> Response {
    let page = query.page.unwrap_or(1).max(1);
    match Post::query().order_by_desc("id").paginate(page, POSTS_PER_PAGE).await {
        Ok(posts) => Html(blog::index_page(posts, nonce(&csp_nonce))).into_response(),
        Err(error) => unavailable(error),
    }
}

pub async fn show(
    Path(slug): Path<String>,
    csp_nonce: Option<Extension<rullst::security::CspNonce>>,
) -> Response {
    match Post::query().where_eq("slug", slug).first().await {
        Ok(Some(post)) => Html(blog::detail_page(post, nonce(&csp_nonce))).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "Blog post not found").into_response(),
        Err(error) => unavailable(error),
    }
}

/// The canonical HTTPS origin from `RULLST_PUBLIC_ORIGIN` (the process
/// environment, then `.env`), without a trailing slash. Crawlers reject
/// relative sitemap URLs, so nothing is advertised until it is configured.
fn public_origin() -> Option<String> {
    normalize_public_origin(&rullst::config::project_setting("RULLST_PUBLIC_ORIGIN").ok()??)
}

fn normalize_public_origin(candidate: &str) -> Option<String> {
    let uri = candidate.trim().parse::<rullst::server::Uri>().ok()?;
    if uri.scheme_str() != Some("https") {
        return None;
    }
    let authority = uri.authority()?.as_str();
    if authority.contains('@')
        || uri.path_and_query().map_or("/", |value| value.as_str()) != "/"
    {
        return None;
    }
    Some(rullst::html::escape_str(&format!("https://{authority}")).into_owned())
}

/// Percent-encodes a slug as one URL path segment, which leaves nothing for
/// XML to escape.
fn path_segment(slug: &str) -> String {
    slug.bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                char::from(byte).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

/// The newest posts' slugs, without loading their content.
async fn sitemap_slugs() -> Result<Vec<String>, rullst_orm::Error> {
    Ok(rullst::db::sqlx::query_scalar::<_, String>(
        "SELECT slug FROM posts ORDER BY id DESC LIMIT 1000",
    )
    .fetch_all(rullst::db::Orm::read_pool()?)
    .await?)
}

pub async fn robots_txt() -> impl IntoResponse {
    let sitemap = public_origin()
        .map_or_else(String::new, |origin| format!("Sitemap: {origin}/sitemap.xml\n"));
    (
        StatusCode::OK,
        format!("User-agent: *\nDisallow: /nexus\n{sitemap}"),
    )
}

/// Lists the home page and the 1,000 newest posts as absolute URLs.
pub async fn sitemap_xml() -> Response {
    let mut urls = String::new();
    if let Some(origin) = public_origin() {
        let slugs = match sitemap_slugs().await {
            Ok(slugs) => slugs,
            Err(error) => return unavailable(error),
        };
        urls.push_str(&format!("<url><loc>{origin}/</loc></url>"));
        for slug in slugs {
            urls.push_str(&format!("<url><loc>{origin}/posts/{}</loc></url>", path_segment(&slug)));
        }
    }
    (
        StatusCode::OK,
        [(rullst::http::header::CONTENT_TYPE, "application/xml")],
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?><urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">{urls}</urlset>"#
        ),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sitemap_urls_are_absolute_encoded_and_xml_safe() {
        assert_eq!(path_segment("hello-world_1.0~"), "hello-world_1.0~");
        assert_eq!(path_segment("a b&c/<d>\"é"), "a%20b%26c%2F%3Cd%3E%22%C3%A9");
        assert_eq!(
            normalize_public_origin("https://blog.example:8443/").as_deref(),
            Some("https://blog.example:8443")
        );
        for rejected in ["", "/", "http://blog.example", "https://user@blog.example", "https://blog.example/path"] {
            assert_eq!(normalize_public_origin(rejected), None, "{rejected}");
        }
    }
}
"##;
