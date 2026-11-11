//! `[app] access_log_paths`: which request path the access log of
//! [`crate::Server`] writes.

use super::Environment;
use serde::Deserialize;

/// The request path that the `[HTTP] METHOD path -> status (ms)` access log
/// of [`crate::Server`] writes.
///
/// A path can carry a secret, such as the token of a sign-in or booking link
/// (`/booking/3f9a…`), and the access log goes wherever stdout goes. Set it
/// with `[app] access_log_paths = "route"` (or `"full"`, `"auto"`) in
/// `Rullst.toml`, or with [`crate::Server::access_log_paths`], which takes
/// precedence. The query string is never written.
///
/// Added in 13.0.0-alpha.5.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum AccessLogPaths {
    /// [`AccessLogPaths::Route`] in Staging and Production, and
    /// [`AccessLogPaths::Full`] in Development and Test. The default.
    #[default]
    Auto,
    /// The route that matched, as declared (`/booking/{token}`). A request
    /// that matched no route (a `404`, or a path under a nested service)
    /// writes its path cut to 128 bytes, ending in `…` when cut, with every
    /// byte outside visible ASCII written as `%XX`.
    Route,
    /// The path as the client sent it.
    Full,
}

impl AccessLogPaths {
    /// Whether the access log writes the matched route in `environment`.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub(crate) const fn writes_route(self, environment: Environment) -> bool {
        match self {
            Self::Auto => environment.requires_secure_defaults(),
            Self::Route => true,
            Self::Full => false,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn auto_writes_routes_only_in_staging_and_production() {
        for (environment, route) in [
            (Environment::Development, false),
            (Environment::Test, false),
            (Environment::Staging, true),
            (Environment::Production, true),
        ] {
            assert_eq!(
                AccessLogPaths::Auto.writes_route(environment),
                route,
                "{environment}"
            );
            assert!(AccessLogPaths::Route.writes_route(environment));
            assert!(!AccessLogPaths::Full.writes_route(environment));
        }
        assert_eq!(AccessLogPaths::default(), AccessLogPaths::Auto);
    }

    #[test]
    fn rullst_toml_selects_the_paths_by_name() {
        let parse = |toml: &str| crate::config::RullstConfig::from_toml(toml);
        assert_eq!(
            parse("").unwrap().app.access_log_paths,
            AccessLogPaths::Auto
        );
        for (name, paths) in [
            ("auto", AccessLogPaths::Auto),
            ("route", AccessLogPaths::Route),
            ("full", AccessLogPaths::Full),
        ] {
            let config = parse(&format!("[app]\naccess_log_paths = \"{name}\"\n")).unwrap();
            assert_eq!(config.app.access_log_paths, paths, "{name}");
        }
        assert!(matches!(
            parse("[app]\naccess_log_paths = \"template\"\n"),
            Err(crate::config::ConfigError::Parse(_))
        ));
    }
}
