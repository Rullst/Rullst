#![allow(dead_code)] // Shared fixtures are exercised by separate feature targets.
pub mod tus;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
};
use rullst_media::{bunny::*, *};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicI64, Ordering},
    },
};

pub const NOW: i64 = 1_800_000_000;
pub const API: &str = "fixture_api_key_123456789";
pub const WEBHOOK: &str = "fixture_webhook_key_123456789";
pub const EMBED: &str = "fixture_embed_key_123456789";
pub const CDN: &str = "fixture_cdn_key_123456789";
pub const VIDEO: &str = "12345678-1234-4234-8234-123456789abc";

pub fn config(mode: &str) -> BunnyConfig {
    let keys = if mode == "offline" {
        ["mock_api", "mock_webhook", "mock_embed", "mock_cdn"]
    } else {
        [API, WEBHOOK, EMBED, CDN]
    };
    BunnyConfig::new(
        LibraryId::new(7).unwrap(),
        Reference::new("acceptance").unwrap(),
        "test-library.b-cdn.net",
        BunnyCredentials::new(keys[0], keys[1], keys[2], keys[3]).unwrap(),
        PrivateDelivery::configured(true, true, true).unwrap(),
    )
    .unwrap()
}
pub fn reference(value: &str) -> Reference {
    Reference::new(value).unwrap()
}
pub fn scope() -> Scope {
    Scope::new("school-a", "rust-course").unwrap()
}
pub fn metadata() -> Metadata {
    Metadata::new("Ownership in Rust", "An accessible lesson transcript.").unwrap()
}

#[derive(Clone)]
pub struct TestClock(pub Arc<AtomicI64>);
impl TestClock {
    pub fn new() -> Self {
        Self(Arc::new(AtomicI64::new(NOW)))
    }
    pub fn set(&self, now: i64) {
        self.0.store(now, Ordering::SeqCst);
    }
    pub fn advance(&self, seconds: i64) {
        self.0.fetch_add(seconds, Ordering::SeqCst);
    }
}
impl Clock for TestClock {
    fn now(&self) -> Result<i64, MediaError> {
        Ok(self.0.load(Ordering::SeqCst))
    }
}

pub struct Auth {
    pub until: AtomicI64,
    pub revoked: AtomicBool,
    pub scope: Scope,
}
impl Auth {
    pub fn new() -> Self {
        Self {
            until: AtomicI64::new(NOW + 10_000),
            revoked: AtomicBool::new(false),
            scope: scope(),
        }
    }
}
impl Authorization for Auth {
    async fn check(
        &self,
        actor: &Reference,
        scope: &Scope,
        action: Action,
    ) -> Result<Permission, MediaError> {
        if self.revoked.load(Ordering::SeqCst)
            || scope != &self.scope
            || (actor.as_str() != "teacher"
                && (actor.as_str() != "learner" || action == Action::Manage))
        {
            return Err(MediaError::Denied);
        }
        Permission::until(self.until.load(Ordering::SeqCst))
    }
}

#[derive(Default)]
pub struct Remote {
    pub next_video: u64,
    pub uploads: BTreeMap<String, tus::Upload>,
    pub app_origin: Option<String>,
    pub videos: BTreeMap<String, Value>,
    pub calls: Vec<String>,
    pub lost_create: bool,
    pub fail_reads: bool,
    pub fail_deletes: bool,
    pub wrong_identity: bool,
    pub redirect: bool,
    pub bad_success: bool,
    /// Simulates a top-level Description that differs from the written
    /// `description` meta tag (for example a Smart Generate rewrite).
    pub generated_description: bool,
    /// Definitively refuses create requests (400) without creating a video.
    pub reject_create: bool,
    /// Acknowledges updates without applying them.
    pub ignore_updates: bool,
    /// Decoded query of each list request.
    pub queries: Vec<BTreeMap<String, String>>,
    /// Reports this page size instead of the requested one.
    pub list_page_size: Option<usize>,
    /// Returns one item more than the requested page size.
    pub list_extra_item: bool,
    /// Reports this total instead of the number of matches.
    pub list_total: Option<usize>,
    pub gate: Option<Arc<(tokio::sync::Notify, tokio::sync::Notify)>>,
}
pub struct Fixture {
    pub origin: String,
    pub remote: Arc<Mutex<Remote>>,
    server: tokio::task::JoinHandle<()>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.server.abort();
    }
}
impl Fixture {
    pub async fn new() -> Self {
        let remote = Arc::new(Mutex::new(Remote::default()));
        let router = Router::new()
            .route("/library/{library}/videos", get(list).post(create))
            .route(
                "/library/{library}/videos/{video}",
                get(read).post(update).delete(delete),
            )
            .layer(axum::extract::DefaultBodyLimit::max(32768));
        let router = tus::routes(router).with_state(remote.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        Self {
            origin,
            remote,
            server,
        }
    }
    pub fn provider(&self) -> BunnyStream {
        BunnyStream::for_protocol_tests(config("fixture"), &self.origin).unwrap()
    }
    pub fn ready(&self, id: &VideoId) {
        self.remote
            .lock()
            .unwrap()
            .videos
            .get_mut(id.as_str())
            .unwrap()["status"] = json!(4);
    }
}
fn authorized(headers: &HeaderMap, library: i64) -> bool {
    library == 7 && headers.get("AccessKey").and_then(|v| v.to_str().ok()) == Some(API)
}
async fn create(
    State(state): State<Arc<Mutex<Remote>>>,
    Path(library): Path<i64>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    if !authorized(&headers, library) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let mut remote = state.lock().unwrap();
    remote.calls.push("create".into());
    if remote.reject_create {
        return StatusCode::BAD_REQUEST.into_response();
    }
    remote.next_video += 1;
    let id = format!("12345678-1234-4234-8234-{:012x}", remote.next_video);
    let video = json!({"videoLibraryId":7,"guid":id,"title":body["title"],"description":"","status":0,"length":12,"hasMP4Fallback":false,"availableResolutions":"720p",
        "dateUploaded":format!("2026-11-01T12:00:{:02}.123", remote.next_video % 60),"thumbnailUrl":format!("https://test-library.b-cdn.net/{id}/thumbnail.jpg")});
    remote.videos.insert(id, video.clone());
    if remote.lost_create {
        return StatusCode::BAD_GATEWAY.into_response();
    }
    Json(video).into_response()
}
async fn list(
    State(state): State<Arc<Mutex<Remote>>>,
    Path(library): Path<i64>,
    Query(query): Query<BTreeMap<String, String>>,
    headers: HeaderMap,
) -> Response {
    if !authorized(&headers, library) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let mut remote = state.lock().unwrap();
    remote.calls.push("list".into());
    remote.queries.push(query.clone());
    let search = query.get("search").map(|s| s.to_lowercase());
    let mut items: Vec<_> = remote
        .videos
        .values()
        .filter(|v| {
            v["title"].as_str().is_some_and(|s| {
                search
                    .as_ref()
                    .is_none_or(|term| s.to_lowercase().contains(term))
            })
        })
        .cloned()
        .collect();
    if query.get("orderBy").map(String::as_str) == Some("title") {
        items.sort_by(|a, b| a["title"].as_str().cmp(&b["title"].as_str()));
    }
    let page: usize = query.get("page").map_or(1, |p| p.parse().unwrap());
    let size: usize = query
        .get("itemsPerPage")
        .map_or(100, |p| p.parse().unwrap());
    let total = remote.list_total.unwrap_or(items.len());
    let take = size + usize::from(remote.list_extra_item);
    let mut items: Vec<_> = items
        .into_iter()
        .skip((page - 1) * size)
        .take(take)
        .collect();
    if remote.wrong_identity {
        for item in &mut items {
            item["videoLibraryId"] = json!(8);
        }
    }
    let size = remote.list_page_size.unwrap_or(size);
    Json(json!({"items":items,"totalItems":total,"currentPage":page,"itemsPerPage":size}))
        .into_response()
}
async fn read(
    State(state): State<Arc<Mutex<Remote>>>,
    Path((library, video)): Path<(i64, String)>,
    headers: HeaderMap,
) -> Response {
    if !authorized(&headers, library) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let gate = state.lock().unwrap().gate.clone();
    if let Some(gate) = gate {
        gate.0.notify_one();
        gate.1.notified().await;
    }
    let mut remote = state.lock().unwrap();
    remote.calls.push("read".into());
    if remote.fail_reads {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    if remote.redirect {
        return (
            StatusCode::FOUND,
            [("location", "http://127.0.0.1:9/secrets")],
        )
            .into_response();
    }
    match remote.videos.get(&video) {
        Some(value) => {
            let mut value = value.clone();
            if remote.wrong_identity {
                value["videoLibraryId"] = json!(8);
            }
            Json(value).into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
async fn update(
    State(state): State<Arc<Mutex<Remote>>>,
    Path((library, video)): Path<(i64, String)>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    if !authorized(&headers, library) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let mut remote = state.lock().unwrap();
    remote.calls.push("update".into());
    let generated = remote.generated_description;
    let ignored = remote.ignore_updates;
    let Some(value) = remote.videos.get_mut(&video) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if ignored {
        return Json(json!({"success":true,"statusCode":200})).into_response();
    }
    value["title"] = body["title"].clone();
    value["metaTags"] = body["metaTags"].clone();
    // API 1.6.6: setting the `description` meta tag also updates Description.
    if let Some(tag) = body["metaTags"]
        .as_array()
        .unwrap()
        .iter()
        .find(|tag| tag["property"] == "description")
    {
        value["description"] = tag["value"].clone();
    }
    if generated {
        value["description"] = json!("Provider-generated summary");
    }
    Json(json!({"success":!remote.bad_success,"statusCode":200})).into_response()
}
async fn delete(
    State(state): State<Arc<Mutex<Remote>>>,
    Path((library, video)): Path<(i64, String)>,
    headers: HeaderMap,
) -> Response {
    if !authorized(&headers, library) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let mut remote = state.lock().unwrap();
    remote.calls.push("delete".into());
    if remote.fail_deletes {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    if remote.videos.remove(&video).is_none() {
        return StatusCode::NOT_FOUND.into_response();
    }
    Json(json!({"success":true,"statusCode":200})).into_response()
}

pub fn notification(provider: &BunnyStream, video: &VideoId, status: u8) -> VerifiedNotification {
    let body =
        serde_json::to_vec(&json!({"VideoLibraryId":7,"VideoGuid":video.as_str(),"Status":status}))
            .unwrap();
    let key = ring::hmac::Key::new(ring::hmac::HMAC_SHA256, WEBHOOK.as_bytes());
    let signature: String = ring::hmac::sign(&key, &body)
        .as_ref()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    provider
        .verify_notification(
            WebhookHeaders {
                version: "v1",
                algorithm: "hmac-sha256",
                signature: &signature,
            },
            &body,
        )
        .unwrap()
}
