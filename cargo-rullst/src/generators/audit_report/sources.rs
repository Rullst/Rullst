//! Project inputs shared by the report checks: production Rust sources,
//! recognized route declarations and the `[security]` table of `Rullst.toml`.

use std::fs;
use std::path::{Path, PathBuf};

use crate::generators::audit_scope::package_source_roots;
use crate::generators::audit_source::audit_source;
use crate::generators::source_walk::rust_sources;

/// Files larger than this are not read by the report scanners.
pub(super) const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;

/// One production Rust source file.
pub(super) struct SourceFile {
    /// Path relative to the project root, for findings.
    pub display: String,
    /// The file without its top-level `#[cfg(test)]` items.
    pub production: String,
    /// `production` without comments; string literals are kept.
    pub code: String,
}

/// The production Rust sources of the package and its workspace members.
pub(super) struct ProjectSources {
    pub files: Vec<SourceFile>,
    /// Whether a `src` directory existed at all.
    pub available: bool,
    /// Why parts of the tree were not read.
    pub incomplete: Vec<String>,
}

impl ProjectSources {
    pub(super) fn load(root: &Path) -> Self {
        let Some(roots) = package_source_roots(root) else {
            return Self {
                files: Vec::new(),
                available: false,
                incomplete: Vec::new(),
            };
        };
        let mut files = Vec::new();
        let mut incomplete = Vec::new();
        for source_root in roots {
            let sources = rust_sources(&source_root);
            if let Some(reason) = sources.incomplete {
                incomplete.push(format!("{}: {reason}", relative(root, &source_root)));
            }
            for path in sources.files {
                let Some(content) = read_bounded(&path) else {
                    continue;
                };
                let views = audit_source(&content);
                files.push(SourceFile {
                    display: relative(root, &path),
                    production: views.production,
                    code: views.code,
                });
            }
        }
        Self {
            files,
            available: true,
            incomplete,
        }
    }

    /// Whether any production code (outside comments) contains `needle`.
    pub(super) fn mentions(&self, needle: &str) -> bool {
        self.files.iter().any(|file| file.code.contains(needle))
    }

    /// The first of `needles` found in production code.
    pub(super) fn first_mention<'a>(&self, needles: &[&'a str]) -> Option<&'a str> {
        needles.iter().copied().find(|needle| self.mentions(needle))
    }

    /// Every recognized route declaration, with its file.
    pub(super) fn routes(&self) -> Vec<(&SourceFile, Route)> {
        self.files
            .iter()
            .flat_map(|file| {
                routes(&file.code)
                    .into_iter()
                    .map(move |route| (file, route))
            })
            .collect()
    }
}

/// A regular, bounded, UTF-8 file; symlinks and special files are not read.
pub(super) fn read_bounded(path: &Path) -> Option<String> {
    let metadata = fs::symlink_metadata(path).ok()?;
    if !metadata.is_file() || metadata.len() > MAX_FILE_BYTES {
        return None;
    }
    fs::read_to_string(path).ok()
}

/// `path` relative to `root`, with `/` separators.
pub(super) fn relative(root: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(root).unwrap_or(path);
    let relative: PathBuf = relative
        .components()
        .filter(|component| !matches!(component, std::path::Component::CurDir))
        .collect();
    relative.to_string_lossy().replace('\\', "/")
}

/// One route declaration recognized on a single line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Route {
    pub method: &'static str,
    pub path: String,
    pub line: usize,
}

impl Route {
    pub(super) fn writes(&self) -> bool {
        matches!(self.method, "post" | "put" | "patch" | "delete")
    }

    pub(super) fn label(&self) -> String {
        format!("{} {}", self.method.to_ascii_uppercase(), self.path)
    }
}

const METHODS: [&str; 5] = ["get", "post", "put", "patch", "delete"];

/// Routes written as `method("/path" => handler)` (`routes!`) or
/// `.route("/path", method(handler))` on one line. Multi-line declarations
/// are not recognized; the checks say so.
pub(super) fn routes(code: &str) -> Vec<Route> {
    let mut found = Vec::new();
    for (index, line) in code.lines().enumerate() {
        let axum_path = line
            .find(".route(")
            .and_then(|start| leading_literal(&line[start + ".route(".len()..]));
        for method in METHODS {
            for (start, _) in line.match_indices(method) {
                let preceded = line[..start]
                    .chars()
                    .next_back()
                    .is_some_and(|before| before.is_alphanumeric() || before == '_');
                let Some(arguments) = line[start + method.len()..].strip_prefix('(') else {
                    continue;
                };
                if preceded {
                    continue;
                }
                let path = match leading_literal(arguments) {
                    Some(path) if path.starts_with('/') => Some(path),
                    Some(_) => None,
                    None => axum_path.clone(),
                };
                if let Some(path) = path {
                    found.push(Route {
                        method,
                        path,
                        line: index + 1,
                    });
                }
            }
        }
    }
    found
}

/// The string literal at the start of `text`, after whitespace.
fn leading_literal(text: &str) -> Option<String> {
    let rest = text.trim_start().strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// The `[security]` table of `Rullst.toml`, with the file text for line
/// numbers. `Ok(None)` when the file does not exist.
pub(super) struct SecurityConfig {
    pub text: String,
    pub table: toml::Table,
}

impl SecurityConfig {
    pub(super) fn load(root: &Path) -> Result<Option<Self>, String> {
        let path = root.join("Rullst.toml");
        if fs::symlink_metadata(&path).is_err() {
            return Ok(None);
        }
        let text = read_bounded(&path)
            .ok_or_else(|| "Rullst.toml is not a readable regular UTF-8 file".to_string())?;
        let document = text
            .parse::<toml::Table>()
            .map_err(|error| format!("Rullst.toml could not be parsed: {error}"))?;
        let table = document
            .get("security")
            .and_then(toml::Value::as_table)
            .cloned()
            .unwrap_or_default();
        Ok(Some(Self { text, table }))
    }

    pub(super) fn string(&self, key: &str) -> Option<&str> {
        self.table.get(key).and_then(toml::Value::as_str)
    }

    pub(super) fn strings(&self, key: &str) -> Vec<String> {
        self.table
            .get(key)
            .and_then(toml::Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(toml::Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The 1-based line of the first `key = ...` assignment.
    pub(super) fn line_of(&self, key: &str) -> Option<usize> {
        self.text
            .lines()
            .position(|line| {
                line.trim_start()
                    .strip_prefix(key)
                    .is_some_and(|rest| rest.trim_start().starts_with('='))
            })
            .map(|index| index + 1)
    }
}
