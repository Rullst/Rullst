use super::{ArtifactError, files};
use semver::Version;
use std::path::Path;

pub(super) const MAX_BINARY: u64 = 128 * 1024 * 1024;
pub(super) const REPOSITORY: &str = "Rullst/Rullst";

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Manifest {
    pub schema: String,
    pub version: String,
    pub target: String,
    pub source_commit: String,
    pub repository: String,
    pub release_tag: String,
    pub build_runner: String,
    pub files: Vec<Binary>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Binary {
    pub name: String,
    pub executable: String,
    pub bytes: u64,
    pub sha256: String,
}

fn lowercase_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

impl Manifest {
    pub(super) fn parse(
        bytes: &[u8],
        version: &Version,
        target: &str,
    ) -> Result<Self, ArtifactError> {
        if bytes.is_empty() || bytes.len() > 16 * 1024 {
            return Err(ArtifactError::Invalid("manifest exceeds its size bound"));
        }
        let manifest: Self = serde_json::from_slice(bytes)?;
        if manifest.schema != "rullst.cli-artifacts.v1"
            || manifest.repository != REPOSITORY
            || manifest.version != version.to_string()
            || manifest.release_tag != format!("v{version}")
            || manifest.target != target
            || !lowercase_hex(&manifest.source_commit, 40)
            || manifest.build_runner.is_empty()
            || manifest.build_runner.len() > 64
            || !manifest
                .build_runner
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
            || manifest.files.len() != 2
        {
            return Err(ArtifactError::Invalid(
                "manifest does not match the selected release, source or native target",
            ));
        }
        let suffix = if target.ends_with("windows-msvc") {
            ".exe"
        } else {
            ""
        };
        for (record, executable) in manifest.files.iter().zip(["cargo-rullst", "rullst"]) {
            let name = format!("{executable}-{version}-{target}{suffix}");
            if record.executable != executable
                || record.name != name
                || record.bytes == 0
                || record.bytes > MAX_BINARY
                || !lowercase_hex(&record.sha256, 64)
            {
                return Err(ArtifactError::Invalid(
                    "invalid executable identity, size or digest",
                ));
            }
        }
        Ok(manifest)
    }

    pub(super) fn verify_files(&self, directory: &Path) -> Result<(), ArtifactError> {
        for record in &self.files {
            if files::digest(&directory.join(&record.name), record.bytes)? != record.sha256 {
                return Err(ArtifactError::Invalid(
                    "executable digest does not match the authenticated manifest",
                ));
            }
        }
        Ok(())
    }
}
