use actix_web::{get, App, HttpResponse, HttpServer, Responder};
use serde::Serialize;

#[derive(Serialize)]
struct Message {
    message: &'static str,
}

#[get("/")]
async fn plaintext() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/plain; charset=utf-8")
        .body("Hello, World!")
}

#[get("/json")]
async fn json_endpoint() -> impl Responder {
    HttpResponse::Ok().json(Message { message: "Hello, World!" })
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(plaintext)
            .service(json_endpoint)
    })
    .bind(("0.0.0.0", 3000))?
    .run()
    .await
}
