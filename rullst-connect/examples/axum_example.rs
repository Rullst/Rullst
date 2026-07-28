use axum::{
    Router,
    response::{Html, IntoResponse},
    routing::get,
};
use rullst_connect::{extractors::AuthCallback, provider::Provider, providers::GoogleProvider};

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(home))
        .route("/login", get(login))
        .route("/callback", get(callback));

    let addr = "127.0.0.1:3000";
    println!("Listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn home() -> Html<&'static str> {
    Html("<h1>Rullst Connect Axum Example</h1><a href='/login'>Login with Google</a>")
}

async fn login() -> impl IntoResponse {
    let provider = get_provider();
    let url = provider.redirect_url_with_state("some_random_state_xyz");

    // Redirect to Google
    axum::response::Redirect::to(&url)
}

// Notice how we magically extract the callback parameters using `AuthCallback` directly!
async fn callback(auth: AuthCallback) -> impl IntoResponse {
    if let Some(error) = auth.error {
        return Html(format!("<h1>Error: {}</h1>", error));
    }

    if let Some(code) = auth.code {
        let provider = get_provider();
        let params = rullst_connect::provider::ExchangeParams {
            auth_code: &code,
            ..Default::default()
        };
        match provider.get_user(params).await {
            Ok(user) => Html(format!(
                "<h1>Welcome, {}!</h1><img src='{}' />",
                user.name,
                user.avatar_url.unwrap_or_default()
            )),
            Err(e) => Html(format!("<h1>Failed to get user: {}</h1>", e)),
        }
    } else {
        Html("<h1>No code provided</h1>".to_string())
    }
}

fn get_provider() -> GoogleProvider {
    GoogleProvider::new(
        std::env::var("GOOGLE_CLIENT_ID").unwrap_or_else(|_| "your_client_id".to_string()),
        std::env::var("GOOGLE_CLIENT_SECRET")
            .unwrap_or_else(|_| "your_client_secret".to_string())
            .into(),
        "http://localhost:3000/callback".to_string(),
    )
}
