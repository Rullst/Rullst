//! The Blog starter's controller template.

/// Reads posts through bounded, ordered queries: the index pages through the
/// newest posts and a post is looked up by its slug, so neither depends on
/// the ORM's default row cap. Database failures return `503`, not `404`.
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

pub async fn robots_txt() -> impl IntoResponse {
    (
        StatusCode::OK,
        "User-agent: *\nDisallow: /nexus\nSitemap: /sitemap.xml\n",
    )
}

pub async fn sitemap_xml() -> impl IntoResponse {
    (
        StatusCode::OK,
        [(rullst::http::header::CONTENT_TYPE, "application/xml")],
        r#"<?xml version="1.0" encoding="UTF-8"?><urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"><url><loc>/</loc></url></urlset>"#,
    )
}
"##;
