//! The static-file paths for which the CSRF middleware never mints a token:
//! `[security]` `csrf_static_prefixes` and `csrf_static_paths`.

use super::SecurityConfig;

/// The prefixes that `Server` and the generated projects serve files under:
/// `static/` and the fixed `/.well-known/` files (such as
/// `assetlinks.json` from `make:pwa`).
pub(super) fn default_prefixes() -> Vec<String> {
    vec!["/static/".to_string(), "/.well-known/".to_string()]
}

/// The fixed files browsers, crawlers and `make:pwa` request at the root.
pub(super) fn default_paths() -> Vec<String> {
    [
        "/favicon.ico",
        "/robots.txt",
        "/sitemap.xml",
        "/manifest.webmanifest",
        "/sw.js",
    ]
    .map(String::from)
    .to_vec()
}

impl SecurityConfig {
    /// Adds a path prefix under which the application serves static files,
    /// such as `"/assets/"`: a `GET` or `HEAD` below it never mints a CSRF
    /// token or sets the `rullst_csrf` cookie. The prefix starts and ends
    /// with `/`; [`SecurityConfig::validate`] rejects any other. Added in
    /// 13.0.0-alpha.6.
    ///
    /// ```
    /// let security = rullst_core::config::SecurityConfig::default()
    ///     .with_csrf_static_prefix("/assets/")
    ///     .with_csrf_static_path("/humans.txt");
    /// assert!(security.is_csrf_static_path("/assets/app.css"));
    /// assert!(security.is_csrf_static_path("/static/app.css"));
    /// assert!(!security.is_csrf_static_path("/report.json"));
    /// ```
    #[must_use]
    pub fn with_csrf_static_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.csrf_static_prefixes.push(prefix.into());
        self
    }

    /// Adds one exact path of a static file served outside the
    /// `csrf_static_prefixes`, such as `"/humans.txt"`, which is then
    /// treated the same way. Added in 13.0.0-alpha.6.
    #[must_use]
    pub fn with_csrf_static_path(mut self, path: impl Into<String>) -> Self {
        self.csrf_static_paths.push(path.into());
        self
    }

    /// Whether `path` is a static file for the CSRF middleware: below one of
    /// the `csrf_static_prefixes` or equal to one of the `csrf_static_paths`.
    /// A `GET` or `HEAD` for such a path never mints a token or sets the
    /// cookie; every other path does, whatever its extension. Added in
    /// 13.0.0-alpha.6.
    pub fn is_csrf_static_path(&self, path: &str) -> bool {
        self.csrf_static_prefixes
            .iter()
            .any(|prefix| path.starts_with(prefix.as_str()))
            || self.csrf_static_paths.iter().any(|exact| exact == path)
    }
}
