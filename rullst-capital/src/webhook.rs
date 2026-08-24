use crate::capital::{WebhookVerificationMode, provider};
use crate::error::CapitalError;
use axum::{
    body::{Body, Bytes},
    extract::{Request, State},
    http::{StatusCode, request::Parts},
    middleware::Next,
    response::Response,
};
use ring::digest::{SHA256, digest};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};

const MAX_WEBHOOK_PAYLOAD_BYTES: usize = 2 * 1024 * 1024;
const DEFAULT_REPLAY_CAPACITY: usize = 10_000;
const DEFAULT_REPLAY_TTL: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Debug)]
struct ReplayEntry {
    key: String,
    accepted_at: Instant,
}

/// Bounded in-memory webhook replay store with time-based eviction.
///
/// Applications requiring replay protection across multiple processes should persist the same
/// payload key in a shared datastore before executing side effects.
#[derive(Debug)]
pub struct InMemoryWebhookReplayStore {
    entries: Mutex<VecDeque<ReplayEntry>>,
    max_entries: usize,
    ttl: Duration,
}

impl InMemoryWebhookReplayStore {
    /// Creates a replay store. Capacity and TTL must both be greater than zero.
    pub fn new(max_entries: usize, ttl: Duration) -> Result<Self, CapitalError> {
        if max_entries == 0 {
            return Err(CapitalError::ConfigurationError(
                "Webhook replay capacity must be greater than zero".to_string(),
            ));
        }
        if ttl.is_zero() {
            return Err(CapitalError::ConfigurationError(
                "Webhook replay TTL must be greater than zero".to_string(),
            ));
        }

        Ok(Self {
            entries: Mutex::new(VecDeque::with_capacity(max_entries.min(1_024))),
            max_entries,
            ttl,
        })
    }

    /// Atomically rejects a replay or records a newly verified payload key.
    pub fn check_and_record(&self, key: impl Into<String>) -> Result<(), CapitalError> {
        self.check_and_record_at(key.into(), Instant::now())
    }

    fn check_and_record_at(&self, key: String, now: Instant) -> Result<(), CapitalError> {
        let mut entries = self.entries.lock().map_err(|_| {
            CapitalError::General("Webhook replay store lock was poisoned".to_string())
        })?;

        while entries.front().is_some_and(|entry| {
            now.checked_duration_since(entry.accepted_at)
                .is_some_and(|age| age >= self.ttl)
        }) {
            let _ = entries.pop_front();
        }

        if entries.iter().any(|entry| entry.key == key) {
            return Err(CapitalError::WebhookReplay(key));
        }

        if entries.len() == self.max_entries {
            let _ = entries.pop_front();
        }
        entries.push_back(ReplayEntry {
            key,
            accepted_at: now,
        });
        Ok(())
    }

    /// Computes and records a stable provider-scoped SHA-256 payload key.
    pub fn record_payload(&self, provider_name: &str, payload: &[u8]) -> Result<(), CapitalError> {
        let mut scoped_payload = Vec::with_capacity(provider_name.len() + payload.len() + 1);
        scoped_payload.extend_from_slice(provider_name.as_bytes());
        scoped_payload.push(0);
        scoped_payload.extend_from_slice(payload);
        let payload_hash = digest(&SHA256, &scoped_payload);
        self.check_and_record(hex::encode(payload_hash.as_ref()))
    }
}

impl Default for InMemoryWebhookReplayStore {
    fn default() -> Self {
        Self {
            entries: Mutex::new(VecDeque::with_capacity(1_024)),
            max_entries: DEFAULT_REPLAY_CAPACITY,
            ttl: DEFAULT_REPLAY_TTL,
        }
    }
}

/// Configuration state for `axum::middleware::from_fn_with_state`.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct WebhookMiddlewareState {
    replay_store: Arc<InMemoryWebhookReplayStore>,
    allow_mock: bool,
}

impl WebhookMiddlewareState {
    /// Creates production-safe state that rejects `mock_*` verifier modes.
    pub fn production(replay_store: Arc<InMemoryWebhookReplayStore>) -> Self {
        Self {
            replay_store,
            allow_mock: false,
        }
    }

    /// Creates explicitly local-only state that permits signed `mock_*` fixtures.
    pub fn local_mock(replay_store: Arc<InMemoryWebhookReplayStore>) -> Self {
        Self {
            replay_store,
            allow_mock: true,
        }
    }
}

static DEFAULT_REPLAY_STORE: LazyLock<InMemoryWebhookReplayStore> =
    LazyLock::new(InMemoryWebhookReplayStore::default);

/// Production-safe Axum middleware for signed billing webhooks.
///
/// It rejects empty configuration and `mock_*` verifier modes, validates provider signature and
/// freshness, prevents payload replay, preserves every request part and the original body, and
/// inserts the parsed `WebhookEvent` into request extensions.
pub async fn verify_webhook(req: Request, next: Next) -> Result<Response, StatusCode> {
    verify_webhook_inner(req, next, &DEFAULT_REPLAY_STORE, false).await
}

/// Explicit local-only middleware variant for deterministic `mock_*` webhook credentials.
///
/// Mock signatures are still mandatory and must equal the configured `mock_*` secret. Do not mount
/// this middleware on a publicly reachable endpoint.
pub async fn verify_webhook_mock_local(req: Request, next: Next) -> Result<Response, StatusCode> {
    verify_webhook_inner(req, next, &DEFAULT_REPLAY_STORE, true).await
}

/// Configurable middleware entry point for a caller-owned replay store.
pub async fn verify_webhook_with_state(
    State(state): State<WebhookMiddlewareState>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    verify_webhook_inner(req, next, &state.replay_store, state.allow_mock).await
}

async fn verify_webhook_inner(
    req: Request,
    next: Next,
    replay_store: &InMemoryWebhookReplayStore,
    allow_mock: bool,
) -> Result<Response, StatusCode> {
    let (parts, body) = req.into_parts();
    let body_bytes = axum::body::to_bytes(body, MAX_WEBHOOK_PAYLOAD_BYTES)
        .await
        .map_err(|_| StatusCode::PAYLOAD_TOO_LARGE)?;

    let mut header_map = HashMap::new();
    for (name, value) in &parts.headers {
        if let Ok(value) = value.to_str() {
            header_map.insert(name.as_str().to_lowercase(), value.to_string());
        }
    }

    let active_provider = provider().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let mode = active_provider
        .webhook_verification_mode()
        .map_err(capital_error_status)?;
    if mode == WebhookVerificationMode::Mock && !allow_mock {
        return Err(capital_error_status(CapitalError::MockWebhookNotAllowed(
            active_provider.name().to_string(),
        )));
    }

    let event = active_provider
        .handle_webhook(&body_bytes, &header_map)
        .map_err(capital_error_status)?;
    replay_store
        .record_payload(active_provider.name(), &body_bytes)
        .map_err(capital_error_status)?;

    let request = rebuild_request(parts, body_bytes, event);
    Ok(next.run(request).await)
}

fn rebuild_request(mut parts: Parts, body: Bytes, event: crate::capital::WebhookEvent) -> Request {
    parts.extensions.insert(event);
    Request::from_parts(parts, Body::from(body))
}

fn capital_error_status(error: CapitalError) -> StatusCode {
    match error {
        CapitalError::ConfigurationError(_) | CapitalError::General(_) => {
            StatusCode::INTERNAL_SERVER_ERROR
        }
        CapitalError::InvalidSignature(_)
        | CapitalError::AuthenticationFailed(_)
        | CapitalError::StaleWebhook(_) => StatusCode::UNAUTHORIZED,
        CapitalError::WebhookReplay(_) => StatusCode::CONFLICT,
        CapitalError::PayloadParseError(_) => StatusCode::BAD_REQUEST,
        CapitalError::ProviderRequestFailed(_)
        | CapitalError::UnsupportedOperation(_)
        | CapitalError::MockWebhookNotAllowed(_)
        | CapitalError::SubscriptionError(_)
        | CapitalError::FiscalError(_) => StatusCode::SERVICE_UNAVAILABLE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capital::{SubscriptionStatus, WebhookEvent};
    use axum::http::{Method, Version};

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct ExistingExtension(&'static str);

    fn event() -> WebhookEvent {
        WebhookEvent {
            subscription_id: "sub_1".to_string(),
            customer_id: "cus_1".to_string(),
            customer_email: "customer@example.com".to_string(),
            plan_id: "plan_1".to_string(),
            status: SubscriptionStatus::Active,
            ends_at: None,
        }
    }

    #[tokio::test]
    async fn reconstructed_request_preserves_parts_extensions_and_body() {
        let mut request = Request::builder()
            .method(Method::PATCH)
            .uri("/billing/webhook?tenant=acme")
            .version(Version::HTTP_2)
            .header("x-trace-id", "trace-123")
            .body(Body::from("original-body"))
            .unwrap();
        request
            .extensions_mut()
            .insert(ExistingExtension("preserved"));

        let (parts, body) = request.into_parts();
        let body = axum::body::to_bytes(body, 1024).await.unwrap();
        let request = rebuild_request(parts, body, event());

        assert_eq!(request.method(), Method::PATCH);
        assert_eq!(request.uri(), "/billing/webhook?tenant=acme");
        assert_eq!(request.version(), Version::HTTP_2);
        assert_eq!(request.headers()["x-trace-id"], "trace-123");
        assert_eq!(
            request.extensions().get::<ExistingExtension>(),
            Some(&ExistingExtension("preserved"))
        );
        assert!(request.extensions().get::<WebhookEvent>().is_some());
        let body = axum::body::to_bytes(request.into_body(), 1024)
            .await
            .unwrap();
        assert_eq!(body, "original-body");
    }

    #[test]
    fn replay_store_rejects_duplicates_and_expires_entries() {
        let store = InMemoryWebhookReplayStore::new(2, Duration::from_secs(10)).unwrap();
        let start = Instant::now();

        assert!(
            store
                .check_and_record_at("first".to_string(), start)
                .is_ok()
        );
        assert!(matches!(
            store.check_and_record_at("first".to_string(), start + Duration::from_secs(1)),
            Err(CapitalError::WebhookReplay(_))
        ));
        assert!(
            store
                .check_and_record_at("first".to_string(), start + Duration::from_secs(10))
                .is_ok()
        );
    }

    #[test]
    fn replay_store_is_bounded_and_provider_scoped() {
        let store = InMemoryWebhookReplayStore::new(1, Duration::from_secs(60)).unwrap();
        assert!(store.record_payload("stripe", b"same body").is_ok());
        assert!(store.record_payload("paddle", b"same body").is_ok());
        assert!(store.record_payload("stripe", b"same body").is_ok());
    }

    #[test]
    fn replay_store_rejects_invalid_configuration() {
        assert!(InMemoryWebhookReplayStore::new(0, Duration::from_secs(1)).is_err());
        assert!(InMemoryWebhookReplayStore::new(1, Duration::ZERO).is_err());
    }
}
