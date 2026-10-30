//! Feature facts of dependency declarations: SQLx's macros, SQLx's ring
//! TLS features and the Nexus AI assistant, all changed in 13.0.0-alpha.4,
//! and the `[features]` entries that enable a dependency's feature.

use std::collections::BTreeMap;

/// SQLx 0.9 features that make ring its rustls provider: `tls-rustls` is an
/// alias of `tls-rustls-ring`, which is `tls-rustls-ring-webpki`. Not
/// `tls-rustls-ring-native-roots`: its aws-lc-rs replacement would trust the
/// bundled WebPKI roots instead of the operating system's.
const SQLX_RING_TLS: [&str; 3] = ["tls-rustls", "tls-rustls-ring", "tls-rustls-ring-webpki"];

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

/// Whether a `sqlx` declaration keeps SQLx's database TLS on ring, which
/// SQLx prefers whenever any crate enables it; Rullst's own SQLx
/// dependencies use `tls-rustls-aws-lc-rs` since 13.0.0-alpha.4.
pub(super) fn enables_sqlx_ring_tls(declaration: &toml::Value) -> bool {
    has_feature(declaration, &SQLX_RING_TLS)
}

/// Whether `feature` of `package`, enabled from `[features]`, is a SQLx ring
/// TLS feature.
pub(super) fn is_sqlx_ring_tls(package: &str, feature: &str) -> bool {
    package == "sqlx" && SQLX_RING_TLS.contains(&feature)
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

fn has_feature(declaration: &toml::Value, names: &[&str]) -> bool {
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
