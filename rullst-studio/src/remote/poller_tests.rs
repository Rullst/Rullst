//! The background poller (`TM-STUDIO-3`, `REMOTE-09`, `REMOTE-10`).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::alerts::AlertRules;
use super::client::FetchLimits;
use super::poller::{clamp_interval, poll_round};
use super::projects::WebhookChange;
use super::router::RemoteState;
use super::server_test::{TOKEN, TempDir, start};
use super::{RemoteStudioOptions, state};
use std::time::Duration;

async fn remote_state(directory: &TempDir) -> RemoteState {
    let mut options = RemoteStudioOptions::new(directory.paths());
    options.limits = FetchLimits {
        max_bytes: 1024 * 1024,
        timeout: Duration::from_millis(800),
        connect_timeout: Duration::from_millis(500),
    };
    state(&options).await.unwrap()
}

#[test]
fn the_interval_is_clamped() {
    assert_eq!(
        clamp_interval(Duration::from_secs(1)),
        Duration::from_secs(15)
    );
    assert_eq!(
        clamp_interval(Duration::from_secs(60)),
        Duration::from_secs(60)
    );
    assert_eq!(
        clamp_interval(Duration::from_secs(7_200)),
        Duration::from_secs(3_600)
    );
}

#[tokio::test]
async fn only_watched_projects_are_read_and_their_samples_kept() {
    let (base, seen) = start(None).await;
    let directory = TempDir::new("poller");
    let state = remote_state(&directory).await;
    state
        .store
        .add("watched", &format!("{base}ok/"), TOKEN)
        .unwrap();
    state
        .store
        .add("idle", &format!("{base}ok/"), TOKEN)
        .unwrap();
    state
        .store
        .update_settings("watched", true, AlertRules::default(), WebhookChange::Keep)
        .unwrap();
    let history = state.history.clone().unwrap();

    assert_eq!(poll_round(&state, &history, 1_000).await, 1);
    assert_eq!(seen.authorizations.lock().unwrap().len(), 1);
    assert_eq!(history.samples("watched", 0).await.unwrap().len(), 1);
    assert!(history.samples("idle", 0).await.unwrap().is_empty());
    assert!(
        state
            .monitoring
            .last_status("watched")
            .unwrap()
            .contains("succeeded")
    );
}

#[tokio::test]
async fn an_unreachable_watched_project_alerts_once_per_cool_down() {
    let directory = TempDir::new("poller-alerts");
    let state = remote_state(&directory).await;
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}/", listener.local_addr().unwrap());
    drop(listener);
    state.store.add("down", &base, TOKEN).unwrap();
    let rules = AlertRules {
        unreachable: true,
        cooldown_minutes: 10,
        ..AlertRules::default()
    };
    state
        .store
        .update_settings("down", true, rules, WebhookChange::Keep)
        .unwrap();
    let history = state.history.clone().unwrap();

    poll_round(&state, &history, 10_000).await;
    poll_round(&state, &history, 10_060).await;
    let alerts = history.alerts(10).await.unwrap();
    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0].rule, "unreachable");
    assert_eq!(alerts[0].message, "down could not be read");
    assert_eq!(alerts[0].delivery, "no webhook configured");
    assert_eq!(state.monitoring.open_alerts(), 1);

    // Ten minutes later the cool-down has passed.
    poll_round(&state, &history, 10_600).await;
    assert_eq!(history.alerts(10).await.unwrap().len(), 2);
    assert_eq!(state.monitoring.open_alerts(), 2);
    assert_eq!(history.samples("down", 0).await.unwrap().len(), 3);
}
