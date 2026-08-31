use std::collections::BTreeMap;
use std::sync::Arc;

use axum::extract::{Form, Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use secrecy::ExposeSecret;
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use super::MockIdpConfig;
use super::signing::MockSigner;

const AUTHORIZATION_TTL_SECONDS: u64 = 300;
const ACCESS_TOKEN_TTL_SECONDS: u64 = 3_600;
const MAX_ACTIVE_RECORDS: usize = 64;
const MAX_REQUEST_FIELD_BYTES: usize = 512;

#[derive(Deserialize)]
struct AuthorizationRequest {
    client_id: String,
    redirect_uri: String,
    response_type: String,
    scope: Option<String>,
    state: Option<String>,
    nonce: Option<String>,
    code_challenge: Option<String>,
    code_challenge_method: Option<String>,
}

#[derive(Deserialize)]
struct TokenRequest {
    client_id: String,
    client_secret: String,
    code: String,
    grant_type: String,
    redirect_uri: String,
    code_verifier: Option<String>,
}

struct AuthorizationGrant {
    client_id: String,
    redirect_uri: String,
    nonce: Option<String>,
    code_challenge: Option<String>,
    expires_at: u64,
    sequence: u64,
}

struct ProtocolState {
    next_sequence: u64,
    grants: BTreeMap<String, AuthorizationGrant>,
    access_tokens: BTreeMap<[u8; 32], u64>,
}

struct MockIdpState {
    config: MockIdpConfig,
    signer: MockSigner,
    protocol: tokio::sync::Mutex<ProtocolState>,
}

pub(super) fn router(config: MockIdpConfig) -> Router {
    let state = Arc::new(MockIdpState {
        config,
        signer: MockSigner::new(),
        protocol: tokio::sync::Mutex::new(ProtocolState {
            next_sequence: 0,
            grants: BTreeMap::new(),
            access_tokens: BTreeMap::new(),
        }),
    });
    Router::new()
        .route("/auth", get(authorize_handler))
        .route("/token", post(token_handler))
        .route("/userinfo", get(userinfo_handler))
        .route("/jwks", get(jwks_handler))
        .route("/.well-known/openid-configuration", get(discovery_handler))
        .with_state(state)
}

async fn authorize_handler(
    State(state): State<Arc<MockIdpState>>,
    Query(request): Query<AuthorizationRequest>,
) -> Response {
    if let Err(reason) = validate_authorization_request(&state.config, &request) {
        return oauth_error(StatusCode::BAD_REQUEST, "invalid_request", reason);
    }
    let now = match now_epoch() {
        Ok(now) => now,
        Err(error) => return server_error(error),
    };
    let code = {
        let mut protocol = state.protocol.lock().await;
        protocol.grants.retain(|_, grant| grant.expires_at > now);
        if protocol.grants.len() >= MAX_ACTIVE_RECORDS {
            return oauth_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "temporarily_unavailable",
                "the bounded authorization-code store is full",
            );
        }
        let Some(sequence) = protocol.next_sequence.checked_add(1) else {
            return oauth_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "temporarily_unavailable",
                "the authorization-code sequence is exhausted",
            );
        };
        protocol.next_sequence = sequence;
        let code = format!("rullst_mock_code_{sequence:016x}");
        protocol.grants.insert(
            code.clone(),
            AuthorizationGrant {
                client_id: request.client_id,
                redirect_uri: request.redirect_uri.clone(),
                nonce: request.nonce,
                code_challenge: request.code_challenge,
                expires_at: now.saturating_add(AUTHORIZATION_TTL_SECONDS),
                sequence,
            },
        );
        code
    };

    let mut redirect = match url::Url::parse(&request.redirect_uri) {
        Ok(redirect) => redirect,
        Err(error) => return server_error(error),
    };
    redirect.query_pairs_mut().append_pair("code", &code);
    if let Some(callback_state) = request.state {
        redirect
            .query_pairs_mut()
            .append_pair("state", &callback_state);
    }
    Redirect::to(redirect.as_str()).into_response()
}

async fn token_handler(
    State(state): State<Arc<MockIdpState>>,
    Form(request): Form<TokenRequest>,
) -> Response {
    if let Err(reason) = validate_token_request(&state.config, &request) {
        return oauth_error(StatusCode::BAD_REQUEST, "invalid_request", reason);
    }
    let now = match now_epoch() {
        Ok(now) => now,
        Err(error) => return server_error(error),
    };
    let grant = {
        let mut protocol = state.protocol.lock().await;
        protocol.grants.remove(&request.code)
    };
    let Some(grant) = grant else {
        return oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_grant",
            "the authorization code is invalid, expired, or already consumed",
        );
    };
    if grant.expires_at <= now
        || grant.client_id != request.client_id
        || grant.redirect_uri != request.redirect_uri
    {
        return oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_grant",
            "the authorization code binding is invalid or expired",
        );
    }
    if let Some(challenge) = grant.code_challenge.as_deref() {
        let Some(verifier) = request.code_verifier.as_deref() else {
            return oauth_error(
                StatusCode::BAD_REQUEST,
                "invalid_grant",
                "the PKCE verifier is required",
            );
        };
        if !(43..=128).contains(&verifier.len())
            || !verifier.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~')
            })
            || !crate::pkce::verify_pkce_challenge(verifier, challenge)
        {
            return oauth_error(
                StatusCode::BAD_REQUEST,
                "invalid_grant",
                "the PKCE verifier does not match the authorization grant",
            );
        }
    }

    let expires_at = now.saturating_add(ACCESS_TOKEN_TTL_SECONDS);
    let token_digest = Sha256::digest(format!("{}:{}", request.code, grant.sequence).as_bytes());
    let access_token = format!(
        "rullst_mock_access_{}",
        URL_SAFE_NO_PAD.encode(token_digest)
    );
    let id_token = match state.signer.sign_id_token(
        &state.config.issuer,
        &state.config.client_id,
        &state.config.user,
        grant.nonce.as_deref(),
        now,
        expires_at,
    ) {
        Ok(token) => token,
        Err(error) => return server_error(error),
    };
    {
        let mut protocol = state.protocol.lock().await;
        protocol.access_tokens.retain(|_, expiry| *expiry > now);
        if protocol.access_tokens.len() >= MAX_ACTIVE_RECORDS {
            return oauth_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "temporarily_unavailable",
                "the bounded access-token store is full",
            );
        }
        protocol
            .access_tokens
            .insert(hash_secret(&access_token), expires_at);
    }

    let mut response = Json(json!({
        "access_token": access_token,
        "token_type": "Bearer",
        "expires_in": ACCESS_TOKEN_TTL_SECONDS,
        "id_token": id_token
    }))
    .into_response();
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store"),
    );
    response
        .headers_mut()
        .insert(header::PRAGMA, header::HeaderValue::from_static("no-cache"));
    response
}

async fn userinfo_handler(State(state): State<Arc<MockIdpState>>, headers: HeaderMap) -> Response {
    let Some(token) = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split_once(' '))
        .filter(|(scheme, _)| scheme.eq_ignore_ascii_case("Bearer"))
        .map(|(_, token)| token)
        .filter(|value| !value.is_empty() && value.len() <= MAX_REQUEST_FIELD_BYTES)
    else {
        return bearer_error();
    };
    let now = match now_epoch() {
        Ok(now) => now,
        Err(error) => return server_error(error),
    };
    let authorized = {
        let mut protocol = state.protocol.lock().await;
        protocol.access_tokens.retain(|_, expiry| *expiry > now);
        protocol
            .access_tokens
            .get(&hash_secret(token))
            .is_some_and(|expiry| *expiry > now)
    };
    if !authorized {
        return bearer_error();
    }
    Json(json!({
        "sub": state.config.user.subject,
        "name": state.config.user.name,
        "email": state.config.user.email,
        "email_verified": true,
        "picture": state.config.user.picture
    }))
    .into_response()
}

async fn jwks_handler(State(state): State<Arc<MockIdpState>>) -> Response {
    Json(state.signer.jwks()).into_response()
}

async fn discovery_handler(State(state): State<Arc<MockIdpState>>) -> Response {
    let issuer = &state.config.issuer;
    Json(json!({
        "issuer": issuer,
        "authorization_endpoint": format!("{issuer}/auth"),
        "token_endpoint": format!("{issuer}/token"),
        "userinfo_endpoint": format!("{issuer}/userinfo"),
        "jwks_uri": format!("{issuer}/jwks"),
        "response_types_supported": ["code"],
        "grant_types_supported": ["authorization_code"],
        "subject_types_supported": ["public"],
        "id_token_signing_alg_values_supported": ["EdDSA"],
        "token_endpoint_auth_methods_supported": ["client_secret_post"],
        "code_challenge_methods_supported": ["S256"],
        "scopes_supported": ["openid", "profile", "email"],
        "claims_supported": ["sub", "name", "email", "email_verified", "picture", "nonce"]
    }))
    .into_response()
}

fn validate_authorization_request(
    config: &MockIdpConfig,
    request: &AuthorizationRequest,
) -> Result<(), &'static str> {
    validate_request_fields([
        request.client_id.as_str(),
        request.redirect_uri.as_str(),
        request.response_type.as_str(),
        request.scope.as_deref().unwrap_or_default(),
        request.state.as_deref().unwrap_or_default(),
        request.nonce.as_deref().unwrap_or_default(),
        request.code_challenge.as_deref().unwrap_or_default(),
        request.code_challenge_method.as_deref().unwrap_or_default(),
    ])?;
    if !secret_eq(&request.client_id, &config.client_id) {
        return Err("the client_id is not registered");
    }
    if request.redirect_uri != config.redirect_uri {
        return Err("the redirect_uri does not exactly match the registered URI");
    }
    if request.response_type != "code" {
        return Err("only response_type=code is supported");
    }
    if !request
        .scope
        .as_deref()
        .unwrap_or_default()
        .split_ascii_whitespace()
        .any(|scope| scope == "openid")
    {
        return Err("the openid scope is required");
    }
    match (
        request.code_challenge.as_deref(),
        request.code_challenge_method.as_deref(),
    ) {
        (Some(challenge), Some("S256"))
            if challenge.len() == 43
                && challenge
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_')) =>
        {
            Ok(())
        }
        (None, None) => Ok(()),
        _ => Err("PKCE must use one valid S256 code challenge"),
    }
}

fn validate_token_request(
    config: &MockIdpConfig,
    request: &TokenRequest,
) -> Result<(), &'static str> {
    validate_request_fields([
        request.client_id.as_str(),
        request.client_secret.as_str(),
        request.code.as_str(),
        request.grant_type.as_str(),
        request.redirect_uri.as_str(),
        request.code_verifier.as_deref().unwrap_or_default(),
    ])?;
    if !secret_eq(&request.client_id, &config.client_id)
        || !secret_eq(&request.client_secret, config.client_secret.expose_secret())
    {
        return Err("client authentication failed");
    }
    if request.grant_type != "authorization_code" {
        return Err("only grant_type=authorization_code is supported");
    }
    if request.redirect_uri != config.redirect_uri {
        return Err("the redirect_uri does not exactly match the registered URI");
    }
    Ok(())
}

fn validate_request_fields<'a>(
    fields: impl IntoIterator<Item = &'a str>,
) -> Result<(), &'static str> {
    if fields
        .into_iter()
        .any(|value| value.len() > MAX_REQUEST_FIELD_BYTES || value.chars().any(char::is_control))
    {
        return Err("request fields exceed the bounded input policy");
    }
    Ok(())
}

fn secret_eq(left: &str, right: &str) -> bool {
    bool::from(hash_secret(left).ct_eq(&hash_secret(right)))
}

fn hash_secret(value: &str) -> [u8; 32] {
    Sha256::digest(value.as_bytes()).into()
}

fn now_epoch() -> Result<u64, crate::ConnectError> {
    Ok(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs())
}

fn oauth_error(
    status: StatusCode,
    error: &'static str,
    description: impl Into<String>,
) -> Response {
    (
        status,
        Json(json!({
            "error": error,
            "error_description": description.into()
        })),
    )
        .into_response()
}

fn bearer_error() -> Response {
    let mut response = oauth_error(
        StatusCode::UNAUTHORIZED,
        "invalid_token",
        "a valid unexpired mock bearer token is required",
    );
    response.headers_mut().insert(
        header::WWW_AUTHENTICATE,
        header::HeaderValue::from_static("Bearer realm=\"rullst-mock-idp\""),
    );
    response
}

fn server_error(error: impl std::fmt::Display) -> Response {
    tracing::error!(target: "rullst_connect", error = %error, "local mock IdP operation failed");
    oauth_error(
        StatusCode::INTERNAL_SERVER_ERROR,
        "server_error",
        "the local mock IdP could not complete the operation",
    )
}
