use askama::Template;
use poem::web::{Html, Json};
use poem::{
    get, handler, listener::TcpListener, web::Data, EndpointExt, Route,
    Server,
};
use serde::Serialize;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::env;

#[derive(Serialize)]
struct Message {
    message: &'static str,
}

#[derive(Serialize, sqlx::FromRow)]
struct World {
    id: i32,
    randomnumber: i32,
}

#[derive(Template)]
#[template(path = "hello.html")]
struct HelloTemplate {
    name: &'static str,
}

#[handler]
fn text_handler() -> &'static str {
    "Hello World"
}

#[handler]
fn json_handler() -> Json<Message> {
    Json(Message {
        message: "Hello World",
    })
}

#[handler]
async fn db_handler(pool: Data<&Pool<Postgres>>) -> poem::Result<Json<World>> {
    let mut conn = pool
        .acquire()
        .await
        .map_err(poem::error::InternalServerError)?;
    let world: World = sqlx::query_as("SELECT id, randomnumber FROM world WHERE id = 1")
        .fetch_one(&mut *conn)
        .await
        .map_err(poem::error::InternalServerError)?;

    Ok(Json(world))
}

#[handler]
fn html_handler() -> poem::Result<Html<String>> {
    let template = HelloTemplate { name: "World" };
    let html = template
        .render()
        .map_err(poem::error::InternalServerError)?;
    Ok(Html(html))
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://benchmark:benchmark@localhost:5432/benchmark".to_string());

    let pool = PgPoolOptions::new()
        .max_connections(50)
        .connect(&database_url)
        .await
        .expect("Failed to create pool.");

    let app = Route::new()
        .at("/text", get(text_handler))
        .at("/json", get(json_handler))
        .at("/db-single", get(db_handler).data(pool))
        .at("/html", get(html_handler));

    Server::new(TcpListener::bind("0.0.0.0:3000"))
        .run(app)
        .await
}
