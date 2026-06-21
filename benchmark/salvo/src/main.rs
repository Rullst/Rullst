use askama::Template;
use salvo::prelude::*;
use serde::Serialize;
use sqlx::{PgPool, FromRow};
use std::sync::Arc;

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

#[handler]
async fn text(res: &mut Response) {
    res.render(Text::Plain("Hello World"));
}

#[handler]
async fn json(res: &mut Response) {
    res.render(Json(Message {
        message: "Hello World".to_string(),
    }));
}

#[handler]
async fn db_single(depot: &mut Depot, res: &mut Response) {
    let pool = depot.obtain::<Arc<PgPool>>().unwrap();
    if let Ok(Some(rec)) = sqlx::query_as::<_, Record>("SELECT id, name FROM records LIMIT 1")
        .fetch_optional(&**pool)
        .await
    {
        res.render(Json(rec));
    } else {
        res.status_code(StatusCode::NOT_FOUND);
    }
}

#[handler]
async fn html(res: &mut Response) {
    let t = HelloTemplate { name: "World" };
    res.render(Text::Html(t.render().unwrap()));
}

#[tokio::main]
async fn main() {
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://benchuser:password@localhost/benchmark".to_string());

    let pool = Arc::new(PgPool::connect(&db_url).await.unwrap_or_else(|_| {
        eprintln!("Warning: Could not connect to database, db-single will fail.");
        PgPool::connect_lazy(&db_url).unwrap()
    }));

    let db_middleware = salvo::affix_state::inject(pool);

    let router = Router::new()
        .hoop(db_middleware)
        .push(Router::with_path("text").get(text))
        .push(Router::with_path("json").get(json))
        .push(Router::with_path("db-single").get(db_single))
        .push(Router::with_path("html").get(html));

    let acceptor = TcpListener::new(format!("0.0.0.0:{}", std::env::var("PORT").unwrap_or_else(|_| "8000".to_string()))).bind().await;
    Server::new(acceptor).serve(router).await;
}
