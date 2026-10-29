//! Feature facts of dependency declarations: SQLx's macros and the Nexus
//! AI assistant, both changed in 13.0.0-alpha.4.

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
