use askama::Template;
use serde::Serialize;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::convert::Infallible;
use std::env;
use warp::Filter;

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

fn with_pool(
    pool: Pool<Postgres>,
) -> impl Filter<Extract = (Pool<Postgres>,), Error = Infallible> + Clone {
    warp::any().map(move || pool.clone())
}

async fn text_handler() -> Result<impl warp::Reply, Infallible> {
    Ok("Hello World")
}

async fn json_handler() -> Result<impl warp::Reply, Infallible> {
    Ok(warp::reply::json(&Message {
        message: "Hello World",
    }))
}

async fn db_handler(pool: Pool<Postgres>) -> Result<impl warp::Reply, warp::Rejection> {
    let mut conn = pool.acquire().await.map_err(|_| warp::reject::reject())?;
    let world: World = sqlx::query_as("SELECT id, randomnumber FROM world WHERE id = 1")
        .fetch_one(&mut *conn)
        .await
        .map_err(|_| warp::reject::reject())?;

    Ok(warp::reply::json(&world))
}

async fn html_handler() -> Result<impl warp::Reply, warp::Rejection> {
    let template = HelloTemplate { name: "World" };
    let html = template.render().map_err(|_| warp::reject::reject())?;
    Ok(warp::reply::html(html))
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

    let text_route = warp::path("text").and(warp::get()).and_then(text_handler);
    let json_route = warp::path("json").and(warp::get()).and_then(json_handler);
    let db_route = warp::path("db-single")
        .and(warp::get())
        .and(with_pool(pool))
        .and_then(db_handler);
    let html_route = warp::path("html").and(warp::get()).and_then(html_handler);

    let routes = text_route.or(json_route).or(db_route).or(html_route);

    warp::serve(routes).run(([0, 0, 0, 0], 3000)).await;

    Ok(())
}
