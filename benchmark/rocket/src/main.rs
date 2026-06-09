#[macro_use] extern crate rocket;
use rocket::serde::{json::Json, Serialize};
use rocket::config::Config;

#[derive(Serialize)]
struct Message {
    message: &'static str,
}

#[get("/")]
fn plaintext() -> &'static str {
    "Hello, World!"
}

#[get("/json")]
fn json_endpoint() -> Json<Message> {
    Json(Message { message: "Hello, World!" })
}

#[launch]
fn rocket() -> _ {
    let config = Config {
        port: 8000,
        address: "0.0.0.0".parse().unwrap(),
        ..Config::default()
    };
    rocket::custom(config)
        .mount("/", routes![plaintext, json_endpoint])
}
