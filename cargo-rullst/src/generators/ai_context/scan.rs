use super::{ContextError, MAX_OUTPUT, ProjectMap, SCHEMA, metadata};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, io::Read, path::Path};

const MAX_ENTRIES: usize = 4096;
const MAX_FILES: usize = 512;
const MAX_FILE: usize = 1024 * 1024;
const MAX_READ: usize = 8 * 1024 * 1024;
const EXCLUSIONS: &[&str] = &[
    "source bodies and configuration values",
    "absolute paths and dependency URLs",
    "live .env files, credentials, databases and non-Rust assets",
    "hidden source descendants",
    "symlinks and special files",
    "target, node_modules, vendor and VCS directories",
    "AGENTS.md contents and generated context output",
];

#[derive(Serialize)]
pub(super) struct SourceFile {
    path: String,
    role: &'static str,
    bytes: usize,
}

fn link(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        metadata.is_symlink()
    }
}

pub(super) fn read_output(path: &Path) -> Result<Option<String>, ContextError> {
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(metadata) if link(&metadata) => return Err(ContextError::UnsafePath),
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    match fs::symlink_metadata(path) {
        Ok(_) => {
            let bytes = bounded_read(path, MAX_OUTPUT)?;
            String::from_utf8(bytes)
                .map(Some)
                .map_err(|_| ContextError::UnsafePath)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn bounded_read(path: &Path, limit: usize) -> Result<Vec<u8>, ContextError> {
    let before = fs::symlink_metadata(path)?;
    if link(&before) || !before.is_file() {
        return Err(ContextError::UnsafePath);
    }
    if before.len() > limit as u64 {
        return Err(ContextError::Limit);
    }
    let file = fs::File::open(path)?;
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(ContextError::Limit);
    }
    let after = fs::symlink_metadata(path)?;
    if link(&after)
        || !after.is_file()
        || before.len() != after.len()
        || after.len() != bytes.len() as u64
        || before.modified()? != after.modified()?
    {
        return Err(ContextError::Changed);
    }
    Ok(bytes)
}

struct Inventory {
    files: Vec<SourceFile>,
    entries: usize,
    excluded: usize,
    bytes: usize,
    fingerprint: Sha256,
}

impl Inventory {
    fn visit(&mut self, root: &Path, directory: &Path, depth: usize) -> Result<(), ContextError> {
        if depth > 16 {
            return Err(ContextError::Limit);
        }
        let metadata = fs::symlink_metadata(directory)?;
        if link(&metadata) || !metadata.is_dir() {
            return Err(ContextError::UnsafePath);
        }
        let mut entries = Vec::new();
        for entry in fs::read_dir(directory)? {
            self.entries += 1;
            if self.entries > MAX_ENTRIES {
                return Err(ContextError::Limit);
            }
            entries.push(entry?);
        }
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                self.excluded += 1;
                continue;
            };
            let metadata = fs::symlink_metadata(&path)?;
            if name.starts_with('.')
                || matches!(name, "target" | "node_modules" | "vendor")
                || link(&metadata)
                || (!metadata.is_dir() && !metadata.is_file())
            {
                self.excluded += 1;
                continue;
            }
            if metadata.is_dir() {
                self.visit(root, &path, depth + 1)?;
                continue;
            }
            if path.extension().is_none_or(|extension| extension != "rs") {
                self.excluded += 1;
                continue;
            }
            if self.files.len() >= MAX_FILES {
                return Err(ContextError::Limit);
            }
            let relative = path
                .strip_prefix(root)
                .map_err(|_| ContextError::UnsafePath)?;
            let name = relative
                .to_str()
                .ok_or(ContextError::UnsafePath)?
                .replace('\\', "/");
            if name.len() > 256
                || !name.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'_' | b'-' | b'.')
                })
            {
                return Err(ContextError::UnsafePath);
            }
            let bytes = bounded_read(&path, MAX_FILE.min(MAX_READ.saturating_sub(self.bytes)))?;
            self.bytes += bytes.len();
            self.fingerprint.update((name.len() as u64).to_be_bytes());
            self.fingerprint.update(name.as_bytes());
            self.fingerprint.update((bytes.len() as u64).to_be_bytes());
            self.fingerprint.update(&bytes);
            let role = match name.split('/').nth(1) {
                Some("main.rs" | "lib.rs") => "entry point",
                Some("models") => "model",
                Some("controllers") => "controller",
                Some("middlewares") => "middleware",
                Some("workers") => "worker",
                Some("migrations") => "migration",
                Some("pages") => "page",
                _ => "Rust source",
            };
            self.files.push(SourceFile {
                path: name,
                role,
                bytes: bytes.len(),
            });
        }
        Ok(())
    }
}

pub(super) fn collect(root: &Path) -> Result<ProjectMap, ContextError> {
    let manifest_bytes = bounded_read(&root.join("Cargo.toml"), 256 * 1024)?;
    let manifest = metadata::manifest(&manifest_bytes)?;
    let mut keys = BTreeMap::new();
    for name in ["Rullst.toml", ".env.example"] {
        let path = root.join(name);
        match fs::symlink_metadata(&path) {
            Ok(_) => {
                let content = bounded_read(&path, 256 * 1024)?;
                let names = if name == "Rullst.toml" {
                    metadata::toml_keys(&content)?
                } else {
                    metadata::env_keys(&content)?
                };
                keys.insert(name.into(), names);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    let mut inventory = Inventory {
        files: Vec::new(),
        entries: 0,
        excluded: 0,
        bytes: 0,
        fingerprint: Sha256::new(),
    };
    inventory.fingerprint.update(SCHEMA.as_bytes());
    // Configuration VALUES and dependency URLs do not even enter the fingerprint.
    inventory.fingerprint.update(metadata::config_bytes(&keys)?);
    inventory.fingerprint.update(
        serde_json::to_vec(&(
            &manifest.project,
            &manifest.dependencies,
            &manifest.features,
            &manifest.requirements,
            &manifest.dependency_features,
        ))
        .map_err(|_| ContextError::Encoding)?,
    );
    let source = root.join("src");
    match fs::symlink_metadata(&source) {
        Ok(_) => inventory.visit(root, &source, 0)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    Ok(ProjectMap {
        schema: SCHEMA,
        generator_version: env!("CARGO_PKG_VERSION"),
        project: manifest.project,
        dependencies: manifest.dependencies,
        declared_features: manifest.features,
        dependency_requirements: manifest.requirements,
        dependency_features: manifest.dependency_features,
        source_scope: "selected Cargo root and src only; workspace members outside src are not inventoried",
        configuration_keys: keys,
        files: inventory.files,
        excluded_entries: inventory.excluded,
        input_sha256: hex::encode(inventory.fingerprint.finalize()),
        exclusions: EXCLUSIONS,
    })
}
