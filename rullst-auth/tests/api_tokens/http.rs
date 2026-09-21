#[allow(dead_code, unused_imports)]
pub mod support;
use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, Request, State},
    http::{Method, StatusCode, header},
    response::Response,
    routing::post,
};
use rullst_auth::recovery::SystemAuthClock;
use rullst_core::{
    config::{Environment, SecurityConfig},
    security::{MachineEndpoint, MachineEndpointPolicy, TenantMembership, apply_security_baseline},
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use support::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Clone)]
struct App {
    subject: String,
    writes: Arc<AtomicUsize>,
}
async fn write(
    State(app): State<App>,
    Extension(principal): Extension<ApiTokenPrincipal>,
    Json(body): Json<serde_json::Value>,
) -> Result<StatusCode, StatusCode> {
    if principal.subject() != app.subject {
        return Err(StatusCode::FORBIDDEN);
    }
    let tenant = body
        .get("tenant")
        .and_then(|value| value.as_str())
        .ok_or(StatusCode::BAD_REQUEST)?;
    TenantMembership::try_new(["school-a"])
        .unwrap()
        .select(tenant)
        .map_err(|_| StatusCode::FORBIDDEN)?;
    if body.get("order").and_then(|value| value.as_str()) != Some("bounded-order") {
        return Err(StatusCode::BAD_REQUEST);
    }
    app.writes.fetch_add(1, Ordering::SeqCst);
    Ok(StatusCode::NO_CONTENT)
}
struct Server {
    address: std::net::SocketAddr,
    stop: tokio::sync::oneshot::Sender<()>,
    task: tokio::task::JoinHandle<()>,
}
impl Server {
    async fn start(service: &ApiTokenService, subject: &str, writes: Arc<AtomicUsize>) -> Self {
        let verifier = service
            .machine_verifier(ApiScopes::new(["orders:write"]).unwrap())
            .unwrap();
        let policy = MachineEndpointPolicy::new(vec![
            MachineEndpoint::verified_bearer(Method::POST, "/api/orders", verifier).unwrap(),
        ])
        .unwrap();
        let router = Router::new()
            .route("/api/orders", post(write))
            .route("/api/orders/extra", post(write))
            .route("/browser/write", post(|| async { StatusCode::NO_CONTENT }))
            .layer(DefaultBodyLimit::max(2048))
            .with_state(App {
                subject: subject.to_owned(),
                writes,
            });
        let router =
            apply_security_baseline(router, SecurityConfig::default(), Environment::Production)
                .unwrap()
                .layer(Extension(policy))
                .layer(axum::middleware::map_response(
                    |mut response: Response| async move {
                        response
                            .headers_mut()
                            .insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
                        response
                    },
                ));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(async {
                    let _ = stopped.await;
                })
                .await
                .unwrap();
        });
        Self {
            address,
            stop,
            task,
        }
    }
    async fn stop(self) {
        let _ = self.stop.send(());
        self.task.await.unwrap();
    }
    async fn request(
        &self,
        path: &str,
        authorization: &str,
        extra: &str,
        tenant: &str,
    ) -> (u16, String) {
        let body = serde_json::json!({"tenant":tenant,"order":"bounded-order"}).to_string();
        let request = format!(
            "POST {path} HTTP/1.1\r\nHost: {}\r\nUser-Agent: Mozilla/5.0\r\n{authorization}{extra}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            self.address,
            body.len()
        );
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            let mut connection = tokio::net::TcpStream::connect(self.address).await.unwrap();
            connection.write_all(request.as_bytes()).await.unwrap();
            let mut output = Vec::new();
            connection
                .take(16384)
                .read_to_end(&mut output)
                .await
                .unwrap();
            let output = String::from_utf8(output).unwrap();
            let status = output.split_whitespace().nth(1).unwrap().parse().unwrap();
            (status, output)
        })
        .await
        .unwrap()
    }
}
fn authorization(token: &IssuedApiToken) -> String {
    format!("Authorization: Bearer {}\r\n", token.expose_bearer())
}

pub async fn run(url: &str) {
    let clock = Clock::new();
    clock.set(SystemAuthClock.now().unwrap());
    let namespace = unique();
    let service = ApiTokenService::initialize(url, keys(), config(&namespace))
        .await
        .unwrap();
    let peer = ApiTokenService::connect(url, keys(), config(&namespace))
        .await
        .unwrap();
    let (owner, _, _) = account(&service, &clock).await;
    let (foreign, _, _) = account(&service, &clock).await;
    let writer = service
        .issue(&owner, scopes(), label(), 600, &SystemAuthClock)
        .await
        .unwrap();
    let reader = service
        .issue(&owner, read(), label(), 600, &SystemAuthClock)
        .await
        .unwrap();
    let foreign = service
        .issue(&foreign, scopes(), label(), 600, &SystemAuthClock)
        .await
        .unwrap();
    let writes = Arc::new(AtomicUsize::new(0));
    let first = Server::start(&service, owner.subject(), writes.clone()).await;
    let second = Server::start(&peer, owner.subject(), writes.clone()).await;
    let auth = authorization(&writer);
    assert_eq!(
        first.request("/api/orders", "", "", "school-a").await.0,
        401
    );
    assert_eq!(
        first
            .request("/api/orders", &authorization(&reader), "", "school-a")
            .await
            .0,
        401
    );
    assert_eq!(
        first
            .request("/api/orders", &authorization(&foreign), "", "school-a")
            .await
            .0,
        403
    );
    assert_eq!(
        first.request("/api/orders", &auth, "", "school-b").await.0,
        403
    );
    for extra in [
        "Cookie: session=ambient\r\n",
        "Origin: https://evil.example\r\n",
        "Sec-Fetch-Site: same-origin\r\n",
        auth.as_str(),
    ] {
        assert_eq!(
            first
                .request("/api/orders", &auth, extra, "school-a")
                .await
                .0,
            401
        );
    }
    assert_eq!(
        first
            .request("/api/orders/extra", &auth, "", "school-a")
            .await
            .0,
        403
    );
    assert_eq!(
        first
            .request("/browser/write", &auth, "", "school-a")
            .await
            .0,
        403
    );
    let (status, response) = first.request("/api/orders", &auth, "", "school-a").await;
    assert_eq!(status, 204);
    assert!(
        response
            .to_ascii_lowercase()
            .contains("cache-control: no-store")
    );
    assert!(
        response
            .to_ascii_lowercase()
            .contains("x-content-type-options: nosniff")
    );
    assert!(!response.contains(writer.expose_bearer()));
    assert_eq!(
        second.request("/api/orders", &auth, "", "school-a").await.0,
        204
    );
    assert_eq!(writes.load(Ordering::SeqCst), 2);
    let rotated = service
        .rotate(&owner, writer.metadata().id(), 1, 600, &SystemAuthClock)
        .await
        .unwrap();
    assert_eq!(
        second.request("/api/orders", &auth, "", "school-a").await.0,
        401
    );
    assert_eq!(
        second
            .request("/api/orders", &authorization(&rotated), "", "school-a")
            .await
            .0,
        204
    );
    service
        .revoke(&owner, rotated.metadata().id(), &SystemAuthClock)
        .await
        .unwrap();
    assert_eq!(
        first
            .request("/api/orders", &authorization(&rotated), "", "school-a")
            .await
            .0,
        401
    );
    // Storage outage never falls back to a previously verified principal.
    peer.clone().close().await;
    assert_eq!(
        second
            .request("/api/orders", &authorization(&rotated), "", "school-a")
            .await
            .0,
        401
    );
    assert_eq!(writes.load(Ordering::SeqCst), 3);
    first.stop().await;
    second.stop().await;
    service.close().await;
    // The adapter itself does not treat ambient cookies as an alternative proof.
    use rullst_core::security::MachineRequestVerifier;
    let request = Request::builder()
        .uri("/api/orders")
        .header(header::COOKIE, "session=ambient")
        .body(axum::body::Body::empty())
        .unwrap();
    assert!(
        peer.machine_verifier(read())
            .unwrap()
            .verify(request)
            .await
            .is_err()
    );
}
