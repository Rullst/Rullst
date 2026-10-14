//! `[security]` settings added in v13: HSTS.

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
