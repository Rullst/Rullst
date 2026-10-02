//! Cheap, read-only discovery of the Rullst project around the working
//! directory: package name, `rullst` features, database family, migration
//! files and Git branch. Every probe is bounded; anything that cannot be read
//! cheaply is omitted instead of guessed.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

const MANIFEST_LIMIT: u64 = 1024 * 1024;
const CONFIG_LIMIT: u64 = 1024 * 1024;
const ANCESTOR_LIMIT: usize = 64;
const MIGRATION_ENTRY_LIMIT: usize = 10_000;
/// Longest single value shown on the home screen.
const DISPLAY_LIMIT: usize = 64;
const FEATURE_LIMIT: usize = 12;

/// A detected Rullst application.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Project {
    pub root: PathBuf,
    /// The root relative to the working directory, when they differ.
    pub relative_root: Option<String>,
    pub name: String,
    pub features: Vec<String>,
    pub database: Database,
    pub migrations: Option<Migrations>,
    pub git_branch: Option<String>,
}

/// Where the database configuration comes from; URLs are never kept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Database {
    Configured {
        kind: &'static str,
        source: &'static str,
    },
    NotConfigured,
    Unreadable(&'static str),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Migrations {
    pub count: usize,
    pub latest: Option<String>,
}

/// Reads a regular file up to `limit` bytes; `Ok(None)` when it is missing,
/// not a regular file or larger than the limit.
pub(crate) fn read_small_file(path: &Path, limit: u64) -> std::io::Result<Option<String>> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    if !metadata.is_file() || metadata.len() > limit {
        return Ok(None);
    }
    let mut contents = String::new();
    open_for_read(path)?
        .take(limit)
        .read_to_string(&mut contents)?;
    Ok(Some(contents))
}

#[cfg(unix)]
fn open_for_read(path: &Path) -> std::io::Result<fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    // Never block on a FIFO swapped in after the type check.
    fs::OpenOptions::new()
        .read(true)
        .custom_flags(rustix::fs::OFlags::NONBLOCK.bits() as i32)
        .open(path)
}

#[cfg(not(unix))]
fn open_for_read(path: &Path) -> std::io::Result<fs::File> {
    fs::File::open(path)
}

/// Replaces control characters (terminal escapes included) and bounds the
/// length, since names come from files the user may not have written.
pub(crate) fn display_safe(value: &str) -> String {
    let mut safe: String = value
        .chars()
        .map(|character| {
            if character.is_control() {
                '?'
            } else {
                character
            }
        })
        .take(DISPLAY_LIMIT)
        .collect();
    if value.chars().count() > DISPLAY_LIMIT {
        safe.push('…');
    }
    safe
}

/// The `rullst` dependency of a manifest, including a renamed one.
fn rullst_dependency(manifest: &toml::Value) -> Option<&toml::Value> {
    let dependencies = manifest.get("dependencies")?.as_table()?;
    let package = |dependency: &toml::Value| {
        dependency
            .get("package")
            .and_then(toml::Value::as_str)
            .map(str::to_owned)
    };
    dependencies.iter().find_map(|(name, dependency)| {
        let is_rullst = match package(dependency) {
            Some(package) => package == "rullst",
            None => name == "rullst",
        };
        is_rullst.then_some(dependency)
    })
}

fn enabled_features(dependency: &toml::Value) -> Vec<String> {
    let defaults = dependency
        .get("default-features")
        .or_else(|| dependency.get("default_features"))
        .and_then(toml::Value::as_bool)
        .unwrap_or(true);
    let listed = dependency
        .get("features")
        .and_then(toml::Value::as_array)
        .map(|features| {
            features
                .iter()
                .filter_map(toml::Value::as_str)
                .map(display_safe)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let mut features = Vec::new();
    if defaults {
        features.push("default".to_string());
    }
    let extra = listed.len().saturating_sub(FEATURE_LIMIT);
    features.extend(listed.into_iter().take(FEATURE_LIMIT));
    if extra > 0 {
        features.push(format!("+{extra} more"));
    }
    features
}

/// How the `rullst` dependency is specified: its version requirement, or
/// `path`, `git` or `workspace` when it is not a registry requirement.
pub(crate) fn rullst_requirement(manifest: &str) -> Option<String> {
    let manifest = toml::from_str::<toml::Value>(manifest).ok()?;
    let dependency = rullst_dependency(&manifest)?;
    if let Some(version) = dependency.as_str() {
        return Some(display_safe(version));
    }
    if let Some(version) = dependency.get("version").and_then(toml::Value::as_str) {
        return Some(display_safe(version));
    }
    ["path", "git", "workspace"]
        .into_iter()
        .find(|key| dependency.get(key).is_some())
        .map(str::to_string)
}

/// Name and features when `manifest` is a package that depends on `rullst`.
pub(super) fn parse_manifest(manifest: &str) -> Option<(String, Vec<String>)> {
    let manifest = toml::from_str::<toml::Value>(manifest).ok()?;
    let name = manifest.get("package")?.get("name")?.as_str()?;
    let dependency = rullst_dependency(&manifest)?;
    Some((display_safe(name), enabled_features(dependency)))
}

/// The nearest ancestor of `start` (itself included) whose `Cargo.toml` is a
/// package depending on `rullst`.
pub(crate) fn find_project(start: &Path) -> Option<(PathBuf, String, Vec<String>)> {
    start
        .ancestors()
        .take(ANCESTOR_LIMIT)
        .find_map(|directory| {
            let manifest = read_small_file(&directory.join("Cargo.toml"), MANIFEST_LIMIT)
                .ok()
                .flatten()?;
            let (name, features) = parse_manifest(&manifest)?;
            Some((directory.to_path_buf(), name, features))
        })
}

/// `..`-style path from `start` up to its ancestor `root`, or `None` when equal.
pub(crate) fn relative_root(start: &Path, root: &Path) -> Option<String> {
    let depth = start.strip_prefix(root).ok()?.components().count();
    (depth > 0).then(|| vec![".."; depth].join("/"))
}

/// Applies the runtime precedence: the process `DATABASE_URL`, then
/// `DATABASE_URL` in `.env`, then `[database].url` in `Rullst.toml`; a
/// Turso-primary project is recognised by `TURSO_DATABASE_URL`.
pub(super) fn resolve_database(root: &Path, var: impl Fn(&str) -> Option<String>) -> Database {
    let configured = |url: &str, source| Database::Configured {
        kind: super::super::dash_tui::database_kind(url),
        source,
    };
    if let Some(url) = var("DATABASE_URL") {
        return configured(&url, "environment");
    }
    let dotenv = match read_small_file(&root.join(".env"), CONFIG_LIMIT) {
        Ok(Some(contents)) => match crate::generators::dev::parse_dotenv(contents.as_bytes()) {
            Ok(values) => values,
            Err(_) => return Database::Unreadable(".env could not be parsed"),
        },
        Ok(None) => Default::default(),
        Err(_) => return Database::Unreadable(".env could not be read"),
    };
    if let Some(url) = dotenv.get("DATABASE_URL") {
        return configured(url, ".env");
    }
    match read_small_file(&root.join("Rullst.toml"), CONFIG_LIMIT) {
        Ok(Some(contents)) => match crate::generators::dev::parse_rullst_toml(&contents) {
            Ok(config) => {
                if let Some(url) = config
                    .get("database")
                    .and_then(|database| database.get("url"))
                    .and_then(toml::Value::as_str)
                {
                    return configured(url, "Rullst.toml");
                }
            }
            Err(_) => return Database::Unreadable("Rullst.toml is not valid TOML"),
        },
        Ok(None) => {}
        Err(_) => return Database::Unreadable("Rullst.toml could not be read"),
    }
    let turso = |value: &String| !value.trim().is_empty();
    if var("TURSO_DATABASE_URL").as_ref().is_some_and(turso) {
        return configured("libsql://", "environment");
    }
    if dotenv.get("TURSO_DATABASE_URL").is_some_and(turso) {
        return configured("libsql://", ".env");
    }
    Database::NotConfigured
}

/// Counts `src/migrations/m*.rs` registrations; `None` without the directory.
pub(crate) fn migrations(root: &Path) -> Option<Migrations> {
    let entries = fs::read_dir(root.join("src").join("migrations")).ok()?;
    let mut count = 0;
    let mut latest: Option<String> = None;
    for entry in entries.take(MIGRATION_ENTRY_LIMIT) {
        let Ok(entry) = entry else { continue };
        let name = entry.file_name();
        let Some(stem) = name
            .to_str()
            .and_then(|name| name.strip_suffix(".rs"))
            .filter(|stem| stem.starts_with('m') && *stem != "mod")
        else {
            continue;
        };
        count += 1;
        if latest.as_deref().is_none_or(|current| stem > current) {
            latest = Some(stem.to_string());
        }
    }
    Some(Migrations {
        count,
        latest: latest.map(|stem| display_safe(&stem)),
    })
}

impl Project {
    /// Detects the project around `start`, reading only small local files.
    pub(crate) fn detect(start: &Path, var: impl Fn(&str) -> Option<String>) -> Option<Self> {
        let (root, name, features) = find_project(start)?;
        Some(Self {
            relative_root: relative_root(start, &root),
            name,
            features,
            database: resolve_database(&root, var),
            migrations: migrations(&root),
            git_branch: super::git::branch(&root),
            root,
        })
    }
}
