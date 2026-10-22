//! Digital Asset Links for an Android app that shows this site in a Trusted
//! Web Activity, the Google Play package of a Progressive Web App (built, for
//! example, with Bubblewrap).
//!
//! Android hides the browser's address bar only when
//! `https://<host>/.well-known/assetlinks.json` names the app's package and
//! the SHA-256 fingerprint of the certificate that signs the installed app.
//! With Play App Signing that is the app signing key shown in Play Console,
//! not only the upload key you sign bundles with.
//!
//! ```rust
//! use rullst_core::pwa::{AndroidApp, AssetLinks};
//!
//! let fingerprint = "14:6D:E9:83:C5:73:06:50:D8:EE:B9:95:2F:34:FC:64:16:A0:83:42:E6:1D:BE:A8:8A:04:96:B2:3F:CF:44:E5";
//! let links = AssetLinks::new().with_app(AndroidApp::new("com.acme.notes", [fingerprint])?);
//! let routes = links.router(); // GET /.well-known/assetlinks.json
//! # let _ = routes;
//! # Ok::<(), rullst_core::pwa::AssetLinksError>(())
//! ```
//!
//! [`AssetLinks::from_project`] reads the same links from the environment or
//! `Rullst.toml`, which is how `cargo rullst make:pwa` wires them. Added in
//! 13.0.0-alpha.3.

use axum::{
    http::{HeaderValue, header},
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Deserialize;
use std::path::Path;

/// The variable [`AssetLinks::from_project`] reads first:
/// `package=FINGERPRINT[,FINGERPRINT…]`, with `;` between apps.
pub const ASSET_LINKS_ENV: &str = "RULLST_ANDROID_ASSET_LINKS";

/// The path Android verifies.
pub const ASSET_LINKS_PATH: &str = "/.well-known/assetlinks.json";

/// Why asset links could not be built. Package names and fingerprints are
/// public values, so they may appear in messages; file content never does.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum AssetLinksError {
    /// The package name is not an Android application ID such as
    /// `com.acme.notes`.
    #[error("`{0}` is not an Android package name such as com.acme.notes")]
    InvalidPackageName(String),
    /// A fingerprint is not 32 colon-separated hexadecimal bytes.
    #[error(
        "SHA-256 certificate fingerprint {position} of `{package}` is not 32 colon-separated hexadecimal bytes"
    )]
    InvalidFingerprint {
        /// The app whose fingerprint is invalid.
        package: String,
        /// Its position in the app's list, from 1.
        position: usize,
    },
    /// The app lists no fingerprint.
    #[error("`{0}` lists no SHA-256 certificate fingerprint")]
    MissingFingerprint(String),
    /// The environment, `.env` or `Rullst.toml` could not be read or parsed.
    #[error("invalid Android asset links configuration: {0}")]
    Configuration(String),
}

/// One Android app allowed to open this site full screen.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct AndroidApp {
    package_name: String,
    sha256_cert_fingerprints: Vec<String>,
}

impl AndroidApp {
    /// Validates `package_name` (dot-separated segments that start with a
    /// letter and hold letters, digits or `_`, at least two) and each
    /// fingerprint (`AB:CD:…`, 32 hexadecimal bytes, stored in upper case).
    pub fn new(
        package_name: impl Into<String>,
        sha256_cert_fingerprints: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<Self, AssetLinksError> {
        let package_name = package_name.into();
        if !valid_package_name(&package_name) {
            return Err(AssetLinksError::InvalidPackageName(package_name));
        }
        let mut fingerprints = Vec::new();
        for (index, fingerprint) in sha256_cert_fingerprints.into_iter().enumerate() {
            let fingerprint = fingerprint.into();
            if !valid_fingerprint(&fingerprint) {
                return Err(AssetLinksError::InvalidFingerprint {
                    package: package_name,
                    position: index + 1,
                });
            }
            fingerprints.push(fingerprint.to_ascii_uppercase());
        }
        if fingerprints.is_empty() {
            return Err(AssetLinksError::MissingFingerprint(package_name));
        }
        Ok(Self {
            package_name,
            sha256_cert_fingerprints: fingerprints,
        })
    }

    /// The package name, such as `com.acme.notes`.
    pub fn package_name(&self) -> &str {
        &self.package_name
    }

    /// The fingerprints, in upper case.
    pub fn sha256_cert_fingerprints(&self) -> &[String] {
        &self.sha256_cert_fingerprints
    }
}

fn valid_package_name(value: &str) -> bool {
    let segments = value.split('.').collect::<Vec<_>>();
    value.len() <= 255
        && segments.len() >= 2
        && segments.iter().all(|segment| {
            let mut characters = segment.chars();
            characters
                .next()
                .is_some_and(|first| first.is_ascii_alphabetic())
                && characters.all(|character| character.is_ascii_alphanumeric() || character == '_')
        })
}

fn valid_fingerprint(value: &str) -> bool {
    let bytes = value.split(':').collect::<Vec<_>>();
    bytes.len() == 32
        && bytes
            .iter()
            .all(|byte| byte.len() == 2 && byte.bytes().all(|digit| digit.is_ascii_hexdigit()))
}

/// The Android apps that `/.well-known/assetlinks.json` names.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AssetLinks {
    apps: Vec<AndroidApp>,
}

#[derive(Deserialize)]
struct ProjectFile {
    #[serde(default)]
    android: AndroidSection,
}

#[derive(Default, Deserialize)]
struct AndroidSection {
    #[serde(default)]
    asset_links: Vec<AppEntry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AppEntry {
    package_name: String,
    sha256_cert_fingerprints: Vec<String>,
}

impl AssetLinks {
    /// No app: [`Self::router`] then serves nothing.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `app`.
    #[must_use]
    pub fn with_app(mut self, app: AndroidApp) -> Self {
        self.apps.push(app);
        self
    }

    /// The apps, in the order they were added.
    pub fn apps(&self) -> &[AndroidApp] {
        &self.apps
    }

    /// Whether no app is configured.
    pub fn is_empty(&self) -> bool {
        self.apps.is_empty()
    }

    /// Parses the [`ASSET_LINKS_ENV`] format: `package=FP[,FP…]`, with `;`
    /// between apps; spaces around the parts are ignored.
    pub fn parse(value: &str) -> Result<Self, AssetLinksError> {
        let mut links = Self::new();
        for (index, entry) in value.split(';').map(str::trim).enumerate() {
            if entry.is_empty() {
                continue;
            }
            let (package, fingerprints) = entry.split_once('=').ok_or_else(|| {
                AssetLinksError::Configuration(format!(
                    "entry {} of {ASSET_LINKS_ENV} has no `=` between package and fingerprints",
                    index + 1
                ))
            })?;
            let fingerprints = fingerprints
                .split(',')
                .map(str::trim)
                .filter(|fingerprint| !fingerprint.is_empty());
            links = links.with_app(AndroidApp::new(package.trim(), fingerprints)?);
        }
        Ok(links)
    }

    /// Reads the `[[android.asset_links]]` tables of a `Rullst.toml`
    /// document; other tables are ignored.
    ///
    /// ```toml
    /// [[android.asset_links]]
    /// package_name = "com.acme.notes"
    /// sha256_cert_fingerprints = ["14:6D:E9:…:44:E5"]
    /// ```
    pub fn from_toml(content: &str) -> Result<Self, AssetLinksError> {
        let file = toml::from_str::<ProjectFile>(content).map_err(|error| {
            // Only the position: the parser's message quotes the source line,
            // which may hold a secret such as a database URL.
            let before = content
                .get(..error.span().unwrap_or_default().start)
                .unwrap_or(content);
            let column = before
                .rsplit('\n')
                .next()
                .map_or(0, |line| line.chars().count());
            AssetLinksError::Configuration(format!(
                "Rullst.toml is invalid at line {}, column {}",
                before.matches('\n').count() + 1,
                column + 1
            ))
        })?;
        file.android
            .asset_links
            .into_iter()
            .try_fold(Self::new(), |links, entry| {
                Ok(links.with_app(AndroidApp::new(
                    entry.package_name,
                    entry.sha256_cert_fingerprints,
                )?))
            })
    }

    /// The links of the application in the current directory, with the
    /// precedence `Server` gives its settings: [`ASSET_LINKS_ENV`] from the
    /// process environment, then from `.env`, then `[[android.asset_links]]`
    /// in `Rullst.toml`. Nothing configured is an empty set.
    pub fn from_project() -> Result<Self, AssetLinksError> {
        Self::from_project_in(Path::new("."))
    }

    pub(crate) fn from_project_in(directory: &Path) -> Result<Self, AssetLinksError> {
        let setting =
            crate::config::project_setting::project_setting_in(directory, ASSET_LINKS_ENV)
                .map_err(|error| AssetLinksError::Configuration(error.to_string()))?;
        if let Some(value) = setting.filter(|value| !value.trim().is_empty()) {
            return Self::parse(&value);
        }
        match std::fs::read_to_string(directory.join("Rullst.toml")) {
            Ok(content) => Self::from_toml(&content),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::new()),
            Err(error) => Err(AssetLinksError::Configuration(format!(
                "Rullst.toml could not be read: {}",
                error.kind()
            ))),
        }
    }

    /// The statement list, one `delegate_permission/common.handle_all_urls`
    /// statement per app. Package names and fingerprints hold no character
    /// that JSON escapes, so they are written as validated.
    pub fn to_json(&self) -> String {
        let statements = self
            .apps
            .iter()
            .map(|app| {
                let fingerprints = app
                    .sha256_cert_fingerprints
                    .iter()
                    .map(|fingerprint| format!("\"{fingerprint}\""))
                    .collect::<Vec<_>>()
                    .join(",");
                format!(
                    "{{\"relation\":[\"delegate_permission/common.handle_all_urls\"],\"target\":{{\"namespace\":\"android_app\",\"package_name\":\"{}\",\"sha256_cert_fingerprints\":[{fingerprints}]}}}}",
                    app.package_name
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        format!("[{statements}]")
    }

    /// `GET /.well-known/assetlinks.json` with [`Self::to_json`] as
    /// `application/json`, cacheable for an hour (`public, max-age=3600`).
    /// Without an app the router has no route, so the path answers 404.
    pub fn router(&self) -> axum::Router {
        if self.apps.is_empty() {
            return axum::Router::new();
        }
        let body = self.to_json();
        axum::Router::new().route(
            ASSET_LINKS_PATH,
            get(move || {
                let body = body.clone();
                async move { statement_list(body) }
            }),
        )
    }
}

fn statement_list(body: String) -> Response {
    (
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/json"),
            ),
            (
                header::CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=3600"),
            ),
        ],
        body,
    )
        .into_response()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "pwa_tests.rs"]
mod tests;
