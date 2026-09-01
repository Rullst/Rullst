use crate::capital::{BillingProvider, WebhookEvent, WebhookVerificationMode, provider};
use crate::error::CapitalError;
#[cfg(feature = "axum")]
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

pub(super) const MAX_WEBHOOK_PAYLOAD_BYTES: usize = 2 * 1024 * 1024;
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

/// Provider and replay state shared by the Axum and Actix middleware adapters.
#[non_exhaustive]
#[derive(Clone)]
pub struct WebhookMiddlewareState {
    replay_store: Arc<InMemoryWebhookReplayStore>,
    allow_mock: bool,
    provider: Option<Arc<dyn BillingProvider>>,
}

impl std::fmt::Debug for WebhookMiddlewareState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WebhookMiddlewareState")
            .field("replay_store", &self.replay_store)
            .field("allow_mock", &self.allow_mock)
            .field(
                "provider",
                &self.provider.as_ref().map(|provider| provider.name()),
            )
            .finish()
    }
}

impl WebhookMiddlewareState {
    /// Creates production-safe state that rejects `mock_*` verifier modes.
    pub fn production(replay_store: Arc<InMemoryWebhookReplayStore>) -> Self {
        Self {
            replay_store,
            allow_mock: false,
            provider: None,
        }
    }

    /// Creates explicitly local-only state that permits signed `mock_*` fixtures.
    pub fn local_mock(replay_store: Arc<InMemoryWebhookReplayStore>) -> Self {
        Self {
            replay_store,
            allow_mock: true,
            provider: None,
        }
    }

    /// Creates production-safe state bound to an explicit provider instance.
    pub fn production_with_provider<P>(
        provider: Arc<P>,
        replay_store: Arc<InMemoryWebhookReplayStore>,
    ) -> Self
    where
        P: BillingProvider + 'static,
    {
        Self {
            replay_store,
            allow_mock: false,
            provider: Some(provider),
        }
    }

    /// Creates local-only state bound to an explicit provider and permits `mock_*` fixtures.
    pub fn local_mock_with_provider<P>(
        provider: Arc<P>,
        replay_store: Arc<InMemoryWebhookReplayStore>,
    ) -> Self
    where
        P: BillingProvider + 'static,
    {
        Self {
            replay_store,
            allow_mock: true,
            provider: Some(provider),
        }
    }

    pub(super) fn resolved_provider(&self) -> Option<&(dyn BillingProvider + '_)> {
        match self.provider.as_deref() {
            Some(provider) => Some(provider),
            None => provider(),
        }
    }
}

pub(super) static DEFAULT_REPLAY_STORE: LazyLock<InMemoryWebhookReplayStore> =
    LazyLock::new(InMemoryWebhookReplayStore::default);

#[cfg(feature = "actix")]
mod actix;
#[cfg(feature = "actix")]
pub use actix::{
    verify_webhook_actix, verify_webhook_actix_mock_local, verify_webhook_actix_with_state,
};

/// Production-safe Axum middleware for signed billing webhooks.
///
/// It rejects empty configuration and `mock_*` verifier modes, validates provider signature and
/// freshness, prevents payload replay, preserves every request part and the original body, and
/// inserts the parsed `WebhookEvent` into request extensions.
#[cfg(feature = "axum")]
pub async fn verify_webhook(req: Request, next: Next) -> Result<Response, StatusCode> {
    verify_webhook_inner(req, next, &DEFAULT_REPLAY_STORE, false, provider()).await
}

/// Explicit local-only middleware variant for deterministic `mock_*` webhook credentials.
///
/// Mock signatures are still mandatory and must equal the configured `mock_*` secret. Do not mount
/// this middleware on a publicly reachable endpoint.
#[cfg(feature = "axum")]
pub async fn verify_webhook_mock_local(req: Request, next: Next) -> Result<Response, StatusCode> {
    verify_webhook_inner(req, next, &DEFAULT_REPLAY_STORE, true, provider()).await
}

/// Configurable middleware entry point for a caller-owned replay store.
#[cfg(feature = "axum")]
pub async fn verify_webhook_with_state(
    State(state): State<WebhookMiddlewareState>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    verify_webhook_inner(
        req,
        next,
        &state.replay_store,
        state.allow_mock,
        state.resolved_provider(),
    )
    .await
}

#[cfg(feature = "axum")]
async fn verify_webhook_inner(
    req: Request,
    next: Next,
    replay_store: &InMemoryWebhookReplayStore,
    allow_mock: bool,
    active_provider: Option<&dyn BillingProvider>,
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

    let active_provider = active_provider.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let event = verify_payload(
        active_provider,
        &body_bytes,
        &header_map,
        replay_store,
        allow_mock,
    )
    .map_err(capital_error_status)?;

    let request = rebuild_request(parts, body_bytes, event);
    Ok(next.run(request).await)
}

#[cfg(feature = "axum")]
fn rebuild_request(mut parts: Parts, body: Bytes, event: WebhookEvent) -> Request {
    parts.extensions.insert(event);
    Request::from_parts(parts, Body::from(body))
}

pub(super) fn verify_payload(
    active_provider: &dyn BillingProvider,
    body: &[u8],
    headers: &HashMap<String, String>,
    replay_store: &InMemoryWebhookReplayStore,
    allow_mock: bool,
) -> Result<WebhookEvent, CapitalError> {
    let mode = active_provider.webhook_verification_mode()?;
    if mode == WebhookVerificationMode::Mock && !allow_mock {
        return Err(CapitalError::MockWebhookNotAllowed(
            active_provider.name().to_string(),
        ));
    }
    let event = active_provider.handle_webhook(body, headers)?;
    replay_store.record_payload(active_provider.name(), body)?;
    Ok(event)
}

pub(super) fn capital_error_status_code(error: &CapitalError) -> u16 {
    match error {
        CapitalError::ConfigurationError(_) | CapitalError::General(_) => 500,
        CapitalError::InvalidSignature(_)
        | CapitalError::AuthenticationFailed(_)
        | CapitalError::StaleWebhook(_) => 401,
        CapitalError::WebhookReplay(_) => 409,
        CapitalError::PayloadParseError(_)
        | CapitalError::InvalidCharge(_)
        | CapitalError::InvalidInvoice(_)
        | CapitalError::InvalidUsage(_) => 400,
        CapitalError::ProviderRequestFailed(_)
        | CapitalError::UnsupportedOperation(_)
        | CapitalError::MockWebhookNotAllowed(_)
        | CapitalError::SubscriptionError(_)
        | CapitalError::Quota(_)
        | CapitalError::FiscalError(_) => 503,
    }
}

#[cfg(feature = "axum")]
fn capital_error_status(error: CapitalError) -> StatusCode {
    StatusCode::from_u16(capital_error_status_code(&error))
        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "axum")]
    use crate::capital::{SubscriptionStatus, WebhookEvent};
    use crate::providers::{LemonSqueezyProvider, StripeProvider};
    #[cfg(feature = "axum")]
    use axum::http::{Method, Version};

    #[cfg(feature = "axum")]
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct ExistingExtension(&'static str);

    #[cfg(feature = "axum")]
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

    #[cfg(feature = "axum")]
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

    #[test]
    fn canonical_verifier_decodes_stripe_and_lemonsqueezy_and_rejects_mock_in_production() {
        let stripe_payload = serde_json::to_vec(&serde_json::json!({
            "type": "customer.subscription.updated",
            "data": { "object": {
                "id": "sub_stripe",
                "customer": "cus_stripe",
                "items": { "data": [{ "price": { "id": "price_stripe" } }] },
                "status": "active"
            }}
        }))
        .expect("fixture serialization must succeed");
        let stripe_headers = HashMap::from([(
            "stripe-signature".to_string(),
            "mock_stripe_signature".to_string(),
        )]);
        let stripe = StripeProvider::new("mock_api", "mock_stripe_signature");
        let stripe_store = InMemoryWebhookReplayStore::default();
        let stripe_event = verify_payload(
            &stripe,
            &stripe_payload,
            &stripe_headers,
            &stripe_store,
            true,
        )
        .expect("local mock signature must produce a normalized event");
        assert_eq!(stripe_event.subscription_id, "sub_stripe");

        let lemon_payload = serde_json::to_vec(&serde_json::json!({
            "meta": { "event_name": "subscription_updated" },
            "data": {
                "id": "sub_lemon",
                "attributes": {
                    "customer_id": 42,
                    "user_email": "lemon@example.com",
                    "variant_id": 7,
                    "status": "active"
                }
            }
        }))
        .expect("fixture serialization must succeed");
        let lemon_headers = HashMap::from([(
            "x-signature".to_string(),
            "mock_lemon_signature".to_string(),
        )]);
        let lemon = LemonSqueezyProvider::new("mock_api", "mock_lemon_signature");
        let lemon_store = InMemoryWebhookReplayStore::default();
        let lemon_event =
            verify_payload(&lemon, &lemon_payload, &lemon_headers, &lemon_store, true)
                .expect("local mock signature must produce a normalized event");
        assert_eq!(lemon_event.subscription_id, "sub_lemon");

        assert!(matches!(
            verify_payload(
                &stripe,
                &stripe_payload,
                &stripe_headers,
                &InMemoryWebhookReplayStore::default(),
                false,
            ),
            Err(CapitalError::MockWebhookNotAllowed(provider)) if provider == "stripe"
        ));
    }
}

#[cfg(all(test, feature = "axum"))]
#[path = "webhook_axum_tests.rs"]
mod axum_tests;
