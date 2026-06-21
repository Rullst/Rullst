use askama::Template;
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Json},
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use std::sync::Arc;

#[derive(Serialize, Deserialize)]
struct Message {
    message: String,
}

#[derive(Serialize, Deserialize)]
struct User {
    id: i32,
    name: String,
}

#[derive(askama::Template)]
#[template(source = "<h1>Hello, {{ name }}!</h1>", ext = "html")]
struct HelloTemplate<'a> {
    name: &'a str,
}

pub struct AppState {
    db: PgPool,
}

async fn text_handler() -> &'static str {
    "Hello World"
}

async fn json_handler() -> Json<Message> {
    Json(Message {
        message: "Hello World".to_string(),
    })
}

async fn html_handler() -> impl IntoResponse {
    let template = HelloTemplate { name: "World" };
    match template.render() {
        Ok(html) => Html(html).into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Template error").into_response(),
    }
}

async fn db_single_handler(State(state): State<Arc<AppState>>) -> Result<Json<User>, StatusCode> {
    let row = sqlx::query("SELECT id, name FROM users LIMIT 1")
        .fetch_one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(User {
        id: row.get("id"),
        name: row.get("name"),
    }))
}

pub fn app_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/text", get(text_handler))
        .route("/json", get(json_handler))
        .route("/html", get(html_handler))
        .route("/db-single", get(db_single_handler))
        .with_state(state)
}

#[tokio::main]
async fn main() {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/benchdb".to_string());

    let pool = PgPoolOptions::new()
        .max_connections(100)
        .connect(&database_url)
        .await
        .expect("Failed to create pool");

    // Auto-migrate for benchmark purposes
    sqlx::query("CREATE TABLE IF NOT EXISTS users (id SERIAL PRIMARY KEY, name VARCHAR(255) NOT NULL)")
        .execute(&pool)
        .await
        .unwrap();

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&pool)
        .await
        .unwrap_or(0);

    if count == 0 {
        sqlx::query("INSERT INTO users (name) VALUES ($1)")
            .bind("Benchmark User")
            .execute(&pool)
            .await
            .unwrap();
    }

    let state = Arc::new(AppState { db: pool });
    let app = app_router(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();
    println!("Axum listening on 0.0.0.0:8000");
    axum::serve(listener, app).await.unwrap();
}
