//! Manifest and `Rullst.toml` rules for 13.0.0-alpha.6: the session
//! registry and `jsonwebtoken` facts that Rust rules depend on,
//! `ed25519-dalek` 2 beside Rullst's 3 (`V13-ED25519-DALEK-3`), SQLite
//! without a journal-mode setting (`V13-SQLITE-WAL-AVAILABLE`) and the static
//! file lists `V13-CSRF-STATIC-PREFIXES` compares paths with.

use super::super::alpha6_defaults::StaticPaths;
use super::super::catalog::ED25519_DALEK_3;
use super::super::{SourceFinding, WorkspaceFacts};
use super::features::{feature_reference, has_feature};
use super::{dependency_line, finding, line_of, minimum, read_small};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The features of a package that enable the session registry.
fn registry_features(package: &str) -> &'static [&'static str] {
    match package {
        "rullst" => &[
            "auth-session-registry-sqlite",
            "auth-session-registry-postgres",
            "auth-session-registry-mysql",
        ],
        "rullst-auth" => &[
            "session-registry-sqlite",
            "session-registry-postgres",
            "session-registry-mysql",
        ],
        _ => &[],
    }
}

/// The facade features that build `rullst-iot` or `rullst-connect`.
const FACADE_ED25519_FEATURES: [&str; 7] = [
    "iot",
    "iot-ingest",
    "oauth",
    "oauth-sqlite",
    "oauth-postgres",
    "oauth-mysql",
    "oauth-mock",
];

/// Whether `package`, with the features `enables` accepts, builds
/// `ed25519-dalek` through Rullst: `rullst-iot`, `rullst-connect`, or the
/// facade with `iot`, `iot-ingest` or an `oauth` feature.
fn builds_rullst_ed25519(package: &str, enables: impl Fn(&[&str]) -> bool) -> bool {
    match package {
        "rullst-iot" | "rullst-connect" => true,
        "rullst" => enables(FACADE_ED25519_FEATURES.as_slice()),
        _ => false,
    }
}

/// What the manifests of a workspace say about the alpha.6 dependency
/// changes, collected manifest by manifest.
#[derive(Debug, Default)]
pub(super) struct Alpha6Manifests {
    /// Declarations of `ed25519-dalek` 2 or earlier: manifest and line.
    ed25519_below_3: Vec<(PathBuf, usize)>,
    /// A manifest builds Rullst's `ed25519-dalek` 3.
    rullst_ed25519: bool,
}

impl Alpha6Manifests {
    /// Reads one dependency declaration of the manifest at `path`.
    pub(super) fn declaration(
        &mut self,
        (path, text): (&Path, &str),
        (key, package): (&str, &str),
        declaration: &toml::Value,
        facts: &mut WorkspaceFacts,
    ) {
        facts.session_registry |= has_feature(declaration, registry_features(package));
        self.rullst_ed25519 |=
            builds_rullst_ed25519(package, |names| has_feature(declaration, names));
        let major = minimum(declaration).map(|(major, _)| major);
        match package {
            "chrono" => facts.chrono_dependency = true,
            "sqlx" => facts.sqlx_chrono |= has_feature(declaration, &["chrono"]),
            "jsonwebtoken" => facts.jsonwebtoken_below_12 |= major.is_some_and(|major| major < 12),
            "ed25519-dalek" if major.is_some_and(|major| major < 3) => self
                .ed25519_below_3
                .push((path.to_path_buf(), dependency_line(text, key))),
            _ => {}
        }
    }

    /// Reads the `[features]` table of a manifest, whose entries can enable
    /// the session registry or Rullst's `ed25519-dalek`.
    pub(super) fn features(
        &mut self,
        document: &toml::Value,
        text: &str,
        packages: &BTreeMap<String, String>,
        facts: &mut WorkspaceFacts,
    ) {
        facts.session_registry |= feature_reference(document, text, packages, |package, name| {
            registry_features(package).contains(&name).then_some(())
        })
        .is_some();
        facts.sqlx_chrono |= feature_reference(document, text, packages, |package, name| {
            (package == "sqlx" && name == "chrono").then_some(())
        })
        .is_some();
        self.rullst_ed25519 |= feature_reference(document, text, packages, |package, name| {
            builds_rullst_ed25519(package, |names| names.contains(&name)).then_some(())
        })
        .is_some();
    }

    /// `V13-ED25519-DALEK-3` at every `ed25519-dalek` 2 declaration, when a
    /// manifest of the workspace also builds Rullst's `ed25519-dalek` 3.
    pub(super) fn findings(self) -> Vec<SourceFinding> {
        if !self.rullst_ed25519 {
            return Vec::new();
        }
        self.ed25519_below_3
            .into_iter()
            .map(|(path, line)| finding(&ED25519_DALEK_3, path, line))
            .collect()
    }
}

/// Whether a database URL selects SQLite.
fn is_sqlite(url: &str) -> bool {
    url.trim()
        .get(..7)
        .is_some_and(|scheme| scheme.eq_ignore_ascii_case("sqlite:"))
}

/// The line of the `url` key of the `[database]` table of `Rullst.toml`.
fn database_url_line(text: &str) -> usize {
    let mut in_database = false;
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.starts_with('[') {
            in_database = line == "[database]";
        } else if in_database
            && line
                .strip_prefix("url")
                .is_some_and(|rest| rest.trim_start().starts_with('='))
        {
            return index + 1;
        }
    }
    1
}

/// The value of a `KEY=value` line of an env file, without quotes or an
/// `export ` prefix, when its key is `key`.
fn env_value<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let line = line.trim();
    let line = line.strip_prefix("export ").unwrap_or(line);
    let (name, value) = line.split_once('=')?;
    (name.trim() == key).then(|| value.trim().trim_matches(['"', '\'']))
}

/// `V13-SQLITE-WAL-AVAILABLE` for a project that uses SQLite and leaves
/// `[database] sqlite_journal_mode` out of `Rullst.toml`: at the `url` line
/// of `Rullst.toml`'s `[database]` when it is a SQLite URL, or else at the
/// `DATABASE_URL` line of `.env` or `.env.example`. A `Rullst.toml` that does
/// not parse is left to the application's own start-up.
pub(super) fn sqlite_without_journal_mode(root: &Path) -> Option<(&'static str, usize)> {
    let rullst_toml = read_small(&root.join("Rullst.toml"));
    let database = match &rullst_toml {
        Some(text) => toml::from_str::<toml::Value>(text)
            .ok()?
            .get("database")
            .cloned(),
        None => None,
    };
    if database
        .as_ref()
        .is_some_and(|database| database.get("sqlite_journal_mode").is_some())
    {
        return None;
    }
    let url = database
        .as_ref()
        .and_then(|database| database.get("url"))
        .and_then(toml::Value::as_str);
    if let (Some(url), Some(text)) = (url, &rullst_toml) {
        return is_sqlite(url).then(|| ("Rullst.toml", database_url_line(text)));
    }
    [".env", ".env.example"].into_iter().find_map(|name| {
        let text = read_small(&root.join(name))?;
        let line = line_of(&text, |line| {
            env_value(line, "DATABASE_URL").is_some_and(is_sqlite)
        })?;
        Some((name, line))
    })
}

/// The `[security]` `csrf_static_prefixes` and `csrf_static_paths` of
/// `root`'s `Rullst.toml`, read as `Server` reads the file
/// ([`RullstConfig::from_toml`], then [`RullstConfig::validate`]): Core's
/// defaults without the file or a key, and a list in the file replaces its
/// default. A file that `Server` refuses keeps the defaults, with the reason.
///
/// [`RullstConfig::from_toml`]: rullst_core::config::RullstConfig::from_toml
/// [`RullstConfig::validate`]: rullst_core::config::RullstConfig::validate
pub(in super::super) fn csrf_static_paths(root: &Path) -> StaticPaths {
    let Some(text) = read_small(&root.join("Rullst.toml")) else {
        return StaticPaths::default();
    };
    match rullst_core::config::RullstConfig::from_toml(&text)
        .and_then(|config| config.validate().map(|()| config))
    {
        Ok(config) => StaticPaths::project(config.security),
        Err(error) => StaticPaths::unread(error),
    }
}
