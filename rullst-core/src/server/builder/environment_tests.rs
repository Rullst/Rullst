#![allow(clippy::unwrap_used)]

use super::insecure_listen_warning;
use crate::config::Environment;
use std::net::IpAddr;

const DEVELOPMENT_WARNING: &str = "Rullst is serving a non-loopback address as development: the WAF only logs what it would refuse and HSTS is not sent over plain HTTP. Set RULLST_ENV=production (or staging) for a deployment; without RULLST_ENV, APP_ENV or [app].env, Rullst runs as development.";
const TEST_WARNING: &str = "Rullst is serving a non-loopback address as test: the WAF is off and HSTS is not sent over plain HTTP. Set RULLST_ENV=production (or staging) for a deployment.";

#[test]
fn only_development_and_test_on_a_non_loopback_address_are_warned() {
    let ip = |value: &str| value.parse::<IpAddr>().unwrap();
    for (environment, address, expected) in [
        (
            Environment::Development,
            "0.0.0.0",
            Some(DEVELOPMENT_WARNING),
        ),
        (Environment::Development, "::", Some(DEVELOPMENT_WARNING)),
        (
            Environment::Development,
            "192.0.2.10",
            Some(DEVELOPMENT_WARNING),
        ),
        (Environment::Test, "0.0.0.0", Some(TEST_WARNING)),
        (Environment::Development, "127.0.0.1", None),
        (Environment::Development, "127.0.0.2", None),
        (Environment::Development, "::1", None),
        (Environment::Development, "::ffff:127.0.0.1", None),
        (Environment::Test, "127.0.0.1", None),
        (Environment::Staging, "0.0.0.0", None),
        (Environment::Production, "0.0.0.0", None),
        (Environment::Production, "::", None),
    ] {
        assert_eq!(
            insecure_listen_warning(environment, ip(address)),
            expected,
            "{environment} {address}"
        );
    }
}
