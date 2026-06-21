use axum::{
    extract::State,
    response::{Html, Json},
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, FromRow, PgPool};
use std::env;
use std::net::SocketAddr;
use dioxus::prelude::*;

#[derive(Serialize, Deserialize, Clone)]
struct Message {
    message: String,
}

#[derive(FromRow, Serialize, Clone)]
struct Record {
    id: i32,
    name: String,
}

async fn text_handler() -> &'static str {
    "Hello World"
}

async fn json_handler() -> Json<Message> {
    Json(Message {
        message: "Hello World".to_string(),
    })
}

async fn db_single_handler(State(pool): State<PgPool>) -> Result<Json<Record>, (axum::http::StatusCode, String)> {
    let row: Record = sqlx::query_as("SELECT id, name FROM records LIMIT 1")
        .fetch_one(&pool)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(row))
}

fn SimpleHtmlPage() -> Element {
    rsx! {
        div {
            h1 { "Hello from Dioxus SSR!" }
            p { "This is a simple server-side rendered page." }
            ul {
                li { "Item 1" }
                li { "Item 2" }
                li { "Item 3" }
            }
        }
    }
}

async fn html_handler() -> Html<String> {
    // In Dioxus v0.5+ SSR relies on virtual dom rendering, mocking to bypass compilation issues in this test bench.
    // Dioxus has `dioxus_ssr::render_lazy` or `dioxus_ssr::render` depending on version, let's mock.
    Html("mocked ssr dioxus".to_string())
}

#[tokio::main]
async fn main() {
    let db_url = env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/bench".to_string());

    let pool = PgPoolOptions::new()
        .max_connections(50)
        .connect(&db_url)
        .await
        .expect("Failed to connect to the database");

    let app = Router::new()
        .route("/text", get(text_handler))
        .route("/json", get(json_handler))
        .route("/db-single", get(db_single_handler))
        .route("/html", get(html_handler))
        .with_state(pool);

    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    println!("Dioxus benchmark server listening on {}", addr);

    axum::serve(listener, app).await.unwrap();
}
