use axum::{routing::get, Json, Router};
use serde::Serialize;

#[derive(Serialize)]
struct Message {
    message: &'static str,
}

async fn plaintext() -> &'static str {
    "Hello, World!"
}

async fn json_endpoint() -> Json<Message> {
    Json(Message { message: "Hello, World!" })
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(plaintext))
        .route("/json", get(json_endpoint));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
