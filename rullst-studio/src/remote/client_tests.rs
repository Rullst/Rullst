//! Reading documents from a local test server, including hostile variants
//! (`TM-STUDIO-3`, `REMOTE-04`, `REMOTE-05`).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::client::{FetchError, FetchLimits, MonitorClient};
use super::document::Database;
use super::projects::{Project, ProjectStore};
use super::server_test::{RefusedPort, TOKEN, TempDir, start};
use std::time::{Duration, Instant};

fn limits() -> FetchLimits {
    FetchLimits {
        max_bytes: 1024 * 1024,
        timeout: Duration::from_millis(800),
        connect_timeout: Duration::from_millis(500),
    }
}

/// A project named `name` reading `<base><variant>/`.
fn project(directory: &TempDir, base: &str, variant: &str, token: &str) -> Project {
    let store = ProjectStore::new(directory.paths());
    let name = format!("p-{variant}-{}", token.len());
    store
        .add(&name, &format!("{base}{variant}/"), token)
        .unwrap();
    store
        .load()
        .unwrap()
        .into_iter()
        .find(|project| project.name == name)
        .unwrap()
}

#[tokio::test]
async fn a_valid_document_is_read_with_the_token_of_its_project() {
    let (base, seen) = start(None).await;
    let directory = TempDir::new("client-ok");
    let client = MonitorClient::new(limits()).unwrap();
    let document = client
        .fetch(&project(&directory, &base, "ok", TOKEN))
        .await
        .unwrap();
    assert_eq!(document.schema, "rullst.monitor.v1");
    assert_eq!(document.http.routes[0].route, "/orders/{id}");
    assert_eq!(document.queues[0].name, "mail");
    assert!(matches!(
        document.database,
        Database::Observed {
            queries_total: 9876,
            ..
        }
    ));
    assert_eq!(
        seen.authorizations.lock().unwrap().as_slice(),
        [format!("Bearer {TOKEN}")]
    );

    let wrong = client
        .fetch(&project(&directory, &base, "ok", "another-token-value"))
        .await;
    assert_eq!(wrong.unwrap_err(), FetchError::Unauthorized);
}

#[tokio::test]
async fn hostile_documents_become_fixed_statuses() {
    let (base, _) = start(None).await;
    let directory = TempDir::new("client-hostile");
    let client = MonitorClient::new(limits()).unwrap();
    for (variant, expected) in [
        ("huge", FetchError::TooLarge),
        ("stream", FetchError::TooLarge),
        ("malformed", FetchError::Invalid),
        ("schema", FetchError::Invalid),
        ("html", FetchError::NotJson),
        ("status", FetchError::UpgradeRequired),
        ("missing", FetchError::NotMounted),
    ] {
        let outcome = client
            .fetch(&project(&directory, &base, variant, TOKEN))
            .await;
        assert_eq!(outcome.unwrap_err(), expected, "{variant}");
    }
}

#[tokio::test]
async fn a_slow_project_is_cut_off_at_the_time_limit() {
    let (base, _) = start(None).await;
    let directory = TempDir::new("client-slow");
    let client = MonitorClient::new(limits()).unwrap();
    let started = Instant::now();
    let outcome = client
        .fetch(&project(&directory, &base, "slow", TOKEN))
        .await;
    assert_eq!(outcome.unwrap_err(), FetchError::Timeout);
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[tokio::test]
async fn redirects_are_never_followed_so_the_token_stays_with_its_origin() {
    let (other, other_seen) = start(None).await;
    let (base, _) = start(Some(format!("{other}ok/_rullst/monitor/v1"))).await;
    let directory = TempDir::new("client-redirect");
    let client = MonitorClient::new(limits()).unwrap();
    let outcome = client
        .fetch(&project(&directory, &base, "redirect", TOKEN))
        .await;
    assert_eq!(outcome.unwrap_err(), FetchError::Redirect);
    assert!(other_seen.authorizations.lock().unwrap().is_empty());
}

#[tokio::test]
async fn an_unreachable_project_or_an_unreadable_token_is_reported() {
    let directory = TempDir::new("client-unreachable");
    // Windows retries a refused loopback connection for about two seconds, so
    // this client waits long enough for the refusal on every platform.
    let client = MonitorClient::new(FetchLimits {
        timeout: Duration::from_secs(6),
        connect_timeout: Duration::from_secs(5),
        ..limits()
    })
    .unwrap();
    let refused = RefusedPort::new();
    let base = format!("http://127.0.0.1:{}/", refused.port);
    let mut unreachable = project(&directory, &base, "ok", TOKEN);
    assert_eq!(
        client.fetch(&unreachable).await.unwrap_err(),
        FetchError::Unreachable
    );
    unreachable.token = None;
    assert_eq!(
        client.fetch(&unreachable).await.unwrap_err(),
        FetchError::TokenUnreadable
    );
}

#[test]
fn every_status_message_is_fixed_text() {
    for error in [
        FetchError::TokenUnreadable,
        FetchError::InvalidUrl,
        FetchError::Unreachable,
        FetchError::Timeout,
        FetchError::Redirect,
        FetchError::Unauthorized,
        FetchError::Forbidden,
        FetchError::NotMounted,
        FetchError::UpgradeRequired,
        FetchError::RateLimited,
        FetchError::TooLarge,
        FetchError::NotJson,
        FetchError::Invalid,
    ] {
        assert!(!error.message().is_empty());
        assert!(!error.message().contains('<'));
    }
    assert_eq!(
        FetchError::Status(502).message(),
        "unexpected HTTP status 502"
    );
}

#[tokio::test]
async fn a_document_may_fill_the_byte_limit_exactly() {
    let (base, _) = start(None).await;
    let exact = super::server_test::document("/orders/{id}")
        .to_string()
        .len();
    for (max_bytes, fits) in [(exact, true), (exact - 1, false)] {
        let directory = TempDir::new("client-exact");
        let client = MonitorClient::new(FetchLimits {
            max_bytes,
            ..limits()
        })
        .unwrap();
        let outcome = client.fetch(&project(&directory, &base, "ok", TOKEN)).await;
        assert_eq!(outcome.is_ok(), fits, "{max_bytes}: {outcome:?}");
    }
}

#[tokio::test]
async fn version_2_is_read_first_with_its_traces_and_security_radar() {
    let (base, seen) = start(None).await;
    let directory = TempDir::new("client-v2");
    let client = MonitorClient::new(limits()).unwrap();
    let document = client
        .fetch(&project(&directory, &base, "v2", TOKEN))
        .await
        .unwrap();
    assert_eq!(document.schema, "rullst.monitor.v2");
    let traces = document.traces.unwrap();
    assert_eq!(traces.sampled_total, 321);
    assert_eq!(traces.requests[0].status, 503);
    assert_eq!(traces.spans.kinds[0].kind, "sql");
    let security = document.security.unwrap();
    assert_eq!(security.events_total, 17);
    assert_eq!(security.counters["rate_limit_blocks"], 40);
    assert_eq!(security.events[0].event_type, "LOGIN_JAIL_TRIGGERED");
    assert_eq!(seen.v2_misses(), 0);
    assert_eq!(client.legacy().len(), 0);
}

// Applications before 13.0.0-alpha.6 serve version 1 only.
#[tokio::test]
async fn an_older_application_is_read_with_version_1_and_remembered() {
    let (base, seen) = start(None).await;
    let directory = TempDir::new("client-v1");
    let client = MonitorClient::new(limits()).unwrap();
    let older = project(&directory, &base, "ok", TOKEN);
    for _ in 0..3 {
        let document = client.fetch(&older).await.unwrap();
        assert_eq!(document.schema, "rullst.monitor.v1");
        assert!(document.traces.is_none() && document.security.is_none());
    }
    // Only the first read asked for version 2.
    assert_eq!(seen.v2_misses(), 1);
    assert_eq!(seen.authorizations.lock().unwrap().len(), 3);
}

#[tokio::test]
async fn version_2_failures_other_than_404_are_reported_not_downgraded() {
    let (base, seen) = start(None).await;
    let directory = TempDir::new("client-v2-errors");
    let client = MonitorClient::new(limits()).unwrap();
    for (variant, expected) in [
        ("v2error", FetchError::Status(500)),
        ("v2bare", FetchError::Invalid),
    ] {
        let outcome = client
            .fetch(&project(&directory, &base, variant, TOKEN))
            .await;
        assert_eq!(outcome.unwrap_err(), expected, "{variant}");
    }
    assert_eq!(seen.v2_misses(), 0);
    assert_eq!(client.legacy().len(), 0);
}

#[test]
fn a_version_1_project_is_rechecked_after_ten_minutes() {
    let legacy = super::client::LegacyProjects::default();
    let now = Instant::now();
    legacy.mark_at("https://a.example/_rullst/monitor/v1", now);
    let ten_minutes = Duration::from_secs(600);
    assert!(legacy.is_legacy_at("https://a.example/_rullst/monitor/v1", now));
    assert!(legacy.is_legacy_at(
        "https://a.example/_rullst/monitor/v1",
        now + ten_minutes - Duration::from_nanos(1)
    ));
    assert!(!legacy.is_legacy_at("https://a.example/_rullst/monitor/v1", now + ten_minutes));
    assert!(!legacy.is_legacy_at("https://b.example/_rullst/monitor/v1", now));
}

#[test]
fn the_version_1_list_is_cleared_when_full() {
    let legacy = super::client::LegacyProjects::default();
    let now = Instant::now();
    for index in 0..64 {
        legacy.mark_at(&format!("https://{index}.example/"), now);
    }
    assert_eq!(legacy.len(), 64);
    legacy.mark_at("https://64.example/", now);
    assert_eq!(legacy.len(), 1);
    assert!(legacy.is_legacy_at("https://64.example/", now));
    assert!(!legacy.is_legacy_at("https://0.example/", now));
}
