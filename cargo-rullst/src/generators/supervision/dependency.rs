//! Pin registry consumers while keeping prerelease evaluation source-explicit.
use super::{files, invalid};
use std::{io, path::Path};
use toml_edit::{Array, InlineTable, Value};

pub(super) fn select(source: Option<&Path>, version: &str) -> io::Result<InlineTable> {
    let parsed =
        semver::Version::parse(version).map_err(|_| invalid("invalid supervision CLI version"))?;
    if !parsed.pre.is_empty() && source.is_none() {
        return Err(invalid(
            "a prerelease CLI requires --supervision-source pointing to the matching local crate",
        ));
    }
    let mut dependency = InlineTable::new();
    if let Some(source) = source {
        let source = source.canonicalize()?;
        let package: toml::Value = toml::from_str(&files::read(&source.join("Cargo.toml"))?)
            .map_err(|_| invalid("invalid supervision source manifest"))?;
        if package
            .get("package")
            .and_then(|p| p.get("name"))
            .and_then(toml::Value::as_str)
            != Some("rullst-supervision")
            || package
                .get("package")
                .and_then(|p| p.get("version"))
                .and_then(toml::Value::as_str)
                != Some(version)
            || !source.join("src/sqlite/mod.rs").is_file()
        {
            return Err(invalid(
                "source must be the supervision package matching this CLI",
            ));
        }
        dependency.insert(
            "path",
            Value::from(
                source
                    .to_str()
                    .ok_or_else(|| invalid("source path must be UTF-8"))?,
            ),
        );
    }
    dependency.insert("version", Value::from(format!("={version}")));
    dependency.insert("default-features", Value::from(false));
    dependency.insert(
        "features",
        Value::Array(["sqlite"].into_iter().collect::<Array>()),
    );
    Ok(dependency)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn stable_registry_selection_pins_the_cli_version_without_a_local_path() {
        let dependency = select(None, "13.0.0").unwrap();
        assert_eq!(
            dependency.get("version").and_then(Value::as_str),
            Some("=13.0.0")
        );
        assert!(!dependency.contains_key("path"));
        assert_eq!(
            dependency.get("default-features").and_then(Value::as_bool),
            Some(false)
        );
        let features = dependency
            .get("features")
            .and_then(Value::as_array)
            .unwrap();
        assert_eq!(features.len(), 1);
        assert_eq!(features.get(0).and_then(Value::as_str), Some("sqlite"));
    }

    #[test]
    fn prerelease_without_source_and_invalid_versions_are_rejected() {
        for version in ["13.0.0-alpha.1", "13.0.0-rc.1", "invalid"] {
            assert!(select(None, version).is_err());
        }
    }

    #[test]
    fn local_source_requires_matching_identity_version_and_sqlite_module() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path();
        fs::create_dir_all(root.join("src/sqlite")).unwrap();
        fs::write(root.join("src/sqlite/mod.rs"), "").unwrap();
        for (name, version, accepted) in [
            ("rullst-supervision", "13.0.0-alpha.1", true),
            ("rullst-supervision", "13.0.0", false),
            ("rullst-supervision", "12.1.0", false),
            ("unrelated", "13.0.0-alpha.1", false),
        ] {
            fs::write(
                root.join("Cargo.toml"),
                format!("[package]\nname={name:?}\nversion={version:?}\n"),
            )
            .unwrap();
            let dependency = select(Some(root), "13.0.0-alpha.1");
            assert_eq!(dependency.is_ok(), accepted);
            if accepted {
                let dependency = dependency.unwrap();
                assert_eq!(
                    dependency.get("version").and_then(Value::as_str),
                    Some("=13.0.0-alpha.1")
                );
                assert_eq!(
                    dependency.get("path").and_then(Value::as_str),
                    root.canonicalize().unwrap().to_str()
                );
            }
        }
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname='rullst-supervision'\nversion='13.0.0-alpha.1'\n",
        )
        .unwrap();
        fs::remove_file(root.join("src/sqlite/mod.rs")).unwrap();
        assert!(select(Some(root), "13.0.0-alpha.1").is_err());
    }
}
