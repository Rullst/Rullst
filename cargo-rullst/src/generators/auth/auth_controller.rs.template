use crate::models::user::User;
use crate::pages::auth;
use rullst::auth as rullst_auth;
use rullst::server::{Extension, Form, HeaderMap, IntoResponse, Redirect, Response, StatusCode};
use serde::Deserialize;

const DUMMY_PASSWORD_HASH: &str =
    "$argon2id$v=19$m=19456,t=2,p=1$VE9CZ2d5dHVyWldOajNXZA$M0zU6o5hE/R6B+nJ9hX8+A";

#[derive(Deserialize)]
pub struct RegisterDto {
    pub name: String,
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct LoginDto {
    pub email: String,
    pub password: String,
}

fn normalize_email(email: &str) -> String {
    email.trim().to_ascii_lowercase()
}

fn valid_email(email: &str) -> bool {
    !email.is_empty()
        && email.len() <= 254
        && !email.contains(['\r', '\n'])
        && email.split_once('@').is_some_and(|(local, domain)| {
            !local.is_empty() && domain.contains('.') && !domain.ends_with('.')
        })
}

fn get_csrf_token(headers: &HeaderMap) -> String {
    headers
        .get(rullst::server::header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(|cookie_header| {
            cookie_header.split(';').find_map(|cookie| {
                cookie
                    .trim()
                    .strip_prefix("rullst_csrf=")
                    .map(ToOwned::to_owned)
            })
        })
        .unwrap_or_default()
}

fn get_csp_nonce(nonce: &Option<Extension<rullst::security::CspNonce>>) -> &str {
    nonce
        .as_ref()
        .map(|Extension(nonce)| nonce.as_str())
        .unwrap_or_default()
}

fn redirect_with_cookie(target: &'static str, cookie: &str) -> Response {
    let Ok(header_value) = rullst::server::HeaderValue::from_bytes(cookie.as_bytes()) else {
        return (StatusCode::INTERNAL_SERVER_ERROR, "Invalid session cookie").into_response();
    };
    let mut response = Redirect::to(target).into_response();
    response
        .headers_mut()
        .insert(rullst::server::header::SET_COOKIE, header_value);
    response
}

pub async fn login_view(
    headers: HeaderMap,
    csrf: Option<Extension<rullst::security::CsrfToken>>,
    csp_nonce: Option<Extension<rullst::security::CspNonce>>,
) -> impl IntoResponse {
    let token = csrf
        .as_ref()
        .map(|Extension(token)| token.as_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| get_csrf_token(&headers));
    auth::login_page(&token, None, get_csp_nonce(&csp_nonce))
}

pub async fn login_submit(
    headers: HeaderMap,
    csp_nonce: Option<Extension<rullst::security::CspNonce>>,
    Form(payload): Form<LoginDto>,
) -> Response {
    let token = get_csrf_token(&headers);
    let nonce = get_csp_nonce(&csp_nonce);
    let email = normalize_email(&payload.email);
    let user = match User::find_by_email(&email).await {
        Ok(user) => user,
        Err(error) => {
            eprintln!("Authentication query failed: {error}");
            return auth::login_page(
                &token,
                Some("Authentication is temporarily unavailable"),
                nonce,
            )
            .into_response();
        }
    };

    let password_hash = user
        .as_ref()
        .and_then(|candidate| candidate.password_hash.as_deref())
        .unwrap_or(DUMMY_PASSWORD_HASH)
        .to_owned();
    let password_valid = rullst_auth::verify_password_async(payload.password, password_hash).await;

    let Some(user) = user.filter(|_| password_valid) else {
        return auth::login_page(&token, Some("Incorrect email or password"), nonce)
            .into_response();
    };

    match rullst_auth::make_login_cookie(user.id) {
        Ok(cookie) => redirect_with_cookie("/dashboard", &cookie),
        Err(error) => {
            eprintln!("Session creation failed: {error}");
            auth::login_page(&token, Some("Error starting session"), nonce).into_response()
        }
    }
}

pub async fn register_view(
    headers: HeaderMap,
    csrf: Option<Extension<rullst::security::CsrfToken>>,
    csp_nonce: Option<Extension<rullst::security::CspNonce>>,
) -> impl IntoResponse {
    let token = csrf
        .as_ref()
        .map(|Extension(token)| token.as_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| get_csrf_token(&headers));
    auth::register_page(&token, None, get_csp_nonce(&csp_nonce))
}

pub async fn register_submit(
    headers: HeaderMap,
    csp_nonce: Option<Extension<rullst::security::CspNonce>>,
    Form(payload): Form<RegisterDto>,
) -> Response {
    let token = get_csrf_token(&headers);
    let nonce = get_csp_nonce(&csp_nonce);
    let email = normalize_email(&payload.email);
    if !valid_email(&email) {
        return auth::register_page(&token, Some("Enter a valid email address"), nonce)
            .into_response();
    }
    if !(12..=72).contains(&payload.password.len()) {
        return auth::register_page(
            &token,
            Some("Password must contain between 12 and 72 characters"),
            nonce,
        )
        .into_response();
    }

    match User::find_by_email(&email).await {
        Ok(Some(_)) => {
            return auth::register_page(&token, Some("Email already registered"), nonce)
                .into_response();
        }
        Ok(None) => {}
        Err(error) => {
            eprintln!("Registration query failed: {error}");
            return auth::register_page(
                &token,
                Some("Registration is temporarily unavailable"),
                nonce,
            )
            .into_response();
        }
    }

    let password_hash = match rullst_auth::hash_password_async(payload.password).await {
        Ok(hash) => hash,
        Err(error) => {
            eprintln!("Password hashing failed: {error}");
            return auth::register_page(&token, Some("Error processing password"), nonce)
                .into_response();
        }
    };

    let mut user = User {
        id: 0,
        name: payload.name.trim().to_owned(),
        email,
        password_hash: Some(password_hash),
        oauth_provider: None,
        oauth_id: None,
        created_at: String::new(),
        updated_at: String::new(),
    };
    if let Err(error) = user.save().await {
        eprintln!("User creation failed: {error}");
        return auth::register_page(&token, Some("Error creating account"), nonce).into_response();
    }

    match rullst_auth::make_login_cookie(user.id) {
        Ok(cookie) => redirect_with_cookie("/dashboard", &cookie),
        Err(error) => {
            eprintln!("Session creation failed: {error}");
            Redirect::to("/login").into_response()
        }
    }
}

pub async fn logout() -> Response {
    redirect_with_cookie("/login", &rullst_auth::make_logout_cookie())
}

pub async fn dashboard(
    rullst::server::Extension(user_id): rullst::server::Extension<i32>,
    csp_nonce: Option<Extension<rullst::security::CspNonce>>,
) -> Response {
    match User::find(user_id).await {
        Ok(Some(user)) => {
            auth::dashboard_page(&user.name, get_csp_nonce(&csp_nonce)).into_response()
        }
        Ok(None) => Redirect::to("/login").into_response(),
        Err(error) => {
            eprintln!("Dashboard user query failed: {error}");
            StatusCode::SERVICE_UNAVAILABLE.into_response()
        }
    }
}
