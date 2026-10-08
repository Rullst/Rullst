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
#[cfg(any(feature = "axum", feature = "actix"))]
use std::sync::{Arc, LazyLock};
use std::time::Duration;

#[cfg(feature = "webhook-sql")]
mod sql;
#[cfg(feature = "webhook-sql")]
pub use sql::{SqlWebhookBackend, SqlWebhookReplayStore};

#[cfg(feature = "webhook-sql")]
mod inbox;
#[cfg(feature = "webhook-sql")]
pub use inbox::{
    SqlStripeEventInbox, StripeInboxError, StripeInboxOutcome, StripeInboxResult, StripeInboxScope,
};

mod replay;
pub use replay::{InMemoryWebhookReplayStore, WebhookReplayBackend};

pub(super) const MAX_WEBHOOK_PAYLOAD_BYTES: usize = 2 * 1024 * 1024;
const MAX_REPLAY_CAPACITY: usize = 1_000_000;
const MAX_REPLAY_TTL: Duration = Duration::from_secs(30 * 24 * 60 * 60);

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
///
/// Replay proofs go to a process-wide default [`InMemoryWebhookReplayStore`] holding at most
/// 10,000 proofs for 24 hours each. When it is full, further deliveries get 503 instead of
/// evicting an unexpired proof; mount [`verify_webhook_with_state`] with a sized or shared store
/// for more than about 10,000 verified deliveries per day.
#[cfg(feature = "axum")]
pub async fn verify_webhook(req: Request, next: Next) -> Result<Response, StatusCode> {
    verify_webhook_inner(req, next, &DEFAULT_REPLAY_STORE, false, provider()).await
}

/// Explicit local-only middleware variant for deterministic `mock_*` webhook credentials.
///
/// Mock signatures are still mandatory and must equal the configured `mock_*` secret. Do not mount
/// this middleware on a publicly reachable endpoint. It shares the bounded default replay store of
/// [`verify_webhook`].
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
    if ["webhook-id", "webhook-timestamp", "webhook-signature"]
        .iter()
        .any(|name| {
            parts
                .headers
                .iter()
                .filter(|(key, _)| key.as_str() == *name)
                .count()
                > 1
        })
    {
        return Err(StatusCode::BAD_REQUEST);
    }
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
        | CapitalError::Quota(_) => 503,
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
mod tests;

#[cfg(all(test, feature = "axum"))]
#[path = "webhook_axum_tests.rs"]
mod axum_tests;
