//! The deprecated `oauth_state` fallback still validates a callback once, and
//! each use logs a deprecation warning without the state.

use super::*;
use axum::http::StatusCode;
use std::fmt::Write as _;
use std::sync::{Arc, Mutex};
use tower_sessions::MemoryStore;
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Level, Metadata, Subscriber};

const STATE: &str = "legacy-state-canary";

/// Records the fields of each `WARN` event as `name=value;` text.
#[derive(Clone, Default)]
struct Warnings(Arc<Mutex<Vec<String>>>);

struct Fields<'text>(&'text mut String);

impl Visit for Fields<'_> {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        let _ = write!(self.0, "{}={:?};", field.name(), value);
    }
}

impl Warnings {
    fn take(&self) -> Vec<String> {
        self.0
            .lock()
            .map(|mut warnings| std::mem::take(&mut *warnings))
            .unwrap_or_default()
    }
}

impl Subscriber for Warnings {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }
    fn new_span(&self, _attributes: &Attributes<'_>) -> Id {
        Id::from_u64(1)
    }
    fn record(&self, _span: &Id, _values: &Record<'_>) {}
    fn record_follows_from(&self, _span: &Id, _follows: &Id) {}
    fn event(&self, event: &Event<'_>) {
        if *event.metadata().level() != Level::WARN {
            return;
        }
        let mut text = String::new();
        event.record(&mut Fields(&mut text));
        if let Ok(mut warnings) = self.0.lock() {
            warnings.push(text);
        }
    }
    fn enter(&self, _span: &Id) {}
    fn exit(&self, _span: &Id) {}
}

async fn extract(session: Session, state: &str) -> Result<AuthSession, StatusCode> {
    let mut request = axum::http::Request::builder()
        .uri(format!("/callback?code=authorization-code&state={state}"))
        .body(())
        .expect("test request URI must be valid");
    request.extensions_mut().insert(session);
    let (mut parts, _) = request.into_parts();
    AuthSession::from_request_parts(&mut parts, &())
        .await
        .map_err(|response| response.status())
}

async fn legacy_session() -> Session {
    let session = Session::new(None, Arc::new(MemoryStore::default()), None);
    session
        .insert("oauth_state", STATE.to_string())
        .await
        .expect("legacy state must be stored");
    session
}

#[tokio::test(flavor = "current_thread")]
async fn the_deprecated_oauth_state_key_still_works_once_and_logs_a_warning() {
    let recorder = Warnings::default();
    let _guard = tracing::subscriber::set_default(recorder.clone());

    // A parallel test can race the callsite's first registration and cache an
    // interest computed before this scoped subscriber existed. Recompute it
    // and retry a bounded number of times.
    let mut warnings = Vec::new();
    for _ in 0..5 {
        tracing::callsite::rebuild_interest_cache();
        let session = legacy_session().await;
        let accepted = extract(session.clone(), STATE)
            .await
            .expect("the deprecated fallback must still validate the callback");
        assert_eq!(
            accepted.callback.code.as_deref(),
            Some("authorization-code")
        );
        assert_eq!(accepted.code_verifier(), None);
        assert_eq!(accepted.expected_nonce(), None);
        assert!(
            session
                .get::<String>("oauth_state")
                .await
                .expect("legacy lookup must succeed")
                .is_none(),
            "the fallback must consume the stored state"
        );
        assert_eq!(
            extract(session, STATE)
                .await
                .expect_err("a consumed legacy state cannot be replayed"),
            StatusCode::BAD_REQUEST
        );
        warnings = recorder.take();
        if !warnings.is_empty() {
            break;
        }
    }
    assert_eq!(warnings.len(), 1, "one warning per use: {warnings:?}");
    let warning = &warnings[0];
    for expected in [
        "session_key=\"oauth_state\"",
        "deprecated",
        "kept through 13.x",
        "begin_oauth_session",
        "begin_oidc_session",
        "challenge::begin_login",
        "finish_login",
    ] {
        assert!(warning.contains(expected), "{expected} missing: {warning}");
    }
    assert!(!warning.contains(STATE), "the state was logged: {warning}");

    // A mismatching state still uses, and consumes, the fallback.
    let session = legacy_session().await;
    assert_eq!(
        extract(session, "another-state")
            .await
            .expect_err("a mismatching legacy state must fail"),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(recorder.take().len(), 1, "a refused use also warns");

    // Without the legacy key, and with a managed challenge, nothing is logged.
    let empty = Session::new(None, Arc::new(MemoryStore::default()), None);
    assert_eq!(
        extract(empty, STATE)
            .await
            .expect_err("no stored state must fail"),
        StatusCode::BAD_REQUEST
    );
    let managed = Session::new(None, Arc::new(MemoryStore::default()), None);
    managed
        .insert(
            CHALLENGE_KEY,
            StoredChallenge {
                state: STATE.to_string(),
                nonce: None,
                code_verifier: "verifier".to_string(),
                expires_at_unix_seconds: u64::MAX,
            },
        )
        .await
        .expect("managed challenge must be stored");
    let accepted = extract(managed, STATE)
        .await
        .expect("the managed challenge must validate the callback");
    assert_eq!(accepted.code_verifier(), Some("verifier"));
    assert_eq!(recorder.take(), Vec::<String>::new());
}
