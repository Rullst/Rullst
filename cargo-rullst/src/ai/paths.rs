//! Project-root path allowlist for model-proposed file edits and user
//! attachments.
//!
//! A model-supplied path is a relative, normalised path below the canonical
//! project root. No component may be a symlink, `..`, `.git`, `target`,
//! `.cargo` or another denied name, and secret or build-tool files are refused
//! outright. Names a filesystem could map onto a denied one are refused too:
//! Windows 8.3 aliases (`GIT~1`), reserved device names, trailing dots or
//! spaces, and the default-ignorable characters HFS+ drops (`.g\u{200c}it`);
//! existing components are also checked under their canonical name. The
//! workspace directory itself is trusted against concurrent adversarial
//! filesystem edits; writes replace files atomically, so a hard link is
//! detached instead of written through.

use std::fs;
use std::path::{Component, Path, PathBuf};

/// Longest accepted relative path.
const MAX_PATH_BYTES: usize = 512;
/// Deepest accepted relative path.
const MAX_COMPONENTS: usize = 32;

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub(super) enum PathError {
    #[error("the path must be a non-empty relative path of at most 512 bytes")]
    Malformed,
    #[error("the path leaves the project root")]
    Escapes,
    #[error("`{0}` is protected and cannot be read or changed by the assistant")]
    Denied(String),
    #[error("the path goes through a symbolic link")]
    Symlink,
    #[error("the path exists but is not a regular file")]
    NotAFile,
    #[error("the path exists but is not a directory")]
    NotADirectory,
    #[error("the path could not be inspected")]
    Io,
}

/// A validated location below the project root.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ProjectPath {
    /// Normalised relative form with `/` separators, used for display.
    pub display: String,
    /// Absolute path below the canonical root.
    pub absolute: PathBuf,
    /// Whether the file already exists.
    pub exists: bool,
    /// Build configuration or code that runs during `cargo check`.
    pub sensitive: bool,
}

/// Directory names that are never entered.
const DENIED_DIRECTORIES: &[&str] = &[".git", "target", ".cargo", ".ssh", ".gnupg", ".aws"];

/// File names that hold secrets, toolchain overrides or managed state.
fn denied_file(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    (lower == ".env" || (lower.starts_with(".env.") && lower != ".env.example"))
        || matches!(
            lower.as_str(),
            "credentials"
                | "credentials.toml"
                | "credentials.json"
                | "credentials.yaml"
                | "credentials.yml"
                | "credentials.ini"
                | "cargo.lock"
                | "rust-toolchain"
                | "rust-toolchain.toml"
                | ".netrc"
                | ".npmrc"
                | ".pypirc"
                | "id_rsa"
                | "id_ecdsa"
                | "id_ed25519"
        )
        || [
            ".pem",
            ".key",
            ".p12",
            ".pfx",
            ".jks",
            ".keystore",
            ".db",
            ".sqlite",
            ".sqlite3",
        ]
        .iter()
        .any(|suffix| lower.ends_with(suffix))
}

/// Default-ignorable characters that HFS+ drops from names (Git's
/// `core.protectHFS` list, plus the soft hyphen and zero-width space).
const fn ignorable(character: char) -> bool {
    matches!(
        character,
        '\u{00AD}'
            | '\u{200B}'..='\u{200F}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2060}'..='\u{206F}'
            | '\u{FEFF}'
    )
}

/// An 8.3 short-name alias such as `GIT~1` or `CARGO~2`, which NTFS
/// resolves to the long name it abbreviates.
fn short_name_alias(name: &str) -> bool {
    name.as_bytes()
        .windows(2)
        .any(|pair| pair[0] == b'~' && pair[1].is_ascii_digit())
}

/// Names that a Windows or macOS filesystem would reinterpret.
fn ambiguous_component(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or(name).to_ascii_uppercase();
    name.ends_with('.')
        || name.ends_with(' ')
        || name.contains(':')
        || short_name_alias(name)
        || name.chars().any(ignorable)
        || matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ((stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.len() == 4
            && stem.as_bytes()[3].is_ascii_digit())
}

/// Lexically validates `raw` and returns its normal components.
fn components(raw: &str) -> Result<Vec<String>, PathError> {
    if raw.is_empty()
        || raw.len() > MAX_PATH_BYTES
        || raw.contains('\\')
        || raw.chars().any(char::is_control)
        || raw.starts_with('~')
    {
        return Err(PathError::Malformed);
    }
    let path = Path::new(raw);
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => {
                let part = part.to_str().ok_or(PathError::Malformed)?;
                if ambiguous_component(part) {
                    return Err(PathError::Malformed);
                }
                parts.push(part.to_string());
            }
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(PathError::Escapes);
            }
        }
    }
    if parts.is_empty() || parts.len() > MAX_COMPONENTS {
        return Err(PathError::Malformed);
    }
    for (index, part) in parts.iter().enumerate() {
        if denied(part, index + 1 == parts.len()) {
            return Err(PathError::Denied(parts[..=index].join("/")));
        }
    }
    Ok(parts)
}

fn denied(name: &str, last: bool) -> bool {
    DENIED_DIRECTORIES.contains(&name.to_ascii_lowercase().as_str()) || (last && denied_file(name))
}

/// Directories of Cargo targets that `cargo test` (or `--all-targets`) builds
/// and runs automatically.
const TARGET_DIRECTORIES: &[&str] = &["tests", "benches", "examples"];

fn sensitive(parts: &[String]) -> bool {
    let first = parts.first().map(|part| part.to_ascii_lowercase());
    let last = parts.last().map(|part| part.to_ascii_lowercase());
    matches!(
        last.as_deref(),
        Some("build.rs" | "cargo.toml" | "rullst.toml")
    ) || matches!(first.as_deref(), Some(".github"))
        || parts[..parts.len().saturating_sub(1)]
            .iter()
            .any(|part| TARGET_DIRECTORIES.contains(&part.to_ascii_lowercase().as_str()))
}

/// Rust source that `cargo check` or `cargo test` compiles into code that runs:
/// tests, benchmarks and procedural macros.
pub(super) fn runs_during_cargo(text: &str) -> bool {
    [
        "#[test]",
        "::test]",
        "#[cfg(test)]",
        "#[bench]",
        "proc_macro",
    ]
    .iter()
    .any(|marker| text.contains(marker))
}

/// The canonical (long, case-preserved) name of an existing component, so an
/// alias the lexical checks did not recognise still meets the denylist.
fn canonical_name_denied(path: &Path, last: bool) -> Result<bool, PathError> {
    let canonical = fs::canonicalize(path).map_err(|_| PathError::Io)?;
    Ok(canonical
        .file_name()
        .and_then(|name| name.to_str())
        .is_none_or(|name| denied(name, last)))
}

/// Resolves `raw` below `root` (already canonical). Every existing prefix is
/// checked with `symlink_metadata`, so no link is ever followed; missing
/// components may be created later as plain directories.
pub(super) fn resolve(root: &Path, raw: &str) -> Result<ProjectPath, PathError> {
    walk(root, raw, false)
}

/// [`resolve`] for a directory a command reads or writes, such as an output
/// directory: an existing final component must be a directory.
pub(super) fn resolve_directory(root: &Path, raw: &str) -> Result<ProjectPath, PathError> {
    walk(root, raw, true)
}

fn walk(root: &Path, raw: &str, directory: bool) -> Result<ProjectPath, PathError> {
    let parts = components(raw)?;
    let mut current = root.to_path_buf();
    let mut exists = true;
    for (index, part) in parts.iter().enumerate() {
        current.push(part);
        if !exists {
            continue;
        }
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => return Err(PathError::Symlink),
            Ok(metadata) => {
                let last = index + 1 == parts.len();
                if canonical_name_denied(&current, last)? {
                    return Err(PathError::Denied(parts[..=index].join("/")));
                }
                if last && directory && !metadata.is_dir() {
                    return Err(PathError::NotADirectory);
                }
                if last && !directory && !metadata.is_file() {
                    return Err(PathError::NotAFile);
                }
                if !last && !metadata.is_dir() {
                    return Err(PathError::NotAFile);
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => exists = false,
            Err(_) => return Err(PathError::Io),
        }
    }
    if !current.starts_with(root) {
        return Err(PathError::Escapes);
    }
    Ok(ProjectPath {
        display: parts.join("/"),
        absolute: current,
        exists,
        sensitive: sensitive(&parts),
    })
}

#[cfg(test)]
#[path = "tests/paths.rs"]
mod tests;
