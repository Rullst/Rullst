use actix_web::{get, web, HttpResponse, Responder};
use askama::Template;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};

#[derive(Serialize, Deserialize)]
struct Message {
    message: String,
}

#[derive(Serialize, Deserialize)]
struct User {
    id: i32,
    name: String,
}

#[derive(Template)]
#[template(source = "<h1>Hello, {{ name }}!</h1>", ext = "html")]
struct HelloTemplate<'a> {
    name: &'a str,
}

pub struct AppState {
    pub db: PgPool,
}

#[get("/text")]
async fn text_handler() -> impl Responder {
    HttpResponse::Ok().body("Hello World")
}

#[get("/json")]
async fn json_handler() -> impl Responder {
    web::Json(Message {
        message: "Hello World".to_string(),
    })
}

#[get("/html")]
async fn html_handler() -> impl Responder {
    let template = HelloTemplate { name: "World" };
    match template.render() {
        Ok(html) => HttpResponse::Ok().content_type("text/html").body(html),
        Err(_) => HttpResponse::InternalServerError().body("Template error"),
    }
}

#[get("/db-single")]
async fn db_single_handler(data: web::Data<AppState>) -> impl Responder {
    match sqlx::query("SELECT id, name FROM users LIMIT 1")
        .fetch_one(&data.db)
        .await
    {
        Ok(row) => {
            let user = User {
                id: row.get("id"),
                name: row.get("name"),
            };
            HttpResponse::Ok().json(user)
        }
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

pub fn configure_app(cfg: &mut web::ServiceConfig) {
    cfg.service(text_handler)
       .service(json_handler)
       .service(html_handler)
       .service(db_single_handler);
}
