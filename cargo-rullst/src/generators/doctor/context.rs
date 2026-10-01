//! What the doctor knows about the project around the working directory:
//! its root, manifest, `.env`, `.env.example` and `Rullst.toml`. Values are
//! kept in memory for checks and never printed.

use crate::ui::home::{find_project, read_small_file, relative_root};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const CONFIG_LIMIT: u64 = 1024 * 1024;

/// The parsed state of an optional dotenv file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum DotEnv {
    Missing,
    Invalid,
    Parsed(HashMap<String, String>),
}

impl DotEnv {
    pub(crate) fn load(path: &Path) -> Self {
        match read_small_file(path, CONFIG_LIMIT) {
            Ok(None) => Self::Missing,
            Ok(Some(contents)) => crate::generators::dev::parse_dotenv(contents.as_bytes())
                .map_or(Self::Invalid, Self::Parsed),
            Err(_) => Self::Invalid,
        }
    }

    pub(crate) fn get(&self, key: &str) -> Option<&str> {
        match self {
            Self::Parsed(values) => values.get(key).map(String::as_str),
            _ => None,
        }
    }
}

/// A detected Rullst project.
#[derive(Clone, Debug)]
pub(crate) struct ProjectContext {
    pub root: PathBuf,
    pub relative_root: Option<String>,
    pub name: String,
    pub manifest: String,
    pub dotenv: DotEnv,
    pub example: DotEnv,
    /// `Rullst.toml` contents, when the file exists and is readable.
    pub rullst_toml: Option<String>,
}

impl ProjectContext {
    pub(crate) fn detect(start: &Path) -> Option<Self> {
        let (root, name, _) = find_project(start)?;
        let manifest = read_small_file(&root.join("Cargo.toml"), CONFIG_LIMIT)
            .ok()
            .flatten()
            .unwrap_or_default();
        Some(Self {
            relative_root: relative_root(start, &root),
            name,
            manifest,
            dotenv: DotEnv::load(&root.join(".env")),
            example: DotEnv::load(&root.join(".env.example")),
            rullst_toml: read_small_file(&root.join("Rullst.toml"), CONFIG_LIMIT)
                .ok()
                .flatten(),
            root,
        })
    }
}

/// Process environment lookups, injectable for tests.
pub(crate) trait Vars {
    fn var(&self, name: &str) -> Option<String>;
}

/// The real process environment.
pub(crate) struct ProcessVars;

impl Vars for ProcessVars {
    fn var(&self, name: &str) -> Option<String> {
        std::env::var(name).ok()
    }
}

impl Vars for HashMap<String, String> {
    fn var(&self, name: &str) -> Option<String> {
        self.get(name).cloned()
    }
}

/// Where a configuration value came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Source {
    Environment,
    DotEnv,
    RullstToml,
}

impl Source {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Environment => "environment",
            Self::DotEnv => ".env",
            Self::RullstToml => "Rullst.toml",
        }
    }
}

/// `key` from the process environment, then `.env` (the runtime precedence).
pub(crate) fn lookup(
    project: &ProjectContext,
    vars: &impl Vars,
    key: &str,
) -> Option<(String, Source)> {
    if let Some(value) = vars.var(key) {
        return Some((value, Source::Environment));
    }
    project
        .dotenv
        .get(key)
        .map(|value| (value.to_string(), Source::DotEnv))
}
