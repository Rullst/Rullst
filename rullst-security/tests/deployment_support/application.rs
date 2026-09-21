//! No public management or real user account: control uses the parent's stdin pipe.
use axum::{
    Extension, Json, Router,
    body::{Body, Bytes},
    extract::{ConnectInfo, State, WebSocketUpgrade},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use futures_util::{StreamExt, stream};
use rullst_core::{Server, health::health_router_with_lifecycle, lifecycle::ApplicationLifecycle};
use rullst_security::{
    cswsh::{CswsPolicy, cswsh_guard_middleware},
    rate_limit::RedisRateLimiter,
};
use serde_json::json;
use std::{
    convert::Infallible,
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    sync::{Semaphore, oneshot, watch},
};

#[derive(Clone)]
struct App {
    id: String,
    limiter: RedisRateLimiter,
    release: Arc<Semaphore>,
    close: watch::Receiver<bool>,
    websockets: Arc<AtomicUsize>,
}

async fn identity(
    State(app): State<App>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Json<serde_json::Value> {
    // Default policy remains the socket peer. Raw proxy headers are diagnostic
    // output in this owned fixture and never establish a user or quota identity.
    Json(json!({"replica":app.id,"peer":peer.ip().to_string(),
        "xff":headers.get("x-forwarded-for").and_then(|v|v.to_str().ok()),
        "forwarded":headers.get("forwarded").and_then(|v|v.to_str().ok())}))
}

async fn limited(State(app): State<App>, request: axum::extract::Request) -> Response {
    let key = rullst_core::resilience::default_key_extractor(&request);
    match tokio::time::timeout(Duration::from_millis(500), app.limiter.check(&key)).await {
        Ok(Ok(decision)) if decision.allowed => {
            Json(json!({"replica":app.id,"remaining":decision.remaining})).into_response()
        }
        Ok(Ok(_)) => StatusCode::TOO_MANY_REQUESTS.into_response(),
        _ => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}

async fn streamed(State(app): State<App>) -> Response {
    let id = app.id;
    let first = stream::iter([Ok::<_, Infallible>(Bytes::from(format!("{id}:begin\n")))]);
    let tail = stream::once(async move {
        let permit = app.release.acquire().await.unwrap();
        permit.forget();
        Ok::<_, Infallible>(Bytes::from(format!("{id}:end\n")))
    });
    (
        [("content-type", "text/plain")],
        Body::from_stream(first.chain(tail)),
    )
        .into_response()
}

async fn websocket(State(app): State<App>, upgrade: WebSocketUpgrade) -> Response {
    upgrade.max_message_size(1024).max_frame_size(1024).on_upgrade(move |mut socket| async move {
        app.websockets.fetch_add(1, Ordering::SeqCst);
        let mut close = app.close.clone();
        let lifetime = tokio::time::sleep(Duration::from_secs(30));
        tokio::pin!(lifetime);
        loop {
            if *close.borrow() { break; }
            tokio::select! {
                _ = close.changed() => break,
                _ = &mut lifetime => break,
                message = socket.recv() => match message {
                    Some(Ok(axum::extract::ws::Message::Text(value))) => {
                        if socket.send(axum::extract::ws::Message::Text(format!("{}:{value}",app.id).into())).await.is_err() { break; }
                    }
                    _ => break,
                }
            }
        }
        let _ = socket.send(axum::extract::ws::Message::Close(Some(axum::extract::ws::CloseFrame {
            code: 1012, reason: "fixture drain".into(),
        }))).await;
        app.websockets.fetch_sub(1, Ordering::SeqCst);
    })
}

pub(super) async fn run() {
    assert_eq!(
        std::env::var("RULLST_DEPLOYMENT_DISPOSABLE").as_deref(),
        Ok("1")
    );
    assert_eq!(std::env::var("RULLST_HOST").as_deref(), Ok("127.0.0.1"));
    let id = std::env::var("RULLST_DEPLOYMENT_REPLICA").unwrap();
    assert!(matches!(id.as_str(), "a" | "b"));
    let port: u16 = std::env::var("RULLST_DEPLOYMENT_PORT")
        .unwrap()
        .parse()
        .unwrap();
    assert_ne!(port, 0);
    let redis = std::env::var("RULLST_DEPLOYMENT_REDIS").unwrap();
    assert!(redis.starts_with("redis://127.0.0.1:"));
    let limiter =
        RedisRateLimiter::new(redis, "owned-deployment-v1", 6, Duration::from_secs(60)).unwrap();
    limiter.require_distributed().unwrap();
    let lifecycle = ApplicationLifecycle::with_required_components(["fixture-dependency"]).unwrap();
    let (close, closed) = watch::channel(false);
    let app = App {
        id,
        limiter,
        release: Arc::new(Semaphore::new(0)),
        close: closed,
        websockets: Arc::new(AtomicUsize::new(0)),
    };
    let origin = std::env::var("RULLST_DEPLOYMENT_ORIGIN").unwrap();
    assert!(origin.starts_with("http://127.0.0.1:"));
    let websocket = Router::new()
        .route("/socket", get(websocket))
        .layer(axum::middleware::from_fn(cswsh_guard_middleware))
        .layer(Extension(CswsPolicy::try_new([origin]).unwrap()));
    let routes = Router::new()
        .route("/identity", get(identity))
        .route("/limited", get(limited))
        .route("/stream", get(streamed))
        .route("/bounded", post(|body: Bytes| async move { body }))
        .layer(axum::extract::DefaultBodyLimit::max(1024))
        .merge(websocket)
        .with_state(app.clone())
        .merge(health_router_with_lifecycle(lifecycle.clone()));
    let router = rullst_core::Router::new().merge_axum(routes);
    let (shutdown, stopped) = oneshot::channel();
    let server = tokio::spawn(
        Server::new(router)
            .with_lifecycle(lifecycle.clone())
            .run_with_shutdown(port, async {
                let _ = stopped.await;
            }),
    );
    let mut commands = BufReader::new(tokio::io::stdin()).lines();
    while let Some(command) = commands.next_line().await.unwrap() {
        assert!(command.len() <= 32);
        match command.as_str() {
            "ready" => lifecycle
                .set_component_ready("fixture-dependency", true)
                .unwrap(),
            "unready" => lifecycle
                .set_component_ready("fixture-dependency", false)
                .unwrap(),
            "release" => app.release.add_permits(1),
            "drain" => {
                lifecycle.begin_draining().unwrap();
                close.send(true).unwrap();
            }
            "snapshot" => {}
            "stop" => break,
            _ => panic!("unrecognized owned fixture command"),
        }
        println!(
            "FIXTURE {}",
            json!({"command":command,"replica":app.id,
            "lifecycle":lifecycle.snapshot(),"websockets":app.websockets.load(Ordering::SeqCst)})
        );
    }
    let _ = close.send(true);
    let _ = shutdown.send(());
    tokio::time::timeout(Duration::from_secs(5), server)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        while app.websockets.load(Ordering::SeqCst) != 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
}
