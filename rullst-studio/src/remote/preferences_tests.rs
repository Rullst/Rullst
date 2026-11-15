//! The settings page and the notification feed (`TM-STUDIO-3`, `REMOTE-02`,
//! `REMOTE-12`).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::notify::NotificationSettings;
use super::router_tests::{ORIGIN, get, post, studio};
use super::server_test::TOKEN;
use axum::http::StatusCode;
use serde_json::Value;

#[tokio::test]
async fn notification_settings_are_saved_through_the_protected_form() {
    let studio = studio().await;
    studio
        .state
        .store
        .add("shop", "https://shop.example", TOKEN)
        .unwrap();
    let projects = std::fs::read_to_string(&studio.state.store.paths().projects).unwrap();
    // Default settings are not written, so earlier Studios keep reading it.
    assert!(!projects.contains("settings"));
    let (status, page) = get(&studio, "/studio/settings").await;
    assert_eq!(status, StatusCode::OK);
    assert!(page.contains("Desktop notifications"));
    assert!(page.contains(r#"id="studio-notify-permission""#));
    assert!(page.contains(r#"name="quiet_minutes" inputmode="numeric" value="10""#));
    assert!(page.contains(r#"<script src="/studio/assets/notifications.js" defer></script>"#));

    let token = studio.state.form_token.clone();
    let form = format!("form_token={token}&enabled=on&quiet_minutes=5");
    assert_eq!(
        post(&studio, "/studio/settings/notifications", None, &form).await,
        StatusCode::FORBIDDEN
    );
    let wrong = "form_token=wrong&enabled=on&quiet_minutes=5";
    assert_eq!(
        post(
            &studio,
            "/studio/settings/notifications",
            Some(ORIGIN),
            wrong
        )
        .await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        studio.state.store.settings().unwrap().notifications,
        NotificationSettings::default()
    );
    assert_eq!(
        post(
            &studio,
            "/studio/settings/notifications",
            Some(ORIGIN),
            &form
        )
        .await,
        StatusCode::SEE_OTHER
    );
    let saved = studio.state.store.settings().unwrap().notifications;
    assert_eq!(
        saved,
        NotificationSettings {
            enabled: true,
            quiet_minutes: 5
        }
    );
    let (_, page) = get(&studio, "/studio/settings?notice=saved").await;
    assert!(page.contains("Settings saved"));
    assert!(page.contains(r#"name="enabled" value="on" checked"#));
    // The project list still loads, with its project.
    assert_eq!(studio.state.store.load().unwrap().len(), 1);

    for invalid in ["0", "1441", "soon", ""] {
        let form = format!("form_token={token}&quiet_minutes={invalid}");
        assert_eq!(
            post(
                &studio,
                "/studio/settings/notifications",
                Some(ORIGIN),
                &form
            )
            .await,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{invalid}"
        );
    }
    assert_eq!(studio.state.store.settings().unwrap().notifications, saved);
    let form = format!("form_token={token}&quiet_minutes=1440");
    assert_eq!(
        post(
            &studio,
            "/studio/settings/notifications",
            Some(ORIGIN),
            &form
        )
        .await,
        StatusCode::SEE_OTHER
    );
    assert!(!studio.state.store.settings().unwrap().notifications.enabled);
}

async fn feed(studio: &super::router_tests::Studio, query: &str) -> Value {
    let (status, body) = get(studio, &format!("/studio/notifications{query}")).await;
    assert_eq!(status, StatusCode::OK);
    serde_json::from_str(&body).unwrap()
}

#[tokio::test]
async fn the_feed_returns_notifications_after_the_page_cursor() {
    let studio = studio().await;
    let session = studio.state.notifier.session().to_string();
    assert_eq!(session.len(), 16);
    let first = feed(&studio, "").await;
    assert_eq!(first["enabled"], false);
    assert_eq!(first["session"], session.as_str());
    assert_eq!(first["cursor"], 0);
    assert_eq!(first["notifications"], serde_json::json!([]));

    let settings = NotificationSettings {
        enabled: true,
        ..NotificationSettings::default()
    };
    studio.state.store.update_notifications(settings).unwrap();
    studio
        .state
        .notifier
        .offer(&settings, "shop", "shop could not be read", 1_800_000_000);
    let next = feed(&studio, &format!("?session={session}&after=0")).await;
    assert_eq!(next["enabled"], true);
    assert_eq!(next["cursor"], 1);
    assert_eq!(
        next["notifications"],
        serde_json::json!([{"id": 1, "title": "Rullst Studio · shop", "body": "shop could not be read"}])
    );
    let seen = feed(&studio, &format!("?session={session}&after=1")).await;
    assert_eq!(seen["notifications"], serde_json::json!([]));
    let other = feed(&studio, "?session=0123456789abcdef&after=0").await;
    assert_eq!(other["notifications"], serde_json::json!([]));
    assert_eq!(other["cursor"], 1);
}
