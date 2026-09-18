//! Strict local ownership evidence; publisher authority is checked separately.
use super::{ArtifactError, Manifest, files};
use semver::Version;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(super) const RECEIPT: &str = ".rullst-cli-installation.json";

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Receipt {
    schema_version: String,
    root: PathBuf,
    /// Preserve exactly the attested bytes, not a reserialized manifest.
    manifest: String,
}

pub(super) fn receipt(root: &Path, manifest: &[u8]) -> Result<Vec<u8>, ArtifactError> {
    let manifest = std::str::from_utf8(manifest)
        .map_err(|_| ArtifactError::Invalid("manifest is not UTF-8"))?;
    let bytes = serde_json::to_vec(&Receipt {
        schema_version: "rullst.cli-installation.v1".into(),
        root: root.to_owned(),
        manifest: manifest.into(),
    })?;
    if bytes.len() > 64 * 1024 {
        return Err(ArtifactError::Invalid("receipt exceeds its bound"));
    }
    Ok(bytes)
}

pub(super) struct Installed {
    pub manifest: Manifest,
    pub raw_manifest: Vec<u8>,
    pub receipt_sha256: String,
}

impl Installed {
    pub fn version(&self) -> Result<Version, ArtifactError> {
        Version::parse(&self.manifest.version)
            .map_err(|_| ArtifactError::Invalid("invalid installed CLI version"))
    }

    pub fn summary(&self) -> serde_json::Value {
        serde_json::json!({"artifact": self.manifest, "receipt_sha256": self.receipt_sha256})
    }
}

pub(super) fn executable_names(target: &str) -> [String; 2] {
    let suffix = if target.ends_with("windows-msvc") {
        ".exe"
    } else {
        ""
    };
    [format!("cargo-rullst{suffix}"), format!("rullst{suffix}")]
}

/// Inspect a new/empty root or exactly the updater-owned receipt and binaries.
/// This never creates directories, repairs permissions or executes binaries.
pub(super) fn inspect(root: &Path, target: &str) -> Result<Option<Installed>, ArtifactError> {
    inspect_at(root, root, target)
}

pub(super) fn inspect_at(
    directory: &Path,
    root: &Path,
    target: &str,
) -> Result<Option<Installed>, ArtifactError> {
    if !directory.try_exists()? {
        return Ok(None);
    }
    files::directory(directory)?;
    let mut names = Vec::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        if names.len() >= 3 || !entry.file_type()?.is_file() {
            return Err(ArtifactError::Invalid(
                "unknown installation entries; refusing package-manager takeover",
            ));
        }
        names.push(entry.file_name());
    }
    if names.is_empty() {
        return Ok(None);
    }
    let executables = executable_names(target);
    if names.len() != 3
        || ![RECEIPT, &executables[0], &executables[1]]
            .iter()
            .all(|expected| names.iter().any(|actual| actual == *expected))
    {
        return Err(ArtifactError::Invalid(
            "existing installation lacks exact updater ownership evidence",
        ));
    }
    super::super::super::cache::installation_file(&directory.join(RECEIPT))?;
    let bytes = files::read_bounded(&directory.join(RECEIPT), 64 * 1024)?;
    let receipt: Receipt = serde_json::from_slice(&bytes)?;
    if receipt.schema_version != "rullst.cli-installation.v1" || receipt.root != root {
        return Err(ArtifactError::Invalid(
            "installation receipt does not belong to this root",
        ));
    }
    if receipt.manifest.is_empty() || receipt.manifest.len() > 16 * 1024 {
        return Err(ArtifactError::Invalid(
            "installed manifest exceeds its bound",
        ));
    }
    let metadata: serde_json::Value = serde_json::from_str(&receipt.manifest)?;
    let version = metadata
        .get("version")
        .and_then(serde_json::Value::as_str)
        .and_then(|text| Version::parse(text).ok())
        .ok_or(ArtifactError::Invalid("invalid installed CLI version"))?;
    let manifest = Manifest::parse(receipt.manifest.as_bytes(), &version, target)?;
    for (file, name) in manifest.files.iter().zip(executables) {
        let path = directory.join(name);
        super::super::super::cache::installation_file(&path)?;
        if files::digest(&path, file.bytes)? != file.sha256 {
            return Err(ArtifactError::Invalid(
                "installed executable diverged from its receipt",
            ));
        }
    }
    Ok(Some(Installed {
        manifest,
        raw_manifest: receipt.manifest.into_bytes(),
        receipt_sha256: hex::encode(Sha256::digest(bytes)),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::update::cache;
    use std::io::Write;

    fn write_private(path: &Path, bytes: &[u8]) {
        cache::new_installation_file(path)
            .unwrap()
            .write_all(bytes)
            .unwrap();
    }

    fn fixture() -> (tempfile::TempDir, PathBuf, String) {
        #[cfg(windows)]
        let directory = tempfile::tempdir_in(std::env::var_os("LOCALAPPDATA").unwrap()).unwrap();
        #[cfg(not(windows))]
        let directory = tempfile::tempdir().unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o700)).unwrap();
        }
        let root =
            cache::create_installation_directory(&directory.path().join("installed")).unwrap();
        let manifest = super::super::super::tests::fixture_manifest();
        let target = manifest.target.clone();
        for name in executable_names(&target) {
            write_private(&root.join(name), b"candidate must never be executed");
        }
        let receipt = Receipt {
            schema_version: "rullst.cli-installation.v1".into(),
            root: root.clone(),
            manifest: serde_json::to_string(&manifest).unwrap(),
        };
        write_private(&root.join(RECEIPT), &serde_json::to_vec(&receipt).unwrap());
        (directory, root, target)
    }

    #[test]
    fn receipt_binds_root_platform_both_executables_and_exact_manifest_bytes() {
        let (_directory, root, target) = fixture();
        let state = inspect(&root, &target).unwrap().unwrap();
        assert_eq!(state.version().unwrap(), Version::new(12, 1, 0));
        assert!(inspect(&root, "x86_64-pc-windows-msvc").is_err());
        fs::write(root.join("rullst"), b"candidate was changed").unwrap();
        assert!(inspect(&root, &target).is_err());
    }

    #[test]
    fn missing_or_foreign_ownership_and_unknown_fields_fail_closed() {
        for field in ["root", "schema_version", "unknown"] {
            let (_directory, root, target) = fixture();
            let mut value: serde_json::Value =
                serde_json::from_slice(&fs::read(root.join(RECEIPT)).unwrap()).unwrap();
            value[field] = serde_json::json!("foreign");
            fs::write(root.join(RECEIPT), serde_json::to_vec(&value).unwrap()).unwrap();
            assert!(inspect(&root, &target).is_err(), "accepted {field}");
        }
        let (_directory, root, target) = fixture();
        fs::write(root.join(".crates.toml"), b"package-manager record").unwrap();
        assert!(inspect(&root, &target).is_err());
        fs::remove_file(root.join(".crates.toml")).unwrap();
        fs::remove_file(root.join(RECEIPT)).unwrap();
        assert!(inspect(&root, &target).is_err());
    }

    #[test]
    fn unchanged_bytes_with_a_hardlinked_alias_are_not_updater_owned() {
        let (_directory, root, target) = fixture();
        let outside = tempfile::tempdir_in(root.parent().unwrap()).unwrap();
        let alias = outside.path().join("user-owned-alias");
        fs::hard_link(root.join("rullst"), &alias).unwrap();
        assert!(inspect(&root, &target).is_err());
        assert_eq!(
            fs::read(alias).unwrap(),
            b"candidate must never be executed"
        );
    }

    #[test]
    fn review_digest_binds_the_exact_prior_receipt() {
        let (_directory, root, target) = fixture();
        let before = inspect(&root, &target).unwrap().unwrap();
        let review = super::super::review(&root, &before.manifest, Some(&before)).unwrap();
        assert_ne!(
            review,
            super::super::review(&root, &before.manifest, None).unwrap()
        );
        let receipt = fs::read_to_string(root.join(RECEIPT)).unwrap();
        fs::write(root.join(RECEIPT), format!("{receipt}\n")).unwrap();
        let after = inspect(&root, &target).unwrap().unwrap();
        assert_ne!(
            review["review_sha256"],
            super::super::review(&root, &after.manifest, Some(&after)).unwrap()["review_sha256"]
        );
    }
}
