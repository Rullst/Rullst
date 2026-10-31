//! Feature facts of dependency declarations: SQLx's macros, SQLx's ring
//! TLS features and the Nexus AI assistant, all changed in 13.0.0-alpha.4,
//! and the `[features]` entries that enable a dependency's feature.

use super::{Rule, SQLX_TLS_NATIVE_ROOTS, SQLX_TLS_RING};
use std::collections::BTreeMap;

/// SQLx 0.9 features that make ring its rustls provider, with their rule;
/// SQLx prefers ring whenever any crate enables one, and Rullst's own SQLx
/// dependencies use `tls-rustls-aws-lc-rs` since 13.0.0-alpha.4.
/// `tls-rustls` is an alias of `tls-rustls-ring`, which is
/// `tls-rustls-ring-webpki`, so `tls-rustls-aws-lc-rs` keeps their bundled
/// WebPKI roots. `tls-rustls-ring-native-roots` trusts the operating system's
/// roots instead, which no aws-lc-rs feature of SQLx 0.9 does, so it has a
/// rule of its own.
pub(super) const SQLX_RING_TLS: [(&[&str], &Rule); 2] = [
    (
        &["tls-rustls", "tls-rustls-ring", "tls-rustls-ring-webpki"],
        &SQLX_TLS_RING,
    ),
    (&["tls-rustls-ring-native-roots"], &SQLX_TLS_NATIVE_ROOTS),
];

/// Whether a `sqlx` declaration enables SQLx's derive or query macros:
/// `derive` or `macros`, or the default features, which include `macros`.
pub(super) fn enables_sqlx_macros(declaration: &toml::Value) -> bool {
    let defaults = declaration.as_str().is_some()
        || declaration
            .get("default-features")
            .or_else(|| declaration.get("default_features"))
            .and_then(toml::Value::as_bool)
            .unwrap_or(true);
    defaults || has_feature(declaration, &["derive", "macros"])
}

/// The line of the first `[features]` entry that enables a dependency's
/// feature through its key (`"sqlx/tls-rustls"`, `"sqlx?/tls-rustls"`) and
/// what `select` makes of that package and feature.
pub(super) fn feature_reference<T>(
    document: &toml::Value,
    text: &str,
    packages: &BTreeMap<String, String>,
    select: impl Fn(&str, &str) -> Option<T>,
) -> Option<(usize, T)> {
    let (reference, selected) = document
        .get("features")
        .and_then(toml::Value::as_table)?
        .values()
        .filter_map(toml::Value::as_array)
        .flatten()
        .filter_map(toml::Value::as_str)
        .find_map(|value| {
            let (key, feature) = value.split_once('/')?;
            let key = key.strip_suffix('?').unwrap_or(key);
            select(packages.get(key)?, feature).map(|selected| (value, selected))
        })?;
    let line = super::line_of(text, |line| {
        !line.starts_with('#') && line.contains(reference)
    })
    .unwrap_or(1);
    Some((line, selected))
}

/// Whether a declaration enables one of `names`.
pub(super) fn has_feature(declaration: &toml::Value, names: &[&str]) -> bool {
    declaration
        .get("features")
        .and_then(toml::Value::as_array)
        .is_some_and(|features| {
            features
                .iter()
                .filter_map(toml::Value::as_str)
                .any(|feature| names.contains(&feature))
        })
}

/// A Nexus dependency without the AI assistant it included up to
/// 13.0.0-alpha.3: `rullst` with `nexus`/`nexus-tz` but not `nexus-ai`, or
/// `rullst-nexus` without its default features and without `ai`.
pub(super) fn nexus_without_assistant(package: &str, declaration: &toml::Value) -> bool {
    match package {
        "rullst" => {
            has_feature(declaration, &["nexus", "nexus-tz"])
                && !has_feature(declaration, &["nexus-ai"])
        }
        "rullst-nexus" => {
            let defaults = declaration
                .get("default-features")
                .or_else(|| declaration.get("default_features"))
                .and_then(toml::Value::as_bool)
                .unwrap_or(true);
            !defaults && !has_feature(declaration, &["ai"])
        }
        _ => false,
    }
}
