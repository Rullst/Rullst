#[macro_use] extern crate rocket;

use askama::Template;
use rocket::serde::json::Json;
use rocket::State;
use serde::Serialize;
use sqlx::{PgPool, FromRow};
use rocket::http::ContentType;

#[derive(Serialize)]
pub struct Message {
    message: String,
}

#[derive(FromRow, Serialize)]
struct Record {
    id: i32,
    name: String,
}

#[derive(Template)]
#[template(path = "hello.html")]
struct HelloTemplate<'a> {
    name: &'a str,
}

#[get("/text")]
fn text() -> &'static str {
    "Hello World"
}

#[get("/json")]
fn json() -> Json<Message> {
    Json(Message {
        message: "Hello World".to_string(),
    })
}

#[get("/db-single")]
async fn db_single(pool: &State<PgPool>) -> Option<Json<Record>> {
    let rec = sqlx::query_as::<_, Record>("SELECT id, name FROM records LIMIT 1")
        .fetch_optional(pool.inner())
        .await
        .ok()??;
    Some(Json(rec))
}

#[get("/html")]
fn html() -> (ContentType, String) {
    let t = HelloTemplate { name: "World" };
    (ContentType::HTML, t.render().unwrap())
}

#[launch]
async fn rocket() -> _ {
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://benchuser:password@localhost/benchmark".to_string());

    // For benchmarking, maybe database is not running during simple routing benchmarks.
    // Try to connect, if it fails, provide a dummy pool or panic based on usage.
    let pool = PgPool::connect(&db_url).await.unwrap_or_else(|_| {
        eprintln!("Warning: Could not connect to database, db-single will fail.");
        PgPool::connect_lazy(&db_url).unwrap()
    });

    let mut config = rocket::Config::release_default();
    config.port = 8000;
    config.address = "0.0.0.0".parse().unwrap();
    // Disable logging for benchmarks
    config.log_level = rocket::config::LogLevel::Off;

    rocket::custom(config)
        .manage(pool)
        .mount("/", routes![text, json, db_single, html])
}
