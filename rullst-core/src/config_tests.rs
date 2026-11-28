//! Unit tests for configuration parsing and security policy validation.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::field_reassign_with_default
)]

use super::*;

#[tokio::test]
async fn test_global_config_access() {
    let config1 = RullstConfig::global();
    let config2 = RullstConfig::global();
    assert!(
        std::ptr::eq(config1, config2),
        "global() should return the same instance"
    );
    assert_eq!(config1.security.csrf_same_site, "Lax");
}

#[test]
fn parse_errors_report_position_without_configuration_content() {
    let canary = "sk_live_toml_redaction_canary";
    for (content, line) in [
        (
            format!(
                "[app]\nenv = \"production\"\n[database]\nurl = \"postgres://owner:{canary}@db\n"
            ),
            4,
        ),
        (format!("[app]\nport = \"{canary}\"\n"), 2),
        (format!("app_key = \"{canary}\"\n[app\n"), 2),
    ] {
        let error = RullstConfig::from_toml(&content).unwrap_err();
        let rendered = format!("{error} {error:?}");
        assert!(!rendered.contains(canary), "{rendered}");
        assert!(!rendered.contains("postgres://"), "{rendered}");
        assert!(
            matches!(&error, ConfigError::Parse(message) if message.contains(&format!("line {line},"))),
            "{rendered}"
        );
    }
}

#[tokio::test]
async fn test_load_config_from_file() {
    let temp_dir = "test_config_dir";
    let _ = std::fs::create_dir_all(temp_dir);
    let path = format!("{}/Rullst.toml", temp_dir);

    let toml_content = r#"
[app]
env = "production"
port = 8080

[database]
url = "sqlite::memory:"

[security]
csrf_same_site = "Strict"
cors_allow_origins = ["https://example.com"]
"#;
    tokio::fs::write(&path, toml_content).await.unwrap();

    let config = RullstConfig::load_from_file(&path).await.unwrap();

    assert_eq!(config.app.env.unwrap(), "production");
    assert_eq!(config.app.port.unwrap(), 8080);
    assert_eq!(config.database.url.unwrap(), "sqlite::memory:");
    assert_eq!(config.security.csrf_same_site, "Strict");
    assert_eq!(config.security.cors_allow_origins.len(), 1);
    assert_eq!(config.security.cors_allow_origins[0], "https://example.com");
    assert!(!config.security.cors_allow_credentials);

    let _ = std::fs::remove_dir_all(temp_dir);
}

#[test]
fn test_default_security_config() {
    let config = SecurityConfig::default();
    assert_eq!(config.csrf_same_site, "Lax");
    assert_eq!(config.coep, "require-corp");
    assert!(config.csp.contains("default-src"));
    assert!(!config.csp.contains("unsafe-inline"));
    assert!(!config.csp.contains("unsafe-eval"));
    assert!(config.user_agent_blocklist.contains(&"gptbot".to_string()));
    assert!(config.csrf_signed_webhook_paths.is_empty());
}

#[test]
fn test_set_global_config() {
    let mut config = RullstConfig::new();
    config.app.env = Some("test_env".to_string());
    let result = RullstConfig::set_global(config);
    match result {
        Ok(_) => assert_eq!(RullstConfig::global().app.env.as_deref(), Some("test_env")),
        Err(c) => assert_eq!(c.app.env.as_deref(), Some("test_env")),
    }
}

#[test]
fn database_url_debug_output_is_redacted() {
    let mut config = RullstConfig::default();
    config.database.url = Some("postgres://app:S3cr3t@db.internal/prod?sslmode=require".into());
    let debug = format!("{config:?} {:?}", config.database);
    assert!(!debug.contains("S3cr3t"), "{debug}");
    assert!(!debug.contains("db.internal"), "{debug}");
    assert!(debug.contains("postgres://<redacted>"), "{debug}");

    assert_eq!(redacted_url("app:S3cr3t@db/prod"), "<redacted>");
    assert_eq!(redacted_url("sqlite://rullst.db"), "sqlite://<redacted>");
    assert_eq!(redacted_url("a b://x"), "<redacted>");
    assert!(format!("{:?}", DatabaseConfig::default()).contains("url: None"));
}

#[cfg(feature = "orm")]
#[test]
fn database_table_chooses_the_sqlite_journal_mode() {
    use rullst_orm::SqliteJournalMode;
    let parse = |toml: &str| RullstConfig::from_toml(toml).map(|config| config.database);
    let wal = parse("[database]\nurl = \"sqlite://db.sqlite\"\nsqlite_journal_mode = \"wal\"\n")
        .expect("wal");
    assert_eq!(wal.sqlite_journal_mode, Some(SqliteJournalMode::Wal));
    assert!(format!("{wal:?}").contains("sqlite_journal_mode: Some(Wal)"));
    let delete = parse("[database]\nsqlite_journal_mode = \"delete\"\n").expect("delete");
    assert_eq!(delete.sqlite_journal_mode, Some(SqliteJournalMode::Delete));
    let absent = parse("[database]\nurl = \"sqlite://db.sqlite\"\n").expect("absent");
    assert_eq!(absent.sqlite_journal_mode, None);
    assert!(matches!(
        parse("[database]\nsqlite_journal_mode = \"truncate\"\n"),
        Err(ConfigError::Parse(_))
    ));
}

#[test]
fn test_deserialize_security_config_defaults() {
    let config: SecurityConfig = toml::from_str("").unwrap();
    assert!(!config.enable_pii_masking);
    assert_eq!(config.coep, "require-corp");
}

#[test]
fn environment_resolution_has_one_precedence_and_validated_aliases() {
    assert_eq!(
        Environment::resolve(Some("prod"), Some("test"), Some("development")).unwrap(),
        Environment::Production
    );
    assert_eq!(
        Environment::resolve(None, Some("STAGE"), Some("development")).unwrap(),
        Environment::Staging
    );
    assert_eq!(
        Environment::resolve(None, None, Some("testing")).unwrap(),
        Environment::Test
    );
    assert_eq!(
        Environment::resolve(None, None, None).unwrap(),
        Environment::Development
    );
    assert!(Environment::resolve(Some("unknown"), None, None).is_err());
}

#[test]
fn only_development_exposes_developer_tools() {
    assert!(Environment::Development.allows_development_tools());
    assert!(!Environment::Test.allows_development_tools());
    assert!(Environment::Staging.requires_secure_defaults());
    assert!(Environment::Production.requires_secure_defaults());
}

#[test]
fn signed_webhook_csrf_exemptions_must_be_exact_paths() {
    let mut config = SecurityConfig::default();
    config.csrf_signed_webhook_paths = vec!["/billing/webhook".to_owned()];
    assert!(config.validate().is_ok());

    for invalid in [
        "billing/webhook",
        "/billing/:provider",
        "/billing/{provider}",
        "/billing/*path",
        "/billing/../admin",
        "/billing/webhook?provider=x",
    ] {
        config.csrf_signed_webhook_paths = vec![invalid.to_owned()];
        assert!(config.validate().is_err(), "{invalid} must be rejected");
    }

    config.csrf_signed_webhook_paths =
        vec!["/billing/webhook".to_owned(), "/billing/webhook".to_owned()];
    assert!(config.validate().is_err());
}

#[test]
fn browser_security_configuration_is_strict_and_exact() {
    let mut config = SecurityConfig::default();
    config.cors_allow_origins = vec![
        "https://academy.example".to_string(),
        "http://localhost:3000".to_string(),
        "http://[::1]:8080".to_string(),
    ];
    assert!(config.validate().is_ok());

    // Browsers send a lowercase scheme and host and omit the default port;
    // CORS compares bytes, so these spellings could never match.
    for unmatched in [
        "https://Academy.example",
        "HTTPS://academy.example",
        "https://academy.example:443",
        "http://localhost:80",
        "http://localhost:03000",
        "http://[::ABCD]:8080",
    ] {
        config.cors_allow_origins = vec![unmatched.to_string()];
        let error = config.validate().unwrap_err().to_string();
        assert!(
            error.contains("as browsers send it"),
            "{unmatched}: {error}"
        );
    }

    for invalid in [
        "*",
        "academy.example",
        "ftp://academy.example",
        "https://academy.example/",
        "https://academy.example/path",
        "https://user@academy.example",
        "https://academy.example?x=1",
    ] {
        config.cors_allow_origins = vec![invalid.to_string()];
        assert!(config.validate().is_err(), "{invalid} must be rejected");
    }
    config.cors_allow_origins = vec![
        "https://academy.example".to_string(),
        "https://academy.example".to_string(),
    ];
    assert!(config.validate().is_err());
    config.cors_allow_origins.clear();
    config.csrf_same_site = "relaxed".to_string();
    assert!(config.validate().is_err());
    config.csrf_same_site = "Lax".to_string();
    config.csp = "default-src 'self'\r\nx-injected: yes".to_string();
    assert!(config.validate().is_err());
}

#[test]
fn coep_policy_is_explicit_and_closed() {
    let mut config = SecurityConfig::default();
    for policy in ["require-corp", "credentialless", "unsafe-none"] {
        config.coep = policy.to_string();
        assert!(config.validate().is_ok(), "{policy} must be accepted");
    }
    for invalid in ["", "off", "cross-origin", "require-corp\nunsafe-none"] {
        config.coep = invalid.to_string();
        assert!(config.validate().is_err(), "{invalid:?} must be rejected");
    }
}

#[test]
fn empty_user_agent_blocklist_entries_are_rejected() {
    let mut config = SecurityConfig::default();
    for empty in ["", "   ", "\t"] {
        config.user_agent_blocklist = vec!["gptbot".to_string(), empty.to_string()];
        let error = config.validate().unwrap_err().to_string();
        assert!(error.contains("user_agent_blocklist entry 2"), "{error}");
    }
    config.user_agent_blocklist = Vec::new();
    assert!(config.validate().is_ok());
}

#[tokio::test]
async fn environment_includes_the_dotenv_selector_a_server_read() {
    let _lock = crate::server::TEST_ENV_LOCK.lock().await;
    let saved: Vec<_> = ["RULLST_ENV", "APP_ENV"]
        .into_iter()
        .map(|key| (key, std::env::var_os(key)))
        .collect();
    unsafe {
        std::env::remove_var("RULLST_ENV");
        std::env::remove_var("APP_ENV");
    }
    let mut config = RullstConfig::new();
    config.app.env = Some("test".to_string());

    assert_eq!(config.environment().unwrap(), Environment::Test);
    record_project_environment_selector(Some("production".to_string()));
    assert_eq!(config.environment().unwrap(), Environment::Production);
    unsafe { std::env::set_var("APP_ENV", "staging") };
    assert_eq!(config.environment().unwrap(), Environment::Staging);
    record_project_environment_selector(None);
    unsafe { std::env::remove_var("APP_ENV") };
    assert_eq!(config.environment().unwrap(), Environment::Test);

    for (key, value) in saved {
        unsafe {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}

#[test]
fn malformed_cors_origins_fail_the_shape_check_first() {
    let mut config = SecurityConfig::default();
    for invalid in [
        "ftp://academy.example",
        "https://academy.example/path",
        "https://academy.example?x=1",
        "https://academy.example/",
        "https://user@academy.example",
    ] {
        config.cors_allow_origins = vec![invalid.to_string()];
        let error = config.validate().unwrap_err().to_string();
        assert!(
            error.contains("without path, query, credentials, wildcard, or trailing slash"),
            "{invalid}: {error}"
        );
    }
}

#[test]
fn the_root_path_is_not_a_signed_webhook_exemption() {
    let mut config = SecurityConfig::default();
    config.csrf_signed_webhook_paths = vec!["/".to_owned()];
    assert!(config.validate().is_err());
}

#[tokio::test]
async fn detect_prefers_the_process_environment_to_the_configured_value() {
    let _lock = crate::server::TEST_ENV_LOCK.lock().await;
    let saved: Vec<_> = ["RULLST_ENV", "APP_ENV"]
        .into_iter()
        .map(|key| (key, std::env::var_os(key)))
        .collect();
    unsafe {
        std::env::remove_var("RULLST_ENV");
        std::env::remove_var("APP_ENV");
    }
    let configured = Environment::detect(Some("production"));
    unsafe { std::env::set_var("RULLST_ENV", "staging") };
    let from_process = Environment::detect(Some("production"));
    for (key, value) in saved {
        unsafe {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
    assert_eq!(configured.unwrap(), Environment::Production);
    assert_eq!(from_process.unwrap(), Environment::Staging);
    assert_eq!(Environment::Staging.to_string(), "staging");
}

#[test]
fn parse_errors_name_their_line_and_column() {
    match RullstConfig::from_toml("[app]\nname = = 1\n") {
        Err(ConfigError::Parse(message)) => {
            assert_eq!(message, "invalid configuration at line 2, column 8");
        }
        other => panic!("expected a parse error, got {other:?}"),
    }
}

/// A configuration that carries `[app] env`, as a loaded `Rullst.toml` does.
fn production_config() -> RullstConfig {
    RullstConfig::from_toml("[app]\nenv = \"production\"\n").unwrap()
}

#[test]
fn early_global_read_returns_defaults_and_server_publication_replaces_them() {
    static CELL: GlobalCell = std::sync::RwLock::new(None);
    let early = read_global(&CELL);
    assert_eq!(early.app.env, None);
    assert!(std::ptr::eq(early, read_global(&CELL)));

    let outcome = publish_in(&CELL, &production_config());
    assert_eq!(outcome, GlobalOutcome::ReplacedDefaults { differs: true });
    let later = read_global(&CELL);
    assert_eq!(later.app.env.as_deref(), Some("production"));
    // The reference taken before the replacement is the one that is stale.
    assert_eq!(early.app.env, None);
}

#[test]
fn early_read_followed_by_identical_configuration_is_not_a_conflict() {
    static CELL: GlobalCell = std::sync::RwLock::new(None);
    read_global(&CELL);
    assert_eq!(
        publish_in(&CELL, &RullstConfig::default()),
        GlobalOutcome::ReplacedDefaults { differs: false }
    );
}

#[test]
fn explicit_global_is_kept_and_a_different_offer_is_reported() {
    static CELL: GlobalCell = std::sync::RwLock::new(None);
    assert_eq!(
        publish_in(&CELL, &production_config()),
        GlobalOutcome::Installed
    );
    assert_eq!(
        publish_in(&CELL, &RullstConfig::default()),
        GlobalOutcome::Kept { differs: true }
    );
    assert_eq!(
        publish_in(&CELL, &production_config()),
        GlobalOutcome::Kept { differs: false }
    );
    assert_eq!(read_global(&CELL).app.env.as_deref(), Some("production"));
}

#[test]
fn global_configuration_comparison_sees_database_url_and_security_changes() {
    let mut other = RullstConfig::default();
    assert_eq!(other, RullstConfig::default());
    other.database.url = Some("sqlite://other.db".into());
    assert_ne!(other, RullstConfig::default());
    let mut other = RullstConfig::default();
    other.security.hsts_preload = true;
    assert_ne!(other, RullstConfig::default());
}
