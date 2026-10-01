//! Tracing spans around the OIDC token exchange must not record the nonce.

use super::discovery::OidcProvider;
use crate::client::{HttpClient, HttpRequest, HttpResponse};
use crate::error::ConnectError;
use crate::provider::Provider;
use async_trait::async_trait;
use serde_json::json;
use std::fmt::Write as _;
use std::sync::{Arc, Mutex};
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Metadata, Subscriber};

const NONCE: &str = "nonce-value-that-must-stay-out-of-spans";

struct FixtureClient;

#[async_trait]
impl HttpClient for FixtureClient {
    async fn execute(&self, request: HttpRequest) -> Result<HttpResponse, ConnectError> {
        let body = if request.url.contains("openid-configuration") {
            json!({
                "issuer": "https://issuer.example",
                "authorization_endpoint": "https://issuer.example/authorize",
                "token_endpoint": "https://issuer.example/token",
                "userinfo_endpoint": "https://issuer.example/userinfo",
                "jwks_uri": "https://issuer.example/jwks"
            })
        } else {
            json!({"access_token": "fixture-access", "expires_in": 60})
        };
        Ok(HttpResponse { status: 200, body })
    }
}

/// Records every span and event field as `name=value;` text.
#[derive(Clone, Default)]
struct FieldRecorder(Arc<Mutex<String>>);

struct Writer<'text>(&'text mut String);

impl Visit for Writer<'_> {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        let _ = write!(self.0, "{}={:?};", field.name(), value);
    }
}

impl FieldRecorder {
    fn text(&self) -> String {
        self.0.lock().map(|text| text.clone()).unwrap_or_default()
    }
}

impl Subscriber for FieldRecorder {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }
    fn new_span(&self, attributes: &Attributes<'_>) -> Id {
        if let Ok(mut text) = self.0.lock() {
            attributes.record(&mut Writer(&mut text));
        }
        Id::from_u64(1)
    }
    fn record(&self, _span: &Id, values: &Record<'_>) {
        if let Ok(mut text) = self.0.lock() {
            values.record(&mut Writer(&mut text));
        }
    }
    fn record_follows_from(&self, _span: &Id, _follows: &Id) {}
    fn event(&self, event: &Event<'_>) {
        if let Ok(mut text) = self.0.lock() {
            event.record(&mut Writer(&mut text));
        }
    }
    fn enter(&self, _span: &Id) {}
    fn exit(&self, _span: &Id) {}
}

#[tokio::test(flavor = "current_thread")]
async fn token_exchange_span_records_only_nonce_presence() {
    let provider = OidcProvider::discover_with_client(
        "https://issuer.example",
        "client_id",
        "client_secret",
        "https://app.example/callback",
        Arc::new(FixtureClient),
    )
    .await
    .unwrap();
    let recorder = FieldRecorder::default();
    let _guard = tracing::subscriber::set_default(recorder.clone());

    // A parallel test can race the callsite's first registration and cache an
    // interest computed before this scoped subscriber existed. Recompute it and
    // retry a bounded number of times; the nonce must never appear.
    let mut recorded = String::new();
    for _ in 0..5 {
        tracing::callsite::rebuild_interest_cache();
        // No ID token is returned, so the nonce-bound login fails after the span opened.
        let result = provider
            .get_user(crate::provider::ExchangeParams {
                auth_code: "code",
                code_verifier: None,
                expected_nonce: Some(NONCE),
            })
            .await;
        assert!(result.is_err());
        recorded = recorder.text();
        assert!(
            !recorded.contains(NONCE),
            "the nonce was recorded in a span"
        );
        if recorded.contains("has_nonce=true") {
            break;
        }
    }
    assert!(
        recorded.contains("has_nonce=true"),
        "nonce presence was not recorded"
    );
}
