use axum::response::Html;
use axum::{Router, routing::get};
use rullst_orm::dashboard_html;

#[tokio::main]
async fn main() {
    let _app: Router = Router::new().route("/admin", get(|| async { Html(dashboard_html()) }));

    let _listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("Rullst ORM Admin Panel is running!");
    println!("Open http://localhost:3000/admin in your browser.");

    // Un-comment to run the actual server
    // axum::serve(listener, app).await.unwrap();
}
