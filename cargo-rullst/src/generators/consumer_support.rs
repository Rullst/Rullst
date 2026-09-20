//! Shared checks for opt-in consumers installed in recognized starter projects.
use super::consumer_files;
use quote::ToTokens;
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use toml_edit::{Array, DocumentMut, InlineTable, Item, Value};

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

pub(super) fn formatted(source: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut child = Command::new("rustfmt")
        .args([
            "--edition",
            "2024",
            "--emit",
            "stdout",
            "--config",
            "skip_children=true",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let write = child
        .stdin
        .take()
        .ok_or_else(|| io::Error::other("rustfmt stdin unavailable"))?
        .write_all(source.as_bytes());
    let output = child.wait_with_output()?;
    write?;
    if !output.status.success() {
        return Err(invalid("rustfmt could not normalize the starter source").into());
    }
    Ok(String::from_utf8(output.stdout)?)
}

fn equivalent(left: &str, right: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let left = syn::parse_file(left)?.into_token_stream().to_string();
    let right = syn::parse_file(right)?.into_token_stream().to_string();
    Ok(left == right || formatted(&left)? == formatted(&right)?)
}

pub(super) fn recognized_auth(
    root: &Path,
    consumer: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let baseline = match consumer {
        "saas" => crate::blueprints::saas::file_manifest(
            "unused",
            false,
            "Active Record",
            "Zero-Bundle HTMX",
        ),
        "lms" => crate::blueprints::lms::file_manifest(
            "unused",
            false,
            "Active Record",
            "Zero-Bundle HTMX",
        ),
        _ => return Err(invalid("unknown authenticated starter").into()),
    };
    for name in [
        "src/middlewares/auth_middleware.rs",
        "src/controllers/auth_controller.rs",
    ] {
        let expected = baseline
            .iter()
            .find(|(path, _)| *path == name)
            .ok_or_else(|| invalid("authentication template is missing"))?;
        if !equivalent(&consumer_files::read(&root.join(name))?, &expected.1)? {
            return Err(invalid("this generator requires the recognized SaaS authentication controller and middleware (or the selected LMS equivalents); review custom authentication separately").into());
        }
    }
    Ok(())
}

pub(super) fn privacy_source(
    source: &Path,
    module: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let source = source.canonicalize()?;
    let privacy: toml::Value = toml::from_str(&fs::read_to_string(source.join("Cargo.toml"))?)?;
    let package = privacy
        .get("package")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| invalid("privacy source has no package table"))?;
    if package.get("name").and_then(toml::Value::as_str) != Some("rullst-privacy")
        || package.get("version").and_then(toml::Value::as_str) != Some("13.0.0-alpha.1")
        || package.get("publish").and_then(toml::Value::as_bool) != Some(false)
        || !source.join(module).is_file()
    {
        return Err(invalid("source must be the matching unpublished v13 privacy package").into());
    }
    Ok(source)
}

/// Compose only the known local preview dependency emitted by these consumers.
/// Unknown source/version/defaults/keys require manual review, never replacement.
pub(super) fn privacy_dependency(
    root: &Path,
    manifest: &mut DocumentMut,
    source: &Path,
    extra: &[&str],
) -> Result<(), Box<dyn std::error::Error>> {
    if manifest
        .get("dependencies")
        .and_then(|deps| deps.get("rullst"))
        .is_none()
    {
        return Err(
            invalid("run this consumer generator inside a generated Rullst project").into(),
        );
    }
    let allowed = [
        "age-assurance",
        "challenge-tokens",
        "sqlite",
        "postgres",
        "consent",
        "consent-sqlite",
    ];
    let mut features = Vec::new();
    if let Some(existing) = manifest["dependencies"].get("rullst-privacy") {
        let table = existing
            .as_inline_table()
            .ok_or_else(|| invalid("existing privacy dependency requires manual review"))?;
        let path = table
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid("existing privacy source is not explicit"))?;
        if root.join(path).canonicalize()? != source
            || table.get("version").and_then(Value::as_str) != Some("=13.0.0-alpha.1")
            || table.get("default-features").and_then(Value::as_bool) != Some(false)
            || table
                .iter()
                .any(|(key, _)| !["path", "version", "default-features", "features"].contains(&key))
        {
            return Err(invalid(
                "existing privacy dependency differs from the recognized local preview",
            )
            .into());
        }
        let array = table
            .get("features")
            .and_then(Value::as_array)
            .ok_or_else(|| invalid("existing privacy features are not explicit"))?;
        for feature in array {
            let feature = feature
                .as_str()
                .filter(|feature| allowed.contains(feature))
                .ok_or_else(|| invalid("unknown privacy feature requires manual review"))?;
            if !features.contains(&feature) {
                features.push(feature);
            }
        }
    }
    for feature in extra {
        if !allowed.contains(feature) {
            return Err(invalid("unsupported privacy feature").into());
        }
        if !features.contains(feature) {
            features.push(feature);
        }
    }
    let mut dependency = InlineTable::new();
    dependency.insert(
        "path",
        Value::from(
            source
                .to_str()
                .ok_or_else(|| invalid("privacy source path must be UTF-8"))?,
        ),
    );
    dependency.insert("version", Value::from("=13.0.0-alpha.1"));
    dependency.insert("default-features", Value::from(false));
    dependency.insert(
        "features",
        Value::Array(features.into_iter().collect::<Array>()),
    );
    manifest["dependencies"]["rullst-privacy"] = Item::Value(Value::InlineTable(dependency));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formatting_equivalence_preserves_semantic_changes() {
        assert!(equivalent("// comment\nfn one() {}", "fn one ( ) { }").unwrap());
        assert!(equivalent("fn one() { call(1); }", "fn one() { call(1,); }").unwrap());
        assert!(!equivalent("fn one() {}", "fn two() {}").unwrap());
    }

    #[test]
    fn compatible_consumers_merge_features_but_cannot_replace_another_source() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().canonicalize().unwrap();
        let mut manifest: DocumentMut = "[dependencies]\nrullst = \"12.1.0\"\n".parse().unwrap();
        privacy_dependency(root.path(), &mut manifest, &source, &["consent-sqlite"]).unwrap();
        privacy_dependency(
            root.path(),
            &mut manifest,
            &source,
            &["challenge-tokens", "sqlite"],
        )
        .unwrap();
        let combined = manifest.to_string();
        for name in ["consent-sqlite", "challenge-tokens", "sqlite"] {
            assert!(combined.contains(name));
        }
        let other = tempfile::tempdir().unwrap();
        assert!(privacy_dependency(root.path(), &mut manifest, other.path(), &["sqlite"]).is_err());
        assert_eq!(manifest.to_string(), combined);
        for change in [
            combined.replace("false", "true"),
            combined.replace("=13.0.0-alpha.1", "13"),
            combined.replace("consent-sqlite", "unknown"),
        ] {
            let mut altered = change.parse().unwrap();
            assert!(privacy_dependency(root.path(), &mut altered, &source, &["sqlite"]).is_err());
            assert_eq!(altered.to_string(), change);
        }
    }
}
