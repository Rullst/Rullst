#![cfg(all(feature = "bunny", feature = "sqlite"))]
mod support;
use axum::{
    Extension, Json, Router,
    body::{Body, to_bytes},
    extract::{Request, State},
    http::{HeaderMap, Method, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use rullst_core::{
    config::{Environment, SecurityConfig},
    security::{
        CsrfToken, MachineEndpoint, MachineEndpointError, MachineEndpointPolicy,
        MachineRequestVerifier, apply_security_baseline_with_machine_endpoints,
    },
};
use rullst_media::{bunny::BunnyStream, sqlite::*, *};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{io::Write, sync::Arc};
use subtle::ConstantTimeEq;
use support::*;

struct App {
    media: MediaService<BunnyStream, TestClock>,
    auth: Auth,
    teacher_cookie: String,
    learner_cookie: String,
}
impl App {
    fn actor(&self, headers: &HeaderMap) -> Result<Reference, StatusCode> {
        let cookie = headers
            .get("cookie")
            .and_then(|v| v.to_str().ok())
            .ok_or(StatusCode::UNAUTHORIZED)?;
        let tokens: Vec<_> = cookie
            .split(';')
            .filter_map(|s| s.trim().strip_prefix("media_fixture="))
            .collect();
        let [token] = tokens.as_slice() else {
            return Err(StatusCode::UNAUTHORIZED);
        };
        if token
            .as_bytes()
            .ct_eq(self.teacher_cookie.as_bytes())
            .unwrap_u8()
            == 1
        {
            Ok(reference("teacher"))
        } else if token
            .as_bytes()
            .ct_eq(self.learner_cookie.as_bytes())
            .unwrap_u8()
            == 1
        {
            Ok(reference("learner"))
        } else {
            Err(StatusCode::UNAUTHORIZED)
        }
    }
    async fn command(
        &self,
        headers: HeaderMap,
        body: Command,
        action: &str,
    ) -> Result<Json<Value>, StatusCode> {
        let actor = self.actor(&headers)?;
        let id = reference("lesson-1");
        let scope = body.scope;
        let result=async {
            match action {
                "create"=>serde_json::to_value(self.media.create(&self.auth,&actor,&scope,&id,metadata()).await?).map_err(|_|MediaError::Protocol),
                "upload"=>Ok(json!({"upload":self.media.upload(&self.auth,&actor,&scope,&id,300).await?})),
                "playback"=>Ok(json!({"playback":self.media.playback(&self.auth,&actor,&scope,&id,300,PlaybackKind::Embed).await?})),
                "publish"|"withdraw"|"delete"=>{
                    let current=self.media.get(&self.auth,&actor,&scope,&id).await?;
                    let asset=match action {
                        "publish"=>self.media.publish(&self.auth,&actor,&scope,&id,current.revision).await?,
                        "withdraw"=>self.media.withdraw(&self.auth,&actor,&scope,&id,current.revision).await?,
                        _=>self.media.delete(&self.auth,&actor,&scope,&id,current.revision).await?,
                    };
                    serde_json::to_value(asset).map_err(|_|MediaError::Protocol)
                }
                _=>Err(MediaError::Unsupported),
            }
        }.await;
        result.map(Json).map_err(|error| match error {
            MediaError::Denied => StatusCode::FORBIDDEN,
            MediaError::NotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::CONFLICT,
        })
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Command {
    scope: Scope,
}
macro_rules! command {
    ($name:ident) => {
        async fn $name(
            State(app): State<Arc<App>>,
            headers: HeaderMap,
            Json(body): Json<Command>,
        ) -> Result<Json<Value>, StatusCode> {
            app.command(headers, body, stringify!($name)).await
        }
    };
}
command!(create);
command!(upload);
command!(publish);
command!(playback);
command!(withdraw);
command!(delete);
async fn page(Extension(csrf): Extension<CsrfToken>) -> Html<String> {
    Html(format!(
        r#"<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width"><meta name="csrf" content="{}"><title>Private course video</title><main><h1>Private course video</h1><p>Select the lesson file, then upload and publish it.</p><label for="file">Video file</label><input id="file" type="file" accept="video/webm,video/mp4"><button id="create">Create lesson</button><button id="upload">Upload video</button><button id="pause">Pause upload</button><button id="cancel">Cancel upload</button><button id="publish">Publish lesson</button><button id="playback">Play or renew access</button><button id="withdraw">Withdraw lesson</button><button id="delete">Delete video</button><output id="status" role="status" aria-live="polite">ready</output><section id="player" aria-label="Lesson player"></section><p id="transcript">Transcript: Rust lesson.</p><noscript>Enable JavaScript to upload the video.</noscript></main><script type="module" src="/static/course.mjs"></script></html>"#,
        rullst_core::html::escape_str(csrf.as_str())
    ))
}
struct Verify(Arc<App>);
#[async_trait::async_trait]
impl MachineRequestVerifier for Verify {
    async fn verify(&self, request: Request) -> Result<Request, MachineEndpointError> {
        let (mut parts, body) = request.into_parts();
        let bytes = to_bytes(body, 4096)
            .await
            .map_err(|_| MachineEndpointError::Unauthorized)?;
        let header = |name| {
            if parts.headers.get_all(name).iter().count() != 1 {
                return Err(MachineEndpointError::Unauthorized);
            }
            parts
                .headers
                .get(name)
                .and_then(|v| v.to_str().ok())
                .ok_or(MachineEndpointError::Unauthorized)
        };
        let event = self
            .0
            .media
            .provider()
            .verify_notification(
                WebhookHeaders {
                    version: header("x-bunnystream-signature-version")?,
                    algorithm: header("x-bunnystream-signature-algorithm")?,
                    signature: header("x-bunnystream-signature")?,
                },
                &bytes,
            )
            .map_err(|_| MachineEndpointError::Unauthorized)?;
        parts.extensions.insert(event);
        Ok(Request::from_parts(parts, Body::from(bytes)))
    }
}
async fn events(
    State(app): State<Arc<App>>,
    Extension(event): Extension<VerifiedNotification>,
) -> Response {
    match app.media.notification(&event).await {
        Ok(_) => StatusCode::NO_CONTENT,
        Err(_) => StatusCode::CONFLICT,
    }
    .into_response()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn authenticated_browser_upload_and_private_playback() {
    // Node fault contracts also run in the browser script. Native matrix retains
    // the ordinary Rust protocol/lifecycle tests on every supported OS.
    if std::env::var("RULLST_MEDIA_BROWSER_TESTS").as_deref() != Ok("1") {
        return;
    }
    let fixture = Fixture::new().await;
    let directory = tempfile::tempdir().unwrap();
    let provider = fixture.provider();
    let clock = TestClock::new();
    clock.set(SystemClock.now().unwrap());
    let store = SqliteMedia::initialize(
        directory.path().join("media.sqlite"),
        StoreConfig::testing(provider.binding(), 32).unwrap(),
        clock,
    )
    .await
    .unwrap();
    let app = Arc::new(App {
        media: MediaService::new(provider, store).unwrap(),
        auth: Auth::new(),
        teacher_cookie: rullst_core::security::generate_csrf_token(),
        learner_cookie: rullst_core::security::generate_csrf_token(),
    });
    let router = Router::new()
        .route("/", get(page))
        .route("/video/create", post(create))
        .route("/video/upload", post(upload))
        .route("/video/publish", post(publish))
        .route("/video/playback", post(playback))
        .route("/video/withdraw", post(withdraw))
        .route("/video/delete", post(delete))
        .route("/bunny/events", post(events))
        .route(
            "/static/course.mjs",
            get(|| async {
                (
                    [("content-type", "text/javascript")],
                    include_str!("fixtures/course.mjs"),
                )
            }),
        )
        .route(
            "/static/upload.mjs",
            get(|| async { ([("content-type", "text/javascript")], BUNNY_UPLOAD_MODULE) }),
        )
        .layer(axum::extract::DefaultBodyLimit::max(16384))
        .with_state(app.clone());
    let mut security = SecurityConfig::default();
    security.csp = format!(
        "default-src 'none'; script-src 'self'; connect-src 'self' {}; frame-src {}; media-src 'self'; base-uri 'none'; frame-ancestors 'none'",
        fixture.origin, fixture.origin
    );
    security.coep = "unsafe-none".into();
    let policy = MachineEndpointPolicy::new(vec![
        MachineEndpoint::signed_webhook(Method::POST, "/bunny/events", Verify(app.clone()))
            .unwrap(),
    ])
    .unwrap();
    let router = apply_security_baseline_with_machine_endpoints(
        router,
        security,
        Environment::Production,
        policy,
    )
    .unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async {
                let _ = stopped.await;
            })
            .await
            .unwrap()
    });
    let input = json!({"origin":origin,"provider":fixture.origin,"teacher":app.teacher_cookie,"learner":app.learner_cookie,"file":concat!(env!("CARGO_MANIFEST_DIR"),"/tests/fixtures/lesson.webm")});
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join(".github/media-browser-smoke.mjs");
    let result = tokio::task::spawn_blocking(move || {
        let mut child = std::process::Command::new("node")
            .arg(script)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.to_string().as_bytes())
            .unwrap();
        child.wait_with_output().unwrap()
    })
    .await
    .unwrap();
    let _ = stop.send(());
    server.await.unwrap();
    app.media.close().await;
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    let state = fixture.remote.lock().unwrap();
    assert!(state.videos.is_empty());
    let uploaded = state.uploads.values().next().unwrap();
    assert_eq!(uploaded.bytes, include_bytes!("fixtures/lesson.webm"));
    assert_eq!(state.calls.iter().filter(|c| *c == "tus-post").count(), 1);
    println!("{}", String::from_utf8_lossy(&result.stdout));
}
