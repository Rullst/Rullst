use axum::{routing::get, Json, Router};
use leptos::*;
use leptos_axum::{generate_route_list, LeptosRoutes};
use serde::Serialize;

#[derive(Serialize)]
struct Message {
    message: &'static str,
}

#[component]
fn App() -> impl IntoView {
    view! {
        <h1>"Hello, World!"</h1>
    }
}

async fn plaintext() -> &'static str {
    "Hello, World!"
}

async fn json_endpoint() -> Json<Message> {
    Json(Message { message: "Hello, World!" })
}

#[tokio::main]
async fn main() {
    let leptos_options = LeptosOptions::builder()
        .output_name("bench-leptos")
        .site_addr("0.0.0.0:3000".parse::<std::net::SocketAddr>().unwrap())
        .build();

    let routes = generate_route_list(App);

    let app = Router::new()
        .route("/", get(plaintext))
        .route("/json", get(json_endpoint))
        .leptos_routes(&leptos_options, routes, App)
        .with_state(leptos_options);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
