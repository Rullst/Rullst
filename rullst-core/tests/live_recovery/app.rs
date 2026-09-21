use axum::{
    Router,
    extract::{Path, State, WebSocketUpgrade},
    http::{HeaderMap, StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use rullst_core::{
    config::{Environment, SecurityConfig},
    live::recovery::*,
    security::{TenantMembership, apply_security_baseline},
};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool, sqlite::SqlitePoolOptions};
use std::{net::SocketAddr, path::PathBuf, sync::Arc, time::Duration};

pub const TEACHER: &str = "fixture-teacher-a";
pub const FRESH_TEACHER: &str = "fixture-fresh-teacher-a";
pub const LEARNER: &str = "fixture-learner-a";
pub const OTHER: &str = "fixture-teacher-b";
pub const CSRF: &str = "12345678901234567890123456789012";

#[derive(Deserialize, Serialize)]
pub struct Configuration {
    pub database: String,
    pub port: u16,
    pub ready: PathBuf,
}

#[derive(Clone)]
struct App {
    pool: SqlitePool,
    live: LiveRecovery,
}

pub struct Server {
    pub address: SocketAddr,
    pub pool: SqlitePool,
    task: tokio::task::JoinHandle<()>,
}

impl Server {
    pub async fn start(database: &str, port: u16) -> Self {
        Self::with_limits(database, port, 8, 32, Duration::from_secs(120)).await
    }
    pub async fn with_limits(
        database: &str,
        port: u16,
        connections: usize,
        actions: usize,
        lifetime: Duration,
    ) -> Self {
        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect(database)
            .await
            .unwrap();
        sqlx::query("CREATE TABLE IF NOT EXISTS counters(tenant TEXT PRIMARY KEY, revision INTEGER NOT NULL, value INTEGER NOT NULL)").execute(&pool).await.unwrap();
        sqlx::query("CREATE TABLE IF NOT EXISTS sessions(token TEXT PRIMARY KEY, account TEXT NOT NULL, tenant TEXT NOT NULL, role TEXT NOT NULL, active INTEGER NOT NULL)").execute(&pool).await.unwrap();
        for (token, account, tenant, role) in [
            (TEACHER, "teacher-a", "school-a", "teacher"),
            (FRESH_TEACHER, "teacher-a", "school-a", "teacher"),
            (LEARNER, "learner-a", "school-a", "learner"),
            (OTHER, "teacher-b", "school-b", "teacher"),
        ] {
            sqlx::query("INSERT OR IGNORE INTO sessions VALUES(?,?,?,?,1)")
                .bind(token)
                .bind(account)
                .bind(tenant)
                .bind(role)
                .execute(&pool)
                .await
                .unwrap();
            sqlx::query("INSERT OR IGNORE INTO counters VALUES(?,0,0)")
                .bind(tenant)
                .execute(&pool)
                .await
                .unwrap();
        }
        let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port))
            .await
            .unwrap();
        let address = listener.local_addr().unwrap();
        let config = LiveRecoveryConfig::loopback_for_tests(format!("http://{address}"))
            .unwrap()
            .with_capacity(connections, actions)
            .unwrap()
            .with_timing(
                Duration::from_millis(200),
                Duration::from_millis(200),
                lifetime,
            )
            .unwrap();
        let state = Arc::new(App {
            pool: pool.clone(),
            live: LiveRecovery::new(config),
        });
        let router = Router::new()
            .route("/", get(page))
            .route("/live/{tenant}", get(upgrade))
            .route("/live-module.js", get(module))
            .route("/app.js", get(app_script))
            .route("/test/revoke", post(revoke))
            .with_state(state);
        let router =
            apply_security_baseline(router, SecurityConfig::default(), Environment::Production)
                .unwrap();
        let task = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        Self {
            address,
            pool,
            task,
        }
    }
    pub async fn stop(self) {
        self.task.abort();
        let _ = self.task.await;
        self.pool.close().await;
    }
}

fn token(headers: &HeaderMap) -> &str {
    headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(|cookies| {
            cookies
                .split(';')
                .map(str::trim)
                .find_map(|part| part.strip_prefix("live_fixture="))
        })
        .unwrap_or("")
}

async fn scope(pool: &SqlitePool, token: &str, requested: &str) -> Result<LiveScope, StatusCode> {
    let row = sqlx::query("SELECT account,tenant FROM sessions WHERE token=? AND active=1")
        .bind(token)
        .fetch_optional(pool)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let account: String = row.get("account");
    let tenant: String = row.get("tenant");
    let membership = TenantMembership::try_new([tenant]).map_err(|_| StatusCode::FORBIDDEN)?;
    let tenant = membership
        .select(requested)
        .map_err(|_| StatusCode::FORBIDDEN)?;
    LiveScope::try_new(&tenant, account, "counter").map_err(|_| StatusCode::FORBIDDEN)
}

async fn upgrade(
    State(app): State<Arc<App>>,
    Path(tenant): Path<String>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    let token = token(&headers).to_owned();
    let scope = match scope(&app.pool, &token, &tenant).await {
        Ok(scope) => scope,
        Err(status) => return status.into_response(),
    };
    app.live
        .upgrade(
            ws,
            &headers,
            scope,
            View {
                pool: app.pool.clone(),
                token,
            },
        )
        .await
}

struct View {
    pool: SqlitePool,
    token: String,
}

impl RecoverableLiveView for View {
    async fn authorize(&self, scope: &LiveScope) -> LiveResult<()> {
        let valid: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sessions WHERE token=? AND account=? AND tenant=? AND active=1",
        )
        .bind(&self.token)
        .bind(scope.account())
        .bind(scope.tenant())
        .fetch_one(&self.pool)
        .await
        .map_err(|_| LiveRecoveryError::Unavailable)?;
        if valid == 1 {
            Ok(())
        } else {
            Err(LiveRecoveryError::Unauthorized)
        }
    }
    async fn snapshot(&self, scope: &LiveScope) -> LiveResult<LiveSnapshot> {
        let row = sqlx::query("SELECT revision,value FROM counters WHERE tenant=?")
            .bind(scope.tenant())
            .fetch_one(&self.pool)
            .await
            .map_err(|_| LiveRecoveryError::Unavailable)?;
        let revision: i64 = row.get("revision");
        let value: i64 = row.get("value");
        LiveSnapshot::try_new(
            revision as u64,
            format!(
                "<output id=\"value\">{value}</output><button type=\"button\" id=\"increment\" data-live-action=\"increment\">Increment</button>"
            ),
        )
    }
    async fn apply(&self, scope: &LiveScope, command: &LiveCommand) -> LiveResult<LiveSnapshot> {
        if command.action() == "slow" {
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        if !matches!(
            command.action(),
            "increment" | "increment-drop" | "increment-revoke"
        ) {
            return Err(LiveRecoveryError::Invalid);
        }
        let role: String =
            sqlx::query_scalar("SELECT role FROM sessions WHERE token=? AND active=1")
                .bind(&self.token)
                .fetch_one(&self.pool)
                .await
                .map_err(|_| LiveRecoveryError::Unauthorized)?;
        if role != "teacher" {
            return Err(LiveRecoveryError::Unauthorized);
        }
        let expected =
            i64::try_from(command.expected_revision()).map_err(|_| LiveRecoveryError::Conflict)?;
        // One bound statement couples authorization, the revision comparison and mutation.
        let changed = sqlx::query("UPDATE counters SET revision=revision+1,value=value+1 WHERE tenant=? AND revision=? AND EXISTS(SELECT 1 FROM sessions WHERE token=? AND tenant=? AND account=? AND active=1 AND role='teacher')")
            .bind(scope.tenant()).bind(expected).bind(&self.token).bind(scope.tenant()).bind(scope.account()).execute(&self.pool).await.map_err(|_| LiveRecoveryError::Unavailable)?.rows_affected();
        if changed != 1 {
            return Err(LiveRecoveryError::Conflict);
        }
        if command.action() == "increment-drop" {
            return Err(LiveRecoveryError::Unavailable);
        }
        if command.action() == "increment-revoke" {
            sqlx::query("UPDATE sessions SET active=0 WHERE token=?")
                .bind(&self.token)
                .execute(&self.pool)
                .await
                .unwrap();
        }
        self.snapshot(scope).await
    }
}

async fn page(State(app): State<Arc<App>>, headers: HeaderMap) -> Response {
    if let Err(status) = scope(&app.pool, token(&headers), "school-a").await {
        return status.into_response();
    }
    Html("<!doctype html><html lang=en><head><meta charset=utf-8><title>Live recovery fixture</title></head><body><main id=view>Connecting</main><p id=status role=status>connecting</p><script type=module src=/app.js></script></body></html>").into_response()
}
async fn module() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "text/javascript"),
            (header::CACHE_CONTROL, "no-store"),
        ],
        LIVE_RECOVERY_MODULE,
    )
}
async fn app_script() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "text/javascript"),
            (header::CACHE_CONTROL, "no-store"),
        ],
        "import {connectLive} from '/live-module.js'; window.outcomes=[]; const root=document.getElementById('view'); root.addEventListener('rullst:live-result',e=>window.outcomes.push(e.detail)); window.live=connectLive(root,'/live/school-a',document.getElementById('status'));",
    )
}
async fn revoke(State(app): State<Arc<App>>, headers: HeaderMap) -> Response {
    let current = token(&headers);
    if current != TEACHER || scope(&app.pool, current, "school-a").await.is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    sqlx::query("UPDATE sessions SET active=0 WHERE token=?")
        .bind(TEACHER)
        .execute(&app.pool)
        .await
        .unwrap();
    StatusCode::NO_CONTENT.into_response()
}
