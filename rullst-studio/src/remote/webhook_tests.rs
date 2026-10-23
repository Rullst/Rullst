//! Webhook URLs and delivery (`TM-STUDIO-3`, `REMOTE-11`).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::alerts::AlertRules;
use super::projects::{ProjectError, ProjectStore, WebhookChange};
use super::server_test::{TOKEN, TempDir};
use super::webhook::{WebhookClient, WebhookPayload, validate_webhook_url};
use axum::Router;
use axum::http::{StatusCode, header};
use axum::response::IntoResponse;
use axum::routing::post;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// A webhook URL that embeds a credential, like most receivers.
const HOOK: &str = "https://hooks.example/services/T000/B000/secret-part?channel=ops";

#[test]
fn webhooks_are_https_urls_without_user_or_fragment() {
    assert_eq!(validate_webhook_url(HOOK).unwrap().as_str(), HOOK);
    for invalid in [
        "http://hooks.example/x",
        "http://127.0.0.1:9000/x",
        "https://user:pass@hooks.example/x",
        "https://hooks.example/x#frag",
        "ftp://hooks.example/x",
        "not a url",
    ] {
        assert_eq!(
            validate_webhook_url(invalid),
            Err(ProjectError::InvalidWebhook),
            "{invalid}"
        );
    }
    let long = format!("https://hooks.example/{}", "a".repeat(2_048));
    assert_eq!(
        validate_webhook_url(&long),
        Err(ProjectError::InvalidWebhook)
    );
}

#[test]
fn a_webhook_is_stored_sealed_and_can_be_kept_or_cleared() {
    let directory = TempDir::new("webhook-store");
    let store = ProjectStore::new(directory.paths());
    store.add("shop", "https://shop.example", TOKEN).unwrap();
    let rules = AlertRules {
        p95_ms: Some(250.0),
        ..AlertRules::default()
    };
    store
        .update_settings("shop", true, rules, WebhookChange::Set(HOOK.to_string()))
        .unwrap();
    let file = std::fs::read_to_string(&directory.paths().projects).unwrap();
    assert!(!file.contains("hooks.example"));
    let project = store.load().unwrap().remove(0);
    assert!(project.watch);
    assert_eq!(project.rules, rules);
    assert_eq!(project.webhook.as_ref().unwrap().expose(), HOOK);
    assert!(!format!("{project:?}").contains("secret-part"));

    store
        .update_settings("shop", false, rules, WebhookChange::Keep)
        .unwrap();
    assert!(store.load().unwrap()[0].webhook.is_some());
    store
        .update_settings("shop", false, rules, WebhookChange::Clear)
        .unwrap();
    let project = store.load().unwrap().remove(0);
    assert!(!project.webhook_configured);
    assert!(!project.watch);

    assert_eq!(
        store.update_settings(
            "shop",
            true,
            rules,
            WebhookChange::Set("http://x.example".into())
        ),
        Err(ProjectError::InvalidWebhook)
    );
    let invalid = AlertRules {
        cooldown_minutes: 0,
        ..rules
    };
    assert_eq!(
        store.update_settings("shop", true, invalid, WebhookChange::Keep),
        Err(ProjectError::InvalidRules)
    );
    assert_eq!(
        store.update_settings("missing", true, rules, WebhookChange::Keep),
        Err(ProjectError::NotFound)
    );
}

/// A receiver answering every post with `status` (or a redirect).
async fn receiver(status: StatusCode, delay: Duration) -> (String, Arc<Mutex<Vec<String>>>) {
    let bodies = Arc::new(Mutex::new(Vec::new()));
    let seen = bodies.clone();
    let app = Router::new().route(
        "/hook",
        post(move |body: String| {
            let seen = seen.clone();
            async move {
                tokio::time::sleep(delay).await;
                seen.lock().unwrap().push(body);
                if status.is_redirection() {
                    (status, [(header::LOCATION, "/elsewhere")]).into_response()
                } else {
                    status.into_response()
                }
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    (format!("http://{address}/hook"), bodies)
}

fn payload() -> WebhookPayload<'static> {
    WebhookPayload {
        source: "rullst-studio",
        project: "shop",
        rule: "p95_latency",
        message: "shop: p95 latency 300.0 ms (threshold 250 ms)",
        at: 1_800_000_000,
    }
}

#[tokio::test]
async fn delivery_posts_the_alert_once_and_reports_a_fixed_status() {
    let client = WebhookClient::with_timeout(Duration::from_millis(500)).unwrap();
    let (url, bodies) = receiver(StatusCode::NO_CONTENT, Duration::ZERO).await;
    assert_eq!(client.deliver(&url, &payload()).await, "delivered");
    let body: serde_json::Value = serde_json::from_str(&bodies.lock().unwrap()[0]).unwrap();
    assert_eq!(
        body,
        serde_json::json!({
            "source": "rullst-studio",
            "project": "shop",
            "rule": "p95_latency",
            "message": "shop: p95 latency 300.0 ms (threshold 250 ms)",
            "at": 1_800_000_000_u64
        })
    );

    let (failing, _) = receiver(StatusCode::INTERNAL_SERVER_ERROR, Duration::ZERO).await;
    assert_eq!(
        client.deliver(&failing, &payload()).await,
        "failed: HTTP 500"
    );
    let (redirect, redirected) = receiver(StatusCode::FOUND, Duration::ZERO).await;
    assert_eq!(
        client.deliver(&redirect, &payload()).await,
        "failed: redirect not followed"
    );
    assert_eq!(redirected.lock().unwrap().len(), 1);
    let (slow, _) = receiver(StatusCode::OK, Duration::from_secs(5)).await;
    assert_eq!(client.deliver(&slow, &payload()).await, "failed: timeout");

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let closed = format!("http://{}/hook", listener.local_addr().unwrap());
    drop(listener);
    // Windows retries a refused loopback connection for about two seconds.
    let patient = WebhookClient::with_timeout(Duration::from_secs(6)).unwrap();
    assert_eq!(
        patient.deliver(&closed, &payload()).await,
        "failed: unreachable"
    );
}

#[test]
fn a_webhook_url_may_fill_2048_characters() {
    let longest = format!("https://hooks.example/{}", "a".repeat(2_048 - 22));
    assert_eq!(longest.len(), 2_048);
    assert!(validate_webhook_url(&longest).is_ok());
    assert_eq!(
        validate_webhook_url(&format!("{longest}a")).unwrap_err(),
        ProjectError::InvalidWebhook
    );
}
