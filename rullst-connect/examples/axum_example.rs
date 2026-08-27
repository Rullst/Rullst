use axum::{
    Router,
    http::StatusCode,
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

async fn login() -> Result<impl IntoResponse, (StatusCode, String)> {
    let provider = get_provider().map_err(internal_error)?;
    let url = provider.redirect_url_with_state("some_random_state_xyz");

    // Redirect to Google
    Ok(axum::response::Redirect::to(&url))
}

fn html_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

// Notice how we magically extract the callback parameters using `AuthCallback` directly!
async fn callback(auth: AuthCallback) -> impl IntoResponse {
    if let Some(error) = auth.error {
        let safe_error = html_escape(&error);
        return Html(format!("<h1>Error: {}</h1>", safe_error));
    }

    if let Some(code) = auth.code {
        let provider = match get_provider() {
            Ok(provider) => provider,
            Err(error) => {
                let safe_error = html_escape(&error.to_string());
                return Html(format!("<h1>Invalid provider: {}</h1>", safe_error));
            }
        };
        let params = rullst_connect::provider::ExchangeParams {
            auth_code: &code,
            ..Default::default()
        };
        match provider.get_user(params).await {
            Ok(user) => {
                let safe_name = html_escape(&user.name);
                let safe_avatar = html_escape(&user.avatar_url.unwrap_or_default());
                Html(format!(
                    "<h1>Welcome, {}!</h1><img src='{}' />",
                    safe_name, safe_avatar
                ))
            }
            Err(e) => {
                let safe_error = html_escape(&e.to_string());
                Html(format!("<h1>Failed to get user: {}</h1>", safe_error))
            }
        }
    } else {
        Html("<h1>No code provided</h1>".to_string())
    }
}

fn get_provider() -> Result<GoogleProvider, rullst_connect::ConnectError> {
    GoogleProvider::try_new(
        std::env::var("GOOGLE_CLIENT_ID").unwrap_or_else(|_| "mock_google_client_id".to_string()),
        std::env::var("GOOGLE_CLIENT_SECRET")
            .unwrap_or_else(|_| "mock_google_client_secret".to_string())
            .into(),
        "http://localhost:3000/callback".to_string(),
    )
}

fn internal_error(error: rullst_connect::ConnectError) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}
