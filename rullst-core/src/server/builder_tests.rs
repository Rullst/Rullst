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
        )
        .await
        .unwrap_err();
    assert!(matches!(error, ServerError::HotReload(_)));
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
