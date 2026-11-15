//! `[security]` settings added in v13: HSTS, WAF body exemptions and the CSRF
//! static-file prefixes and paths.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::field_reassign_with_default
)]

use super::super::{ConfigError, RullstConfig, SecurityConfig};

fn hsts(max_age: u64, include_subdomains: bool, preload: bool) -> SecurityConfig {
    let mut config = SecurityConfig::default();
    config.hsts_max_age_seconds = max_age;
    config.hsts_include_subdomains = include_subdomains;
    config.hsts_preload = preload;
    config
}

#[test]
fn hsts_defaults_to_one_year_with_subdomains_and_without_preload() {
    for config in [
        SecurityConfig::default(),
        RullstConfig::from_toml("").unwrap().security,
        RullstConfig::from_toml("[security]\ncsrf_same_site = \"Lax\"\n")
            .unwrap()
            .security,
    ] {
        assert_eq!(config.hsts_max_age_seconds, 31_536_000);
        assert!(config.hsts_include_subdomains);
        assert!(!config.hsts_preload);
    }
}

#[test]
fn hsts_settings_are_read_from_the_security_table() {
    let config = RullstConfig::from_toml(
        "[security]\nhsts_max_age_seconds = 63072000\nhsts_include_subdomains = false\nhsts_preload = true\n",
    )
    .unwrap()
    .security;
    assert_eq!(config.hsts_max_age_seconds, 63_072_000);
    assert!(!config.hsts_include_subdomains);
    assert!(config.hsts_preload);
}

#[test]
fn hsts_preload_needs_subdomains_and_one_year() {
    for (max_age, include_subdomains, preload) in [
        (31_536_000, true, true),
        (63_072_000, true, true),
        (0, false, false),
        (86_400, true, false),
    ] {
        assert_eq!(
            hsts(max_age, include_subdomains, preload).validate(),
            Ok(()),
            "max-age={max_age} includeSubDomains={include_subdomains} preload={preload}"
        );
    }
    for (max_age, include_subdomains) in [(31_535_999, true), (63_072_000, false), (0, false)] {
        let error = hsts(max_age, include_subdomains, true)
            .validate()
            .unwrap_err();
        assert_eq!(
            error,
            ConfigError::InvalidSecurityConfiguration(
                "hsts_preload requires hsts_include_subdomains = true and hsts_max_age_seconds of at least 31536000 (one year)".to_string()
            ),
            "max-age={max_age} includeSubDomains={include_subdomains}"
        );
    }
}

#[test]
fn waf_body_exemptions_must_be_unique_exact_paths() {
    let mut config = SecurityConfig::default();
    assert!(config.waf_body_exempt_paths.is_empty());
    config.waf_body_exempt_paths = vec!["/lessons/answers".to_owned(), "/snippets".to_owned()];
    assert_eq!(config.validate(), Ok(()));

    for invalid in [
        "/",
        "lessons/answers",
        "/lessons/:id",
        "/lessons/{id}",
        "/lessons/}",
        "/lessons/*rest",
        "/lessons/answers?draft=1",
        "/lessons/answers#top",
        "/lessons/../admin",
        "/lessons/./answers",
    ] {
        config.waf_body_exempt_paths = vec![invalid.to_owned()];
        assert_eq!(
            config.validate(),
            Err(ConfigError::InvalidSecurityConfiguration(format!(
                "WAF body exemption `{invalid}` must be an exact absolute path"
            ))),
        );
    }
    config.waf_body_exempt_paths = vec!["/snippets".to_owned(), "/snippets".to_owned()];
    assert_eq!(
        config.validate(),
        Err(ConfigError::InvalidSecurityConfiguration(
            "duplicate WAF body exemption `/snippets`".to_string()
        )),
    );
    // The webhook list keeps its own wording.
    config.waf_body_exempt_paths.clear();
    config.csrf_signed_webhook_paths = vec!["/hooks/*".to_owned()];
    assert_eq!(
        config.validate(),
        Err(ConfigError::InvalidSecurityConfiguration(
            "CSRF signed-webhook exemption `/hooks/*` must be an exact absolute path".to_string()
        )),
    );
}

#[test]
fn waf_body_exemptions_are_read_from_the_security_table() {
    let config =
        RullstConfig::from_toml("[security]\nwaf_body_exempt_paths = [\"/lessons/answers\"]\n")
            .unwrap()
            .security;
    assert_eq!(config.waf_body_exempt_paths, ["/lessons/answers"]);
}

#[test]
fn csrf_static_files_are_read_from_the_security_table() {
    let defaults = RullstConfig::from_toml("[security]\ncsrf_same_site = \"Lax\"\n")
        .unwrap()
        .security;
    assert_eq!(defaults.csrf_static_prefixes, ["/static/", "/.well-known/"]);
    assert_eq!(defaults.csrf_static_paths.len(), 5);
    assert_eq!(defaults.validate(), Ok(()));

    let config = RullstConfig::from_toml(
        "[security]\ncsrf_static_prefixes = [\"/assets/\"]\ncsrf_static_paths = []\n",
    )
    .unwrap()
    .security;
    assert_eq!(config.csrf_static_prefixes, ["/assets/"]);
    assert!(config.csrf_static_paths.is_empty());
    assert_eq!(config.validate(), Ok(()));
}

#[test]
fn csrf_static_prefixes_must_be_unique_paths_ending_with_a_slash() {
    let config = SecurityConfig::default()
        .with_csrf_static_prefix("/assets/")
        .with_csrf_static_prefix("/a/b/");
    assert_eq!(config.validate(), Ok(()));

    for invalid in [
        "/",
        "",
        "//",
        "/assets",
        "assets/",
        "/assets/*/",
        "/{tenant}/",
        "/../",
        "/assets/./",
        "/assets?x/",
    ] {
        let mut config = SecurityConfig::default();
        config.csrf_static_prefixes = vec![invalid.to_owned()];
        assert_eq!(
            config.validate(),
            Err(ConfigError::InvalidSecurityConfiguration(format!(
                "CSRF static prefix `{invalid}` must be an absolute path that ends with `/`, such as `/assets/`"
            ))),
            "{invalid}"
        );
    }
    let config = SecurityConfig::default().with_csrf_static_prefix("/static/");
    assert_eq!(
        config.validate(),
        Err(ConfigError::InvalidSecurityConfiguration(
            "duplicate CSRF static prefix `/static/`".to_string()
        )),
    );
}

#[test]
fn csrf_static_paths_must_be_unique_exact_paths() {
    for invalid in ["/", "humans.txt", "/files/*", "/files/{name}"] {
        let config = SecurityConfig::default().with_csrf_static_path(invalid);
        assert_eq!(
            config.validate(),
            Err(ConfigError::InvalidSecurityConfiguration(format!(
                "CSRF static path `{invalid}` must be an exact absolute path"
            ))),
            "{invalid}"
        );
    }
    let config = SecurityConfig::default().with_csrf_static_path("/robots.txt");
    assert_eq!(
        config.validate(),
        Err(ConfigError::InvalidSecurityConfiguration(
            "duplicate CSRF static path `/robots.txt`".to_string()
        )),
    );
}
