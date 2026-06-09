use rullst::{routes, Server};
use rullst::server::{Json, IntoResponse};
use serde::Serialize;

#[derive(Serialize)]
struct Message {
    message: &'static str,
}

async fn plaintext() -> &'static str {
    "Hello, World!"
}

async fn json_endpoint() -> impl IntoResponse {
    Json(Message { message: "Hello, World!" })
}

#[rullst::runtime::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let router = routes![
        get("/" => plaintext),
        get("/json" => json_endpoint),
    ];
    Server::new(router).run(3000).await?;
    Ok(())
}
