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

pub(super) fn equivalent(left: &str, right: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let left = syn::parse_file(left)?.into_token_stream().to_string();
    let right = syn::parse_file(right)?.into_token_stream().to_string();
    Ok(left == right || formatted(&left)? == formatted(&right)?)
}

/// Checks the unmodified SaaS authentication, or the same files after
/// `make:mfa` wired its second step into `login_submit`; a refusal ends with
/// `remedy`, what the calling generator offers a project without it.
pub(super) fn recognized_auth(
    root: &Path,
    consumer: &str,
    remedy: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let baseline = match consumer {
        "saas" => crate::blueprints::saas::file_manifest(
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
        let state = match consumer_files::read(&root.join(name)) {
            Ok(source) if equivalent(&source, &expected.1)? => continue,
            // `make:mfa` adds its second step to `login_submit`; these
            // consumers do not touch that function, so the wired controller
            // is still the recognized one, as in the reverse order.
            Ok(source)
                if name == "src/controllers/auth_controller.rs"
                    && crate::generators::auth::mfa::login_hook(&expected.1)?
                        .map(|wired| equivalent(&source, &wired))
                        .transpose()?
                        .unwrap_or(false) =>
            {
                continue;
            }
            Ok(_) => "differs from the starter's",
            Err(error) if error.kind() == io::ErrorKind::NotFound => "is missing",
            Err(error) => return Err(error.into()),
        };
        return Err(invalid(&format!(
            "this generator installs only into the recognized SaaS authentication: the unmodified auth_controller.rs and auth_middleware.rs of `cargo rullst new <name> --blueprint saas`, and {name} {state}. {remedy}"
        ))
        .into());
    }
    Ok(())
}

pub(super) fn privacy_source(
    source: Option<&Path>,
    module: &str,
) -> Result<Option<PathBuf>, Box<dyn std::error::Error>> {
    let Some(source) = source else {
        return Ok(None);
    };
    let source = source.canonicalize()?;
    let privacy: toml::Value = toml::from_str(&fs::read_to_string(source.join("Cargo.toml"))?)?;
    let package = privacy
        .get("package")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| invalid("privacy source has no package table"))?;
    if package.get("name").and_then(toml::Value::as_str) != Some("rullst-privacy")
        || package.get("version").and_then(toml::Value::as_str) != Some(env!("CARGO_PKG_VERSION"))
        || !source.join(module).is_file()
    {
        return Err(invalid("source must be the privacy package matching this CLI version").into());
    }
    Ok(Some(source))
}

/// Compose only the known registry or explicit local dependency from these consumers.
/// Unknown source/version/defaults/keys require manual review, never replacement.
pub(super) fn privacy_dependency(
    root: &Path,
    manifest: &mut DocumentMut,
    source: Option<&Path>,
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
        let matching_source = match (table.get("path"), source) {
            (None, None) => true,
            (Some(path), Some(expected)) => path
                .as_str()
                .map(|path| root.join(path).canonicalize().map(|path| path == expected))
                .transpose()?
                .unwrap_or(false),
            _ => false,
        };
        let exact_version = format!("={}", env!("CARGO_PKG_VERSION"));
        if !matching_source
            || table.get("version").and_then(Value::as_str) != Some(exact_version.as_str())
            || table.get("default-features").and_then(Value::as_bool) != Some(false)
            || table
                .iter()
                .any(|(key, _)| !["path", "version", "default-features", "features"].contains(&key))
        {
            return Err(invalid(
                "existing privacy dependency differs from the selected source/version; review it manually",
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
    if let Some(source) = source {
        dependency.insert(
            "path",
            Value::from(
                source
                    .to_str()
                    .ok_or_else(|| invalid("privacy source path must be UTF-8"))?,
            ),
        );
    }
    dependency.insert(
        "version",
        Value::from(format!("={}", env!("CARGO_PKG_VERSION"))),
    );
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

    const REMEDY: &str = "Review a custom authentication separately.";

    #[test]
    fn only_the_recognized_saas_authentication_is_accepted() {
        let directory = tempfile::tempdir().unwrap();
        // Scaffolds refuse symlinked paths, and macOS keeps temporary
        // directories below the `/var` -> `/private/var` link.
        let root = directory.path().canonicalize().unwrap();
        assert!(recognized_auth(root.as_path(), "lms", REMEDY).is_err());
        let baseline = crate::blueprints::saas::file_manifest(
            "unused",
            false,
            "Active Record",
            "Zero-Bundle HTMX",
        );
        for name in [
            "src/middlewares/auth_middleware.rs",
            "src/controllers/auth_controller.rs",
        ] {
            let (_, content) = baseline.iter().find(|(path, _)| *path == name).unwrap();
            let path = root.as_path().join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, content).unwrap();
        }
        recognized_auth(root.as_path(), "saas", REMEDY).unwrap();
        // The controller as `make:mfa` leaves it is still recognized.
        let controller = root.as_path().join("src/controllers/auth_controller.rs");
        let starter = fs::read_to_string(&controller).unwrap();
        let wired = crate::generators::auth::mfa::login_hook(&starter)
            .unwrap()
            .expect("make:mfa recognizes the starter's login_submit");
        assert_ne!(wired, starter);
        fs::write(&controller, &wired).unwrap();
        recognized_auth(root.as_path(), "saas", REMEDY).unwrap();
        // Another edit of the same file is not.
        fs::write(&controller, format!("{wired}\nfn custom() {{}}\n")).unwrap();
        let error = recognized_auth(root.as_path(), "saas", REMEDY)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("src/controllers/auth_controller.rs differs from the starter's"),
            "{error}"
        );
        fs::write(&controller, &starter).unwrap();
        let middleware = root.as_path().join("src/middlewares/auth_middleware.rs");
        let edited = format!(
            "{}\nfn custom() {{}}\n",
            fs::read_to_string(&middleware).unwrap()
        );
        fs::write(&middleware, edited).unwrap();
        let error = recognized_auth(root.as_path(), "saas", REMEDY)
            .unwrap_err()
            .to_string();
        assert!(
            error.ends_with(&format!(
                "and src/middlewares/auth_middleware.rs differs from the starter's. {REMEDY}"
            )),
            "{error}"
        );
        assert!(
            error.starts_with(
                "this generator installs only into the recognized SaaS authentication"
            )
        );
        fs::remove_file(root.as_path().join("src/controllers/auth_controller.rs")).unwrap();
        fs::write(
            &middleware,
            &baseline
                .iter()
                .find(|(path, _)| *path == "src/middlewares/auth_middleware.rs")
                .unwrap()
                .1,
        )
        .unwrap();
        let error = recognized_auth(root.as_path(), "saas", REMEDY)
            .unwrap_err()
            .to_string();
        assert!(
            error.ends_with(&format!(
                "and src/controllers/auth_controller.rs is missing. {REMEDY}"
            )),
            "{error}"
        );
        // A file that cannot be read for another reason (here a directory in
        // its place) is reported as that I/O error, not as a missing file.
        fs::create_dir(root.as_path().join("src/controllers/auth_controller.rs")).unwrap();
        let error = recognized_auth(root.as_path(), "saas", REMEDY).unwrap_err();
        assert!(error.downcast_ref::<io::Error>().is_some(), "{error}");
        assert!(!error.to_string().contains("is missing"), "{error}");
    }

    #[test]
    fn a_privacy_source_must_be_this_version_of_the_package() {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../rullst-privacy");
        assert_eq!(
            privacy_source(Some(&workspace), "src/lib.rs").unwrap(),
            Some(workspace.canonicalize().unwrap())
        );
        assert_eq!(privacy_source(None, "src/lib.rs").unwrap(), None);
        let version = env!("CARGO_PKG_VERSION");
        for (name, package_version, module) in [
            ("other-privacy", version, true),
            ("rullst-privacy", "0.0.1", true),
            ("rullst-privacy", version, false),
        ] {
            let source = tempfile::tempdir().unwrap();
            fs::write(
                source.path().join("Cargo.toml"),
                format!("[package]\nname = \"{name}\"\nversion = \"{package_version}\"\n"),
            )
            .unwrap();
            if module {
                fs::create_dir(source.path().join("src")).unwrap();
                fs::write(source.path().join("src/lib.rs"), "").unwrap();
            }
            assert!(
                privacy_source(Some(source.path()), "src/lib.rs").is_err(),
                "{name} {package_version} {module}"
            );
        }
    }

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
        privacy_dependency(
            root.path(),
            &mut manifest,
            Some(&source),
            &["consent-sqlite"],
        )
        .unwrap();
        privacy_dependency(
            root.path(),
            &mut manifest,
            Some(&source),
            &["challenge-tokens", "sqlite"],
        )
        .unwrap();
        let combined = manifest.to_string();
        for name in ["consent-sqlite", "challenge-tokens", "sqlite"] {
            assert!(combined.contains(name));
        }
        let other = tempfile::tempdir().unwrap();
        assert!(
            privacy_dependency(root.path(), &mut manifest, Some(other.path()), &["sqlite"])
                .is_err()
        );
        assert!(privacy_dependency(root.path(), &mut manifest, None, &["sqlite"]).is_err());
        assert_eq!(manifest.to_string(), combined);
        for change in [
            combined.replace("false", "true"),
            combined.replace(&format!("={}", env!("CARGO_PKG_VERSION")), "13"),
            combined.replace("consent-sqlite", "unknown"),
        ] {
            let mut altered = change.parse().unwrap();
            assert!(
                privacy_dependency(root.path(), &mut altered, Some(&source), &["sqlite"]).is_err()
            );
            assert_eq!(altered.to_string(), change);
        }
    }

    #[test]
    fn registry_consumers_preserve_exact_version_features_and_source() {
        let root = tempfile::tempdir().unwrap();
        let mut manifest: DocumentMut = "[dependencies]\nrullst = \"13\"\n".parse().unwrap();
        privacy_dependency(root.path(), &mut manifest, None, &["consent-sqlite"]).unwrap();
        privacy_dependency(
            root.path(),
            &mut manifest,
            None,
            &["challenge-tokens", "postgres"],
        )
        .unwrap();
        let original = manifest.to_string();
        let table = manifest["dependencies"]["rullst-privacy"]
            .as_inline_table()
            .unwrap();
        assert!(table.get("path").is_none());
        assert_eq!(
            table["version"].as_str().unwrap(),
            format!("={}", env!("CARGO_PKG_VERSION"))
        );
        assert_eq!(table["features"].as_array().unwrap().len(), 3);
        assert!(
            privacy_dependency(root.path(), &mut manifest, Some(root.path()), &["sqlite"]).is_err()
        );
        assert_eq!(original, manifest.to_string());
        for extra in [
            "git = 'https://example.invalid/repo'",
            "registry = 'other'",
            "optional = true",
            "package = 'other'",
        ] {
            let changed =
                original.replace("default-features", &format!("{extra}, default-features"));
            let mut manifest = changed.parse().unwrap();
            assert!(privacy_dependency(root.path(), &mut manifest, None, &["sqlite"]).is_err());
            assert_eq!(manifest.to_string(), changed);
        }
    }
}
