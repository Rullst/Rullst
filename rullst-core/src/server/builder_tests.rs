#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::config::{Environment, RullstConfig};

struct EnvironmentGuard {
    values: Vec<(&'static str, Option<std::ffi::OsString>)>,
}

impl EnvironmentGuard {
    fn clear(keys: &[&'static str]) -> Self {
        let values = keys
            .iter()
            .map(|key| (*key, std::env::var_os(key)))
            .collect::<Vec<_>>();
        for key in keys {
            unsafe { std::env::remove_var(key) };
        }
        Self { values }
    }

    fn set(&self, key: &'static str, value: impl AsRef<std::ffi::OsStr>) {
        unsafe { std::env::set_var(key, value) };
    }
}

impl Drop for EnvironmentGuard {
    fn drop(&mut self) {
        for (key, value) in &self.values {
            unsafe {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
        }
    }
}

#[tokio::test]
async fn networking_respects_precedence_defaults_and_invalid_values() {
    let _lock = crate::server::TEST_ENV_LOCK.lock().await;
    let environment = EnvironmentGuard::clear(&["HOST", "RULLST_HOST", "PORT"]);
    let empty = HashMap::new();

    let development = Server::setup_networking(3000, None, Environment::Development, &empty)
        .expect("development default address");
    assert_eq!(development, "127.0.0.1:3000".parse().unwrap());
    let production = Server::setup_networking(3000, Some(8080), Environment::Production, &empty)
        .expect("production default address");
    assert_eq!(production, "0.0.0.0:8080".parse().unwrap());

    let dotenv = HashMap::from([
        ("HOST".to_string(), "127.0.0.2".to_string()),
        ("RULLST_HOST".to_string(), "127.0.0.3".to_string()),
        ("PORT".to_string(), "4000".to_string()),
    ]);
    let from_dotenv = Server::setup_networking(3000, Some(8080), Environment::Test, &dotenv)
        .expect("dotenv address");
    assert_eq!(from_dotenv, "127.0.0.2:4000".parse().unwrap());

    environment.set("RULLST_HOST", "127.0.0.4");
    environment.set("PORT", "5000");
    let legacy_host = Server::setup_networking(3000, None, Environment::Test, &HashMap::new())
        .expect("legacy host address");
    assert_eq!(legacy_host, "127.0.0.4:5000".parse().unwrap());

    environment.set("HOST", "127.0.0.5");
    let primary_host = Server::setup_networking(3000, None, Environment::Test, &HashMap::new())
        .expect("primary host address");
    assert_eq!(primary_host, "127.0.0.5:5000".parse().unwrap());

    environment.set("PORT", "not-a-port");
    assert!(matches!(
        Server::setup_networking(3000, None, Environment::Test, &HashMap::new()),
        Err(ServerError::Configuration(message)) if message.contains("valid u16")
    ));
    environment.set("PORT", "3000");
    environment.set("HOST", "not a host");
    assert!(matches!(
        Server::setup_networking(3000, None, Environment::Test, &HashMap::new()),
        Err(ServerError::InvalidAddress { .. })
    ));
}

#[tokio::test]
async fn environment_resolution_obeys_process_dotenv_and_config_precedence() {
    let _lock = crate::server::TEST_ENV_LOCK.lock().await;
    let environment = EnvironmentGuard::clear(&["RULLST_ENV", "APP_ENV"]);
    let mut config = RullstConfig::new();
    config.app.env = Some("test".to_string());
    assert_eq!(
        resolve_environment(&config, &HashMap::new()).unwrap(),
        Environment::Test
    );

    let dotenv = HashMap::from([("APP_ENV".to_string(), "staging".to_string())]);
    assert_eq!(
        resolve_environment(&config, &dotenv).unwrap(),
        Environment::Staging
    );
    environment.set("APP_ENV", "production");
    assert_eq!(
        resolve_environment(&config, &dotenv).unwrap(),
        Environment::Production
    );
    environment.set("RULLST_ENV", "development");
    assert_eq!(
        resolve_environment(&config, &dotenv).unwrap(),
        Environment::Development
    );
    environment.set("RULLST_ENV", "invalid");
    assert!(matches!(
        resolve_environment(&config, &dotenv),
        Err(ServerError::Configuration(_))
    ));
}

#[cfg(unix)]
#[tokio::test]
async fn non_unicode_environment_values_fail_closed() {
    use std::os::unix::ffi::OsStringExt;

    let _lock = crate::server::TEST_ENV_LOCK.lock().await;
    let environment = EnvironmentGuard::clear(&["RULLST_NON_UNICODE_TEST"]);
    environment.set(
        "RULLST_NON_UNICODE_TEST",
        std::ffi::OsString::from_vec(vec![0xff]),
    );
    assert!(matches!(
        read_optional_environment_variable("RULLST_NON_UNICODE_TEST"),
        Err(ServerError::Configuration(message)) if message.contains("not valid Unicode")
    ));
}

#[tokio::test]
async fn scheduler_shield_and_missing_hot_library_have_typed_lifecycles() {
    let mut scheduled = Server::new(Router::new()).schedule(Scheduler::new());
    let handle = scheduled
        .start_scheduler()
        .expect("scheduler start")
        .expect("scheduler handle");
    handle.shutdown().await.expect("scheduler shutdown");
    assert!(scheduled.start_scheduler().unwrap().is_none());

    let shield = crate::resilience::TrafficShield::new(
        crate::resilience::TrafficShieldConfig::new().with_db_probe(false),
    );
    let shielded = Server::new(Router::new()).shield(shield);
    let lifecycle = shielded
        .start_traffic_shield()
        .expect("shield start")
        .expect("shield lifecycle");
    lifecycle.shutdown();
    assert!(
        Server::new(Router::new())
            .start_traffic_shield()
            .unwrap()
            .is_none()
    );

    let _lock = crate::server::TEST_ENV_LOCK.lock().await;
    let environment = EnvironmentGuard::clear(&["RULLST_HMR_TOKEN"]);
    environment.set(
        "RULLST_HMR_TOKEN",
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    );
    let error = Server::new_hot("/definitely/missing/rullst-app")
        .run_hot_reload(
            "/definitely/missing/rullst-app".to_string(),
            "127.0.0.1:0".parse().unwrap(),
            Environment::Development,
            std::future::ready(()),
        )
        .await
        .unwrap_err();
    if cfg!(debug_assertions) {
        assert!(
            matches!(&error, ServerError::HotReload(_)),
            "debug hot reload should reach the missing-library boundary, got {error:?}"
        );
    } else {
        assert!(
            matches!(&error, ServerError::HotReloadDisabled),
            "release builds must reject hot reload before loading a library, got {error:?}"
        );
    }
}

#[test]
fn server_accepts_a_shared_application_lifecycle() {
    let lifecycle = crate::lifecycle::ApplicationLifecycle::new();
    let server = Server::new(Router::new()).with_lifecycle(lifecycle.clone());
    assert!(server.lifecycle.is_some());
    assert_eq!(
        lifecycle.phase(),
        crate::lifecycle::ApplicationPhase::Starting
    );
}

#[tokio::test]
async fn lifecycle_helpers_are_monotonic_and_startup_failure_stops() {
    let lifecycle = crate::lifecycle::ApplicationLifecycle::new();
    mark_lifecycle_ready(Some(&lifecycle)).expect("ready transition");
    assert_eq!(lifecycle.phase(), crate::lifecycle::ApplicationPhase::Ready);

    lifecycle.begin_draining().expect("drain transition");
    assert!(matches!(
        mark_lifecycle_ready(Some(&lifecycle)),
        Err(ServerError::Lifecycle(_))
    ));
    mark_lifecycle_stopped(Some(&lifecycle));
    assert_eq!(
        lifecycle.phase(),
        crate::lifecycle::ApplicationPhase::Stopped
    );

    let failed = crate::lifecycle::ApplicationLifecycle::new();
    failed.begin_draining().expect("pre-start drain");
    let result = Server::new(Router::new())
        .with_lifecycle(failed.clone())
        .run(0)
        .await;
    assert!(matches!(result, Err(ServerError::Lifecycle(_))));
    assert_eq!(failed.phase(), crate::lifecycle::ApplicationPhase::Stopped);
}

#[tokio::test]
async fn custom_shutdown_drives_ready_drain_and_stopped_phases() {
    let _lock = crate::server::TEST_ENV_LOCK.lock().await;
    let environment = EnvironmentGuard::clear(&[
        "HOST",
        "RULLST_HOST",
        "PORT",
        "RULLST_ENV",
        "APP_ENV",
        "DATABASE_URL",
    ]);
    environment.set("RULLST_ENV", "test");

    let lifecycle = crate::lifecycle::ApplicationLifecycle::new();
    let observed = lifecycle.clone();
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        Server::new(Router::new())
            .with_lifecycle(lifecycle)
            .run_with_shutdown(0, async move {
                let _ = shutdown_rx.await;
            })
            .await
    });

    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while observed.phase() != crate::lifecycle::ApplicationPhase::Ready {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("server became ready");
    shutdown_tx.send(()).expect("shutdown receiver alive");
    tokio::time::timeout(std::time::Duration::from_secs(2), server)
        .await
        .expect("server stopped before deadline")
        .expect("server task joined")
        .expect("server shutdown succeeded");
    assert_eq!(
        observed.phase(),
        crate::lifecycle::ApplicationPhase::Stopped
    );
}

#[tokio::test]
async fn hot_reload_token_configuration_fails_closed() {
    let _lock = crate::server::TEST_ENV_LOCK.lock().await;
    let environment = EnvironmentGuard::clear(&["RULLST_HMR_TOKEN"]);
    assert!(matches!(
        resolve_hot_reload_token(),
        Err(ServerError::HotReloadConfiguration(message)) if message.contains("missing")
    ));

    environment.set("RULLST_HMR_TOKEN", "too-short");
    assert!(matches!(
        resolve_hot_reload_token(),
        Err(ServerError::HotReloadConfiguration(message)) if message.contains("64 hexadecimal")
    ));

    environment.set(
        "RULLST_HMR_TOKEN",
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    );
    assert_eq!(
        resolve_hot_reload_token().unwrap().as_ref(),
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    );
}

#[cfg(not(feature = "orm"))]
#[tokio::test]
async fn database_configuration_fails_closed_without_the_orm_feature() {
    let config = RullstConfig::new();
    let mut server = Server::new(Router::new());
    server
        .init_database(&config, &HashMap::new())
        .await
        .unwrap();

    let mut requested = Server::new(Router::new()).with_db("sqlite::memory:");
    assert!(matches!(
        requested.init_database(&config, &HashMap::new()).await,
        Err(ServerError::Database(_))
    ));
}
