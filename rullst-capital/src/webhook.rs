#[cfg(any(feature = "axum", feature = "actix"))]
use crate::capital::provider;
#[cfg(any(feature = "axum", feature = "actix", test))]
use crate::capital::{BillingProvider, WebhookEvent, WebhookVerificationMode};
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
#[cfg(any(feature = "axum", feature = "actix", test))]
use std::collections::HashMap;
use std::collections::VecDeque;
#[cfg(any(feature = "axum", feature = "actix"))]
use std::sync::LazyLock;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[cfg(feature = "webhook-sql")]
mod sql;
#[cfg(feature = "webhook-sql")]
pub use sql::{SqlWebhookBackend, SqlWebhookReplayStore};

pub(super) const MAX_WEBHOOK_PAYLOAD_BYTES: usize = 2 * 1024 * 1024;
const DEFAULT_REPLAY_CAPACITY: usize = 10_000;
const DEFAULT_REPLAY_TTL: Duration = Duration::from_secs(24 * 60 * 60);
const MAX_REPLAY_CAPACITY: usize = 1_000_000;
const MAX_REPLAY_TTL: Duration = Duration::from_secs(30 * 24 * 60 * 60);

#[derive(Debug)]
struct ReplayEntry {
    key: String,
    accepted_at: Instant,
}

/// Bounded in-memory webhook replay store with time-based eviction.
///
/// Multi-process applications can select `SqlWebhookReplayStore` through the
/// `webhook-sql` feature. This process-local variant fails closed at capacity
/// rather than evicting an active replay proof.
#[derive(Debug)]
pub struct InMemoryWebhookReplayStore {
    entries: Mutex<VecDeque<ReplayEntry>>,
    max_entries: usize,
    ttl: Duration,
}

impl InMemoryWebhookReplayStore {
    /// Creates a replay store. Capacity and TTL must both be greater than zero.
    pub fn new(max_entries: usize, ttl: Duration) -> Result<Self, CapitalError> {
        if max_entries == 0 || max_entries > MAX_REPLAY_CAPACITY {
            return Err(CapitalError::ConfigurationError(
                "Webhook replay capacity must be between 1 and 1000000".to_string(),
            ));
        }
        if ttl.is_zero() || ttl > MAX_REPLAY_TTL {
            return Err(CapitalError::ConfigurationError(
                "Webhook replay TTL must be between 1 second and 30 days".to_string(),
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
        let key = key.into();
        let valid = !key.is_empty()
            && key.len() <= 128
            && key
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'));
        if !valid {
            return Err(CapitalError::ConfigurationError(
                "Webhook replay key must use 1-128 ASCII letters, digits, dots, hyphens, or underscores"
                    .to_string(),
            ));
        }
        self.check_and_record_at(key, Instant::now())
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

        if entries.len() >= self.max_entries {
            return Err(CapitalError::WebhookReplayStoreFull);
        }
        entries.push_back(ReplayEntry {
            key,
            accepted_at: now,
        });
        Ok(())
    }

    /// Computes and records a stable provider-scoped SHA-256 payload key.
    pub fn record_payload(&self, provider_name: &str, payload: &[u8]) -> Result<(), CapitalError> {
        validate_replay_provider(provider_name)?;
        if payload.is_empty() || payload.len() > MAX_WEBHOOK_PAYLOAD_BYTES {
            return Err(CapitalError::ConfigurationError(
                "Webhook replay payload must contain 1 byte through the configured body limit"
                    .to_string(),
            ));
        }
        self.check_and_record(payload_key(provider_name, payload))
    }

    /// Records a provider-scoped semantic event identifier selected after signature verification.
    pub fn record_event_key(
        &self,
        provider_name: &str,
        event_key: &str,
    ) -> Result<(), CapitalError> {
        validate_replay_provider(provider_name)?;
        validate_replay_event_key(event_key)?;
        self.check_and_record(event_key_hash(provider_name, event_key))
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

/// Explicit replay backend used by framework middleware.
#[derive(Clone)]
#[non_exhaustive]
pub enum WebhookReplayBackend {
    /// Process-local bounded replay protection.
    Memory(Arc<InMemoryWebhookReplayStore>),
    /// Durable relational replay protection shared by multiple processes.
    #[cfg(feature = "webhook-sql")]
    Sql(Arc<SqlWebhookReplayStore>),
}

impl std::fmt::Debug for WebhookReplayBackend {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Memory(store) => formatter.debug_tuple("Memory").field(store).finish(),
            #[cfg(feature = "webhook-sql")]
            Self::Sql(store) => formatter.debug_tuple("Sql").field(store).finish(),
        }
    }
}

impl From<Arc<InMemoryWebhookReplayStore>> for WebhookReplayBackend {
    fn from(store: Arc<InMemoryWebhookReplayStore>) -> Self {
        Self::Memory(store)
    }
}

#[cfg(feature = "webhook-sql")]
impl From<Arc<SqlWebhookReplayStore>> for WebhookReplayBackend {
    fn from(store: Arc<SqlWebhookReplayStore>) -> Self {
        Self::Sql(store)
    }
}

impl WebhookReplayBackend {
    #[cfg(any(feature = "axum", feature = "actix", test))]
    async fn record_payload(&self, provider: &str, payload: &[u8]) -> Result<(), CapitalError> {
        match self {
            Self::Memory(store) => store.record_payload(provider, payload),
            #[cfg(feature = "webhook-sql")]
            Self::Sql(store) => store.check_and_record_payload(provider, payload).await,
        }
    }
}

/// Provider and replay state shared by the Axum and Actix middleware adapters.
#[cfg(any(feature = "axum", feature = "actix"))]
#[non_exhaustive]
#[derive(Clone)]
pub struct WebhookMiddlewareState {
    replay_store: WebhookReplayBackend,
    allow_mock: bool,
    provider: Option<Arc<dyn BillingProvider>>,
}

#[cfg(any(feature = "axum", feature = "actix"))]
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

#[cfg(any(feature = "axum", feature = "actix"))]
impl WebhookMiddlewareState {
    /// Creates production-safe state that rejects `mock_*` verifier modes.
    pub fn production(replay_store: impl Into<WebhookReplayBackend>) -> Self {
        Self {
            replay_store: replay_store.into(),
            allow_mock: false,
            provider: None,
        }
    }

    /// Creates explicitly local-only state that permits signed `mock_*` fixtures.
    pub fn local_mock(replay_store: impl Into<WebhookReplayBackend>) -> Self {
        Self {
            replay_store: replay_store.into(),
            allow_mock: true,
            provider: None,
        }
    }

    /// Creates production-safe state bound to an explicit provider instance.
    pub fn production_with_provider<P>(
        provider: Arc<P>,
        replay_store: impl Into<WebhookReplayBackend>,
    ) -> Self
    where
        P: BillingProvider + 'static,
    {
        Self {
            replay_store: replay_store.into(),
            allow_mock: false,
            provider: Some(provider),
        }
    }

    /// Creates local-only state bound to an explicit provider and permits `mock_*` fixtures.
    pub fn local_mock_with_provider<P>(
        provider: Arc<P>,
        replay_store: impl Into<WebhookReplayBackend>,
    ) -> Self
    where
        P: BillingProvider + 'static,
    {
        Self {
            replay_store: replay_store.into(),
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

#[cfg(any(feature = "axum", feature = "actix"))]
pub(super) static DEFAULT_REPLAY_STORE: LazyLock<WebhookReplayBackend> =
    LazyLock::new(|| WebhookReplayBackend::Memory(Arc::new(InMemoryWebhookReplayStore::default())));

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
    replay_store: &WebhookReplayBackend,
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
    .await
    .map_err(capital_error_status)?;

    let request = rebuild_request(parts, body_bytes, event);
    Ok(next.run(request).await)
}

#[cfg(feature = "axum")]
fn rebuild_request(mut parts: Parts, body: Bytes, event: WebhookEvent) -> Request {
    parts.extensions.insert(event);
    Request::from_parts(parts, Body::from(body))
}

#[cfg(any(feature = "axum", feature = "actix", test))]
pub(super) async fn verify_payload(
    active_provider: &dyn BillingProvider,
    body: &[u8],
    headers: &HashMap<String, String>,
    replay_store: &WebhookReplayBackend,
    allow_mock: bool,
) -> Result<WebhookEvent, CapitalError> {
    let mode = active_provider.webhook_verification_mode()?;
    if mode == WebhookVerificationMode::Mock && !allow_mock {
        return Err(CapitalError::MockWebhookNotAllowed(
            active_provider.name().to_string(),
        ));
    }
    let event = active_provider.handle_webhook(body, headers)?;
    replay_store
        .record_payload(active_provider.name(), body)
        .await?;
    Ok(event)
}

pub(super) fn validate_replay_provider(provider: &str) -> Result<(), CapitalError> {
    let valid = !provider.is_empty()
        && provider.len() <= 64
        && provider
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'));
    if !valid {
        return Err(CapitalError::ConfigurationError(
            "Webhook replay provider must use 1-64 ASCII letters, digits, dots, hyphens, or underscores"
                .to_string(),
        ));
    }
    Ok(())
}

pub(super) fn validate_replay_event_key(event_key: &str) -> Result<(), CapitalError> {
    let valid = !event_key.is_empty()
        && event_key.len() <= 128
        && event_key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b':'));
    if !valid {
        return Err(CapitalError::ConfigurationError(
            "Webhook event key must use 1-128 ASCII letters, digits, dots, colons, hyphens, or underscores"
                .to_string(),
        ));
    }
    Ok(())
}

pub(super) fn payload_key(provider_name: &str, payload: &[u8]) -> String {
    scoped_replay_key(provider_name, b"payload", payload)
}

pub(super) fn event_key_hash(provider_name: &str, event_key: &str) -> String {
    scoped_replay_key(provider_name, b"event", event_key.as_bytes())
}

fn scoped_replay_key(provider_name: &str, domain: &[u8], material: &[u8]) -> String {
    let mut scoped = Vec::with_capacity(provider_name.len() + domain.len() + material.len() + 2);
    scoped.extend_from_slice(provider_name.as_bytes());
    scoped.push(0);
    scoped.extend_from_slice(domain);
    scoped.push(0);
    scoped.extend_from_slice(material);
    let hash = digest(&SHA256, &scoped);
    hex::encode(hash.as_ref())
}

#[cfg(any(feature = "axum", feature = "actix"))]
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
        | CapitalError::Provider(_)
        | CapitalError::WebhookReplayStoreFull
        | CapitalError::WebhookReplayStoreUnavailable
        | CapitalError::WebhookReplayConfigurationDrift
        | CapitalError::WebhookReplayCorruptState
        | CapitalError::UnsupportedOperation(_)
        | CapitalError::MockWebhookNotAllowed(_)
        | CapitalError::SubscriptionError(_)
        | CapitalError::Quota(_)
        | CapitalError::FiscalError(_) => 503,
    }
}

#[cfg(feature = "axum")]
fn capital_error_status(error: CapitalError) -> StatusCode {
    match StatusCode::from_u16(capital_error_status_code(&error)) {
        Ok(status) => status,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
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
        let store = InMemoryWebhookReplayStore::new(2, Duration::from_secs(60)).unwrap();
        assert!(store.record_payload("stripe", b"same body").is_ok());
        assert!(store.record_payload("paddle", b"same body").is_ok());
        assert!(matches!(
            store.record_payload("stripe", b"same body"),
            Err(CapitalError::WebhookReplay(_))
        ));
        assert_eq!(
            store.record_payload("stripe", b"different body"),
            Err(CapitalError::WebhookReplayStoreFull)
        );
    }

    #[test]
    fn semantic_event_keys_are_provider_scoped_and_validated() {
        let store = InMemoryWebhookReplayStore::new(2, Duration::from_secs(60)).unwrap();
        assert!(store.record_event_key("stripe", "evt_123").is_ok());
        assert!(store.record_event_key("paddle", "evt_123").is_ok());
        assert!(matches!(
            store.record_event_key("stripe", "evt_123"),
            Err(CapitalError::WebhookReplay(_))
        ));
        assert!(
            InMemoryWebhookReplayStore::default()
                .record_event_key("stripe", "contains spaces")
                .is_err()
        );
    }

    #[test]
    fn replay_store_rejects_invalid_configuration() {
        assert!(InMemoryWebhookReplayStore::new(0, Duration::from_secs(1)).is_err());
        assert!(InMemoryWebhookReplayStore::new(1, Duration::ZERO).is_err());
    }

    #[tokio::test]
    async fn canonical_verifier_decodes_stripe_and_lemonsqueezy_and_rejects_mock_in_production() {
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
        let stripe_store =
            WebhookReplayBackend::Memory(Arc::new(InMemoryWebhookReplayStore::default()));
        let stripe_event = verify_payload(
            &stripe,
            &stripe_payload,
            &stripe_headers,
            &stripe_store,
            true,
        )
        .await
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
        let lemon_store =
            WebhookReplayBackend::Memory(Arc::new(InMemoryWebhookReplayStore::default()));
        let lemon_event =
            verify_payload(&lemon, &lemon_payload, &lemon_headers, &lemon_store, true)
                .await
                .expect("local mock signature must produce a normalized event");
        assert_eq!(lemon_event.subscription_id, "sub_lemon");

        assert!(matches!(
            verify_payload(
                &stripe,
                &stripe_payload,
                &stripe_headers,
                &WebhookReplayBackend::Memory(Arc::new(
                    InMemoryWebhookReplayStore::default(),
                )),
                false,
            )
            .await,
            Err(CapitalError::MockWebhookNotAllowed(provider)) if provider == "stripe"
        ));
    }
}

#[cfg(all(test, feature = "axum"))]
#[path = "webhook_axum_tests.rs"]
mod axum_tests;
