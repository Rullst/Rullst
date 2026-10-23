//! History and alerts pages and their forms (`TM-STUDIO-3`, `REMOTE-02`,
//! `REMOTE-08`, `REMOTE-11`).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::history::Sample;
use super::router_tests::{ORIGIN, get, post, studio};
use super::server_test::TOKEN;
use axum::http::StatusCode;

fn settings(form_token: &str, extra: &str) -> String {
    format!("form_token={form_token}&name=shop&{extra}")
}

#[tokio::test]
async fn settings_are_saved_through_the_protected_form() {
    let studio = studio().await;
    studio
        .state
        .store
        .add("shop", "https://shop.example", TOKEN)
        .unwrap();
    let token = studio.state.form_token.clone();
    let body = settings(
        &token,
        "watch=on&unreachable=on&error_rate_percent=5&p95_ms=250&queue_pending=&queue_failures=3&cooldown_minutes=30&webhook=https%3A%2F%2Fhooks.example%2Fsecret-part",
    );
    assert_eq!(
        post(&studio, "/studio/alerts/settings", Some(ORIGIN), &body).await,
        StatusCode::SEE_OTHER
    );
    let project = studio.state.store.load().unwrap().remove(0);
    assert!(project.watch);
    assert!(project.rules.unreachable);
    assert_eq!(project.rules.error_rate_percent, Some(5.0));
    assert_eq!(project.rules.p95_ms, Some(250.0));
    assert_eq!(project.rules.queue_pending, None);
    assert_eq!(project.rules.queue_failures, Some(3));
    assert_eq!(project.rules.cooldown_minutes, 30);
    assert!(project.webhook.is_some());

    let (status, page) = get(&studio, "/studio/alerts?notice=saved").await;
    assert_eq!(status, StatusCode::OK);
    assert!(page.contains("Settings saved"));
    assert!(page.contains("configured — leave empty to keep"));
    assert!(!page.contains("secret-part"));

    // Invalid values are refused with the page and change nothing.
    let invalid = settings(&token, "watch=on&error_rate_percent=abc");
    assert_eq!(
        post(&studio, "/studio/alerts/settings", Some(ORIGIN), &invalid).await,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let http_hook = settings(&token, "webhook=http%3A%2F%2Fhooks.example%2Fx");
    assert_eq!(
        post(&studio, "/studio/alerts/settings", Some(ORIGIN), &http_hook).await,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert!(studio.state.store.load().unwrap()[0].watch);

    // A cross-origin post or a wrong form token changes nothing either.
    let off = settings(&token, "");
    assert_eq!(
        post(
            &studio,
            "/studio/alerts/settings",
            Some("https://attacker.example"),
            &off
        )
        .await,
        StatusCode::FORBIDDEN
    );
    let wrong = settings(&"0".repeat(64), "");
    assert_eq!(
        post(&studio, "/studio/alerts/settings", Some(ORIGIN), &wrong).await,
        StatusCode::FORBIDDEN
    );
    assert!(studio.state.store.load().unwrap()[0].watch);

    // Clearing the webhook and the watch flag.
    let clear = settings(&token, "clear_webhook=on&cooldown_minutes=15");
    assert_eq!(
        post(&studio, "/studio/alerts/settings", Some(ORIGIN), &clear).await,
        StatusCode::SEE_OTHER
    );
    let project = studio.state.store.load().unwrap().remove(0);
    assert!(!project.watch);
    assert!(!project.webhook_configured);
}

#[tokio::test]
async fn history_and_alerts_render_escaped_and_alerts_can_be_acknowledged() {
    let studio = studio().await;
    studio
        .state
        .store
        .add("shop", "https://shop.example", TOKEN)
        .unwrap();
    let history = studio.state.history.clone().unwrap();
    let now = super::monitoring::unix_now();
    for (offset, reachable) in [(120, true), (60, false), (0, true)] {
        history
            .record(
                "shop",
                &Sample {
                    at: now - offset,
                    reachable,
                    requests_per_second: reachable.then_some(1.25),
                    window_requests: reachable.then_some(30),
                    error_rate_percent: reachable.then_some(2.0),
                    p95_ms: reachable.then_some(40.0),
                    queue_pending: reachable.then_some(4),
                    queue_failed_total: reachable.then_some(1),
                },
            )
            .await
            .unwrap();
    }
    let (status, page) = get(&studio, "/studio/history?project=shop&range=1h").await;
    assert_eq!(status, StatusCode::OK);
    assert!(page.contains("History · shop"));
    assert!(page.contains("3 samples, 1 unreachable."));
    assert!(page.contains("<polyline"));
    assert!(page.contains("1.250"));

    history
        .record_alert(
            "shop",
            now,
            "p95_latency",
            "<b>shop</b>: p95",
            "no webhook configured",
        )
        .await
        .unwrap();
    studio.state.monitoring.set_open_alerts(1);
    let (_, page) = get(&studio, "/studio/alerts").await;
    assert!(page.contains("&lt;b&gt;shop&lt;/b&gt;: p95"));
    assert!(!page.contains("<b>shop</b>"));
    assert!(page.contains("Acknowledge all"));
    assert!(page.contains(r#"border-rose-500/30">1</span>"#));

    let body = format!("form_token={}", studio.state.form_token);
    assert_eq!(
        post(&studio, "/studio/alerts/acknowledge", Some(ORIGIN), &body).await,
        StatusCode::SEE_OTHER
    );
    assert_eq!(history.unacknowledged().await.unwrap(), 0);
    assert_eq!(studio.state.monitoring.open_alerts(), 0);
    let (_, page) = get(&studio, "/studio/alerts?notice=acknowledged").await;
    assert!(page.contains("Alerts acknowledged"));
    assert!(!page.contains(r#"border-rose-500/30">1</span>"#));
}

#[tokio::test]
async fn an_unwatched_project_has_an_empty_history() {
    let studio = studio().await;
    studio
        .state
        .store
        .add("shop", "https://shop.example", TOKEN)
        .unwrap();
    let (status, page) = get(&studio, "/studio/history?project=shop").await;
    assert_eq!(status, StatusCode::OK);
    assert!(page.contains("No history yet"));
}

#[tokio::test]
async fn the_settings_form_shows_the_saved_rules() {
    let studio = studio().await;
    studio
        .state
        .store
        .add("shop", "https://shop.example", TOKEN)
        .unwrap();
    let body = settings(
        &studio.state.form_token,
        "watch=on&p95_ms=250&cooldown_minutes=30",
    );
    assert_eq!(
        post(&studio, "/studio/alerts/settings", Some(ORIGIN), &body).await,
        StatusCode::SEE_OTHER
    );
    let (_, page) = get(&studio, "/studio/alerts").await;
    for expected in [
        r#"name="watch" value="on" checked />"#,
        r#"name="unreachable" value="on" />"#,
        r#"name="p95_ms" inputmode="decimal" value="250""#,
        r#"name="error_rate_percent" inputmode="decimal" value="""#,
        r#"name="cooldown_minutes" inputmode="decimal" value="30""#,
    ] {
        assert!(page.contains(expected), "{expected}");
    }
}
