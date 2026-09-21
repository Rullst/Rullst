use super::ContextError;
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct Manifest {
    pub project: Option<String>,
    pub dependencies: Vec<String>,
    pub features: Vec<String>,
    pub requirements: BTreeMap<String, Vec<String>>,
    pub dependency_features: BTreeMap<String, Vec<String>>,
}

fn token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
}

pub(super) fn manifest(bytes: &[u8]) -> Result<Manifest, ContextError> {
    let source = std::str::from_utf8(bytes).map_err(|_| ContextError::Manifest)?;
    let value: toml::Value = toml::from_str(source).map_err(|_| ContextError::Manifest)?;
    if !value.get("package").is_some_and(toml::Value::is_table)
        && !value.get("workspace").is_some_and(toml::Value::is_table)
    {
        return Err(ContextError::Manifest);
    }
    let project = value
        .get("package")
        .and_then(|package| package.get("name"))
        .and_then(toml::Value::as_str)
        .map(str::to_owned);
    if project.as_deref().is_some_and(|value| !token(value)) {
        return Err(ContextError::Manifest);
    }
    // Only named dependency tables; URLs, paths, versions and package metadata
    // are never rendered as arbitrary user strings.
    fn dependencies(
        value: &toml::Value,
        names: &mut BTreeSet<String>,
        requirements: &mut BTreeMap<String, BTreeSet<String>>,
        features: &mut BTreeMap<String, BTreeSet<String>>,
    ) -> Result<(), ContextError> {
        for key in ["dependencies", "dev-dependencies", "build-dependencies"] {
            if let Some(value) = value.get(key) {
                let table = value.as_table().ok_or(ContextError::Manifest)?;
                for (name, declaration) in table {
                    if !token(name) {
                        return Err(ContextError::Manifest);
                    }
                    names.insert(name.clone());
                    if let Some(selected) = declaration.get("features") {
                        let selected = selected.as_array().ok_or(ContextError::Manifest)?;
                        if selected.len() > 256 {
                            return Err(ContextError::Limit);
                        }
                        for value in selected {
                            let feature = value
                                .as_str()
                                .filter(|value| token(value))
                                .ok_or(ContextError::Manifest)?;
                            features
                                .entry(name.clone())
                                .or_default()
                                .insert(feature.to_owned());
                        }
                    }
                    let version = declaration
                        .as_str()
                        .or_else(|| declaration.get("version").and_then(toml::Value::as_str));
                    let requirement = if let Some(version) = version {
                        if version.len() > 128 {
                            return Err(ContextError::Limit);
                        }
                        semver::VersionReq::parse(version)
                            .map_err(|_| ContextError::Manifest)?
                            .to_string()
                    } else if declaration.get("workspace").and_then(toml::Value::as_bool)
                        == Some(true)
                    {
                        "workspace-inherited".into()
                    } else if declaration.get("path").is_some() {
                        "path-without-version".into()
                    } else if declaration.get("git").is_some() {
                        "git-without-version".into()
                    } else {
                        return Err(ContextError::Manifest);
                    };
                    requirements
                        .entry(name.clone())
                        .or_default()
                        .insert(requirement);
                    if names.len() > 256 {
                        return Err(ContextError::Limit);
                    }
                }
            }
        }
        Ok(())
    }
    let mut names = BTreeSet::new();
    let mut requirements = BTreeMap::new();
    let mut dependency_features = BTreeMap::new();
    dependencies(
        &value,
        &mut names,
        &mut requirements,
        &mut dependency_features,
    )?;
    if let Some(workspace) = value.get("workspace") {
        dependencies(
            workspace,
            &mut names,
            &mut requirements,
            &mut dependency_features,
        )?;
    }
    if let Some(targets) = value.get("target").and_then(toml::Value::as_table) {
        for target in targets.values() {
            dependencies(
                target,
                &mut names,
                &mut requirements,
                &mut dependency_features,
            )?;
        }
    }
    let mut features = value
        .get("features")
        .and_then(toml::Value::as_table)
        .map(|table| table.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    if features.len() > 256 || features.iter().any(|name| !token(name)) {
        return Err(ContextError::Manifest);
    }
    features.sort();
    Ok(Manifest {
        project,
        dependencies: names.into_iter().collect(),
        features,
        dependency_features: dependency_features
            .into_iter()
            .map(|(name, values)| (name, values.into_iter().collect()))
            .collect(),
        requirements: requirements
            .into_iter()
            .map(|(name, versions)| (name, versions.into_iter().collect()))
            .collect(),
    })
}

pub(super) fn toml_keys(bytes: &[u8]) -> Result<Vec<String>, ContextError> {
    let source = std::str::from_utf8(bytes).map_err(|_| ContextError::Configuration)?;
    let value: toml::Value = toml::from_str(source).map_err(|_| ContextError::Configuration)?;
    let mut keys = BTreeSet::new();
    fn visit(
        value: &toml::Value,
        prefix: &str,
        depth: usize,
        keys: &mut BTreeSet<String>,
    ) -> Result<(), ContextError> {
        if depth > 16 {
            return Err(ContextError::Limit);
        }
        if let Some(table) = value.as_table() {
            for (key, child) in table {
                if !token(key) {
                    return Err(ContextError::Configuration);
                }
                let key = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                if key.len() > 256 || keys.len() >= 512 {
                    return Err(ContextError::Limit);
                }
                keys.insert(key.clone());
                visit(child, &key, depth + 1, keys)?;
            }
        } else if let Some(array) = value.as_array() {
            for child in array {
                visit(child, prefix, depth + 1, keys)?;
            }
        }
        Ok(())
    }
    visit(&value, "", 0, &mut keys)?;
    Ok(keys.into_iter().collect())
}

pub(super) fn env_keys(bytes: &[u8]) -> Result<Vec<String>, ContextError> {
    let source = std::str::from_utf8(bytes).map_err(|_| ContextError::Configuration)?;
    let mut keys = BTreeSet::new();
    for line in source.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line);
        // This inventory accepts only single-line declarations. Reject an open
        // quote before a continuation could be mistaken for a key and exposed.
        let (name, value) = line.split_once('=').ok_or(ContextError::Configuration)?;
        single_line_value(value)?;
        let name = name.trim();
        if name.len() > 128
            || name.is_empty()
            || name.starts_with(|c: char| c.is_ascii_digit())
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        {
            return Err(ContextError::Configuration);
        }
        keys.insert(name.to_owned());
        if keys.len() > 512 {
            return Err(ContextError::Limit);
        }
    }
    Ok(keys.into_iter().collect())
}

fn single_line_value(value: &str) -> Result<(), ContextError> {
    let mut quote = None;
    let mut escaped = false;
    let mut whitespace = true;
    for character in value.chars() {
        if escaped {
            escaped = false;
            whitespace = false;
            continue;
        }
        if character == '\\' && quote != Some('\'') {
            escaped = true;
        } else if quote == Some(character) {
            quote = None;
        } else if quote.is_none() {
            if character == '#' && whitespace {
                break;
            }
            if matches!(character, '\'' | '"') {
                quote = Some(character);
            }
        }
        whitespace = character.is_whitespace();
    }
    if quote.is_some() || escaped {
        return Err(ContextError::Configuration);
    }
    Ok(())
}

pub(super) fn config_bytes(keys: &BTreeMap<String, Vec<String>>) -> Result<Vec<u8>, ContextError> {
    serde_json::to_vec(keys).map_err(|_| ContextError::Encoding)
}
