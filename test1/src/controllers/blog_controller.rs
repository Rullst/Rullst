use rullst::server::{Path, IntoResponse};
use rullst::response::Html;
use crate::models::post::Post;
use crate::pages::blog;

pub async fn index() -> impl IntoResponse {
    let posts = Post::all().await.unwrap_or_default();
    Html(blog::index_page(posts))
}

pub async fn show(Path(slug): Path<String>) -> impl IntoResponse {
    let posts = Post::all().await.unwrap_or_default();
    let post = posts.into_iter().find(|p| p.slug == slug).unwrap();
    Html(blog::detail_page(post))
}
