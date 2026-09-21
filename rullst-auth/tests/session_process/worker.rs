use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use rullst_auth::recovery::{RecoveryError, RecoverySecrets, SessionId, SqlRecoveryStore};
use rullst_core::{
    config::{Environment, SecurityConfig},
    security::{TenantMembership, apply_security_baseline},
};
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::io::AsyncReadExt;

#[derive(Serialize, Deserialize)]
pub struct Configuration {
    pub database: String,
    pub subject: String,
    pub ready: PathBuf,
}

pub fn keys() -> RecoverySecrets {
    RecoverySecrets::new([3; 32], [9; 32]).unwrap()
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[derive(Clone)]
struct App {
    store: SqlRecoveryStore,
    subject: String,
}

fn failure(error: RecoveryError) -> StatusCode {
    match error {
        RecoveryError::InvalidAction => StatusCode::UNAUTHORIZED,
        RecoveryError::InvalidInput => StatusCode::BAD_REQUEST,
        _ => StatusCode::SERVICE_UNAVAILABLE,
    }
}

async fn authorized(app: &App, headers: &HeaderMap, tenant: &str) -> Result<String, StatusCode> {
    let cookies: Vec<_> = headers.get_all(header::COOKIE).iter().collect();
    if cookies.len() != 1 {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let cookie = cookies[0].to_str().map_err(|_| StatusCode::UNAUTHORIZED)?;
    if cookie.len() > 1024 {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let tokens: Vec<_> = cookie
        .split(';')
        .filter_map(|pair| pair.trim().strip_prefix("session="))
        .collect();
    if tokens.len() != 1 || tokens[0].len() != 43 {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let subject = app
        .store
        .verify_session(tokens[0], now())
        .await
        .map_err(failure)?
        .ok_or(StatusCode::UNAUTHORIZED)?;
    // Fixture membership comes from the authenticated subject and host-owned
    // records. The route parameter cannot choose a database or confer membership.
    if subject != app.subject {
        return Err(StatusCode::FORBIDDEN);
    }
    TenantMembership::try_new(["school-a"])
        .unwrap()
        .select(tenant)
        .map_err(|_| StatusCode::FORBIDDEN)?;
    Ok(tokens[0].to_string())
}

async fn private(
    State(app): State<App>,
    Path(tenant): Path<String>,
    headers: HeaderMap,
) -> Result<StatusCode, StatusCode> {
    authorized(&app, &headers, &tenant).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn inventory(
    State(app): State<App>,
    Path(tenant): Path<String>,
    headers: HeaderMap,
) -> Result<Response, StatusCode> {
    let token = authorized(&app, &headers, &tenant).await?;
    let sessions = app
        .store
        .active_sessions(&token, now())
        .await
        .map_err(failure)?;
    let rows: Vec<_> = sessions
        .iter()
        .map(|session| {
            serde_json::json!({
                "id": session.id().as_str(), "created_at": session.created_at(),
                "expires_at": session.expires_at(), "current": session.is_current(),
                "label": session.label().map(|label| label.as_str()),
            })
        })
        .collect();
    Ok(([(header::CACHE_CONTROL, "no-store")], Json(rows)).into_response())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Target {
    target: String,
}

async fn revoke(
    State(app): State<App>,
    Path(tenant): Path<String>,
    headers: HeaderMap,
    Json(target): Json<Target>,
) -> Result<StatusCode, StatusCode> {
    let token = authorized(&app, &headers, &tenant).await?;
    let target = SessionId::new(target.target).map_err(failure)?;
    app.store
        .revoke_other_session(&token, &target, now())
        .await
        .map_err(failure)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn revoke_others(
    State(app): State<App>,
    Path(tenant): Path<String>,
    headers: HeaderMap,
) -> Result<StatusCode, StatusCode> {
    let token = authorized(&app, &headers, &tenant).await?;
    app.store
        .revoke_other_sessions(&token, now())
        .await
        .map_err(failure)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn serve() {
    let mut input = Vec::new();
    tokio::io::stdin()
        .take(8192)
        .read_to_end(&mut input)
        .await
        .unwrap();
    let config: Configuration = serde_json::from_slice(&input).unwrap();
    let store = SqlRecoveryStore::connect(&config.database, keys())
        .await
        .unwrap();
    let app = Router::new()
        .route("/tenants/{tenant}/private", get(private))
        .route("/tenants/{tenant}/sessions", get(inventory))
        .route("/tenants/{tenant}/sessions/revoke", post(revoke))
        .route(
            "/tenants/{tenant}/sessions/revoke-others",
            post(revoke_others),
        )
        .layer(DefaultBodyLimit::max(1024))
        .with_state(App {
            store,
            subject: config.subject,
        });
    let app =
        apply_security_baseline(app, SecurityConfig::default(), Environment::Production).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    tokio::fs::write(config.ready, listener.local_addr().unwrap().to_string())
        .await
        .unwrap();
    let _ = tokio::time::timeout(
        Duration::from_secs(60),
        axum::serve(listener, app).into_future(),
    )
    .await;
}
