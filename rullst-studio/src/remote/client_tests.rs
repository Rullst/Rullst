//! Reading documents from a local test server, including hostile variants
//! (`TM-STUDIO-3`, `REMOTE-04`, `REMOTE-05`).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::client::{FetchError, FetchLimits, MonitorClient};
use super::document::Database;
use super::projects::{Project, ProjectStore};
use super::server_test::{TOKEN, TempDir, start};
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
    // A port nothing listens on: bind, read the port, then close.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}/", listener.local_addr().unwrap());
    drop(listener);
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
