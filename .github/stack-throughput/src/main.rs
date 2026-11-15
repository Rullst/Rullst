//! The application of `.github/check-stack-throughput.py`.
//!
//! `STACK=generated` (the default) serves the router as a generated
//! application does: `Server::new(router()).run(port)`, with the generated
//! router's CSRF and security-header layers, so `RULLST_ENV=production` adds
//! the production baseline and the access log. `STACK=minimal` serves the
//! same routes without any layer through `axum::serve`, as Rullst/Benchmarks'
//! `rullst-minimal` does. The check compares the two on one machine.

use rullst::server::Json;
use rullst::{Markup, Router, Server, html, routes};
use serde::Serialize;

#[derive(Serialize)]
struct Message {
    message: &'static str,
}

async fn text() -> &'static str {
    "Hello World"
}

async fn json() -> Json<Message> {
    Json(Message {
        message: "Hello World",
    })
}

async fn page() -> Markup {
    let name = "World";
    html! {
        <!DOCTYPE html>
        <html><head><title>Benchmark</title></head><body><h1>Hello {name}</h1></body></html>
    }
}

fn routes() -> Router {
    routes![
        get("/text" => text),
        get("/json" => json),
        get("/html" => page),
    ]
}

/// The router of the generated `src/lib.rs`.
fn generated() -> Router {
    routes()
        .layer(rullst::server::from_fn(rullst::security::csrf_middleware))
        .merge_axum(rullst::health::health_router())
        .layer(rullst::server::from_fn(rullst::security::headers_middleware))
}

#[rullst::runtime::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = std::env::var("PORT")
        .ok()
        .and_then(|port| port.parse().ok())
        .unwrap_or(8080_u16);
    if std::env::var("STACK").as_deref() == Ok("minimal") {
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
        rullst::web::axum::serve(listener, routes().into_axum()).await?;
    } else {
        Server::new(generated()).run(port).await?;
    }
    Ok(())
}
