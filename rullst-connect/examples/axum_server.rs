use axum::{
    Router,
    extract::Query,
    response::{Html, IntoResponse, Redirect},
    routing::get,
};
use rullst_connect::provider::Provider;
use rullst_connect::providers::github::GithubProvider;
use rullst_connect::providers::google::GoogleProvider;
use serde::Deserialize;

#[derive(Deserialize)]
struct AuthRequest {
    code: String,
}

// Em um projeto real, isso viria de variáveis de ambiente (.env)
fn google_client_id() -> String {
    std::env::var("GOOGLE_CLIENT_ID").unwrap_or_else(|_| "SEU_GOOGLE_CLIENT_ID".to_string())
}
fn google_client_secret() -> String {
    std::env::var("GOOGLE_CLIENT_SECRET").unwrap_or_else(|_| "SEU_GOOGLE_CLIENT_SECRET".to_string())
}
fn github_client_id() -> String {
    std::env::var("GITHUB_CLIENT_ID").unwrap_or_else(|_| "SEU_GITHUB_CLIENT_ID".to_string())
}
fn github_client_secret() -> String {
    std::env::var("GITHUB_CLIENT_SECRET").unwrap_or_else(|_| "SEU_GITHUB_CLIENT_SECRET".to_string())
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(index))
        .route("/auth/google", get(login_google))
        .route("/auth/google/callback", get(callback_google))
        .route("/auth/github", get(login_github))
        .route("/auth/github/callback", get(callback_github));

    println!("🚀 Servidor rodando em http://localhost:3000");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn index() -> Html<&'static str> {
    Html(
        r#"
        <h1>Rullst Connect Example</h1>
        <a href="/auth/google">Login com Google</a><br><br>
        <a href="/auth/github">Login com GitHub</a>
    "#,
    )
}

// ==========================================
// GOOGLE
// ==========================================
async fn login_google() -> Redirect {
    let provider = GoogleProvider::new(
        google_client_id(),
        google_client_secret().into(),
        "http://localhost:3000/auth/google/callback".to_string(),
    );
    Redirect::to(&provider.redirect_url())
}

async fn callback_google(Query(query): Query<AuthRequest>) -> impl IntoResponse {
    let provider = GoogleProvider::new(
        google_client_id(),
        google_client_secret().into(),
        "http://localhost:3000/auth/google/callback".to_string(),
    );

    let params = rullst_connect::provider::ExchangeParams {
        auth_code: &query.code,
        ..Default::default()
    };
    match provider.get_user(params).await {
        Ok(user) => Html(format!(
            "<h2>Welcome, {}!</h2><p>Email: {:?}</p><p>ID: {}</p><img src='{:?}'>",
            user.name, user.email, user.id, user.avatar_url
        )),
        Err(e) => Html(format!("Login error: {:?}", e)),
    }
}

// ==========================================
// GITHUB
// ==========================================
async fn login_github() -> Redirect {
    let provider = GithubProvider::new(
        github_client_id(),
        github_client_secret().into(),
        "http://localhost:3000/auth/github/callback".to_string(),
    );
    Redirect::to(&provider.redirect_url())
}

async fn callback_github(Query(query): Query<AuthRequest>) -> impl IntoResponse {
    let provider = GithubProvider::new(
        github_client_id(),
        github_client_secret().into(),
        "http://localhost:3000/auth/github/callback".to_string(),
    );

    let params = rullst_connect::provider::ExchangeParams {
        auth_code: &query.code,
        ..Default::default()
    };
    match provider.get_user(params).await {
        Ok(user) => Html(format!(
            "<h2>Welcome, {}!</h2><p>Email: {:?}</p><p>ID: {}</p><img src='{:?}'>",
            user.name, user.email, user.id, user.avatar_url
        )),
        Err(e) => Html(format!("Login error: {:?}", e)),
    }
}
