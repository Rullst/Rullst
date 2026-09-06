/// A browser request to the local development CMS. Keep these HTTP inputs
/// explicit in fixtures rather than bypassing the production access layer.
pub fn local_request() -> axum::http::request::Builder {
    axum::http::Request::builder()
        .header("host", "localhost:3000")
        .header("origin", "http://localhost:3000")
}
