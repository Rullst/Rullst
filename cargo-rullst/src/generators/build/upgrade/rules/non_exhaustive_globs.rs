//! Where a glob-imported struct name comes from, for the literals that
//! [`super::alpha6_non_exhaustive`] can only report for review
//! (`V13-NON-EXHAUSTIVE-LITERAL-GLOB`): a file that does not name the
//! struct's crate but imports `use super::*;`, such as a test module declared
//! with `#[path = "x_tests.rs"] mod tests;`. Each file is read alone, so the
//! name's origin is only known once the package's module tree is: this module
//! records every file's top-level `mod`, `use` and type items while the
//! package is scanned, then follows the `use super::*;` chain to the module
//! that brings the name in. When that module imports it from the struct's
//! Rullst crate (by name or through a glob of a path naming the crate), the
//! literal must change (`V13-NON-EXHAUSTIVE-LITERAL`); when it defines the
//! name or imports it from another crate, the literal builds the
//! application's own type and nothing is reported; otherwise (a `crate::`
//! import, another glob, an unknown parent) the review finding stays.

use super::SourceFinding;
use super::catalog::{NON_EXHAUSTIVE_LITERAL, NON_EXHAUSTIVE_LITERAL_GLOB};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

/// `use super::*;` links followed at most.
const MAX_DEPTH: usize = 16;

/// A file's top-level module items.
#[derive(Debug, Default)]
pub(super) struct ModuleFacts {
    /// `mod name;` declarations with their `#[path = ".."]`, if any.
    modules: Vec<(String, Option<String>)>,
    /// Each name a `use` brings in, with the path it comes from.
    imports: BTreeMap<String, Vec<String>>,
    /// The paths of the `use ..::*;` globs.
    globs: Vec<Vec<String>>,
    /// Structs, enums, unions and type aliases the file defines.
    defines: BTreeSet<String>,
}

impl ModuleFacts {
    pub(super) fn of(file: &syn::File) -> Self {
        let mut facts = Self::default();
        for item in &file.items {
            match item {
                syn::Item::Mod(item) if item.content.is_none() => {
                    let path = item.attrs.iter().find_map(|attribute| {
                        let syn::Meta::NameValue(pair) = &attribute.meta else {
                            return None;
                        };
                        let syn::Expr::Lit(literal) = &pair.value else {
                            return None;
                        };
                        let syn::Lit::Str(text) = &literal.lit else {
                            return None;
                        };
                        pair.path.is_ident("path").then(|| text.value())
                    });
                    facts.modules.push((item.ident.to_string(), path));
                }
                syn::Item::Use(item) => facts.read_use(&item.tree, &mut Vec::new()),
                syn::Item::Struct(item) => {
                    facts.defines.insert(item.ident.to_string());
                }
                syn::Item::Enum(item) => {
                    facts.defines.insert(item.ident.to_string());
                }
                syn::Item::Union(item) => {
                    facts.defines.insert(item.ident.to_string());
                }
                syn::Item::Type(item) => {
                    facts.defines.insert(item.ident.to_string());
                }
                _ => {}
            }
        }
        facts
    }

    fn read_use(&mut self, tree: &syn::UseTree, prefix: &mut Vec<String>) {
        match tree {
            syn::UseTree::Path(path) => {
                prefix.push(path.ident.to_string());
                self.read_use(&path.tree, prefix);
                prefix.pop();
            }
            syn::UseTree::Name(name) => {
                let mut path = prefix.clone();
                path.push(name.ident.to_string());
                self.imports.insert(name.ident.to_string(), path);
            }
            syn::UseTree::Rename(rename) => {
                let mut path = prefix.clone();
                path.push(rename.ident.to_string());
                self.imports.insert(rename.rename.to_string(), path);
            }
            syn::UseTree::Glob(_) => self.globs.push(prefix.clone()),
            syn::UseTree::Group(group) => {
                for tree in &group.items {
                    self.read_use(tree, prefix);
                }
            }
        }
    }
}

/// Whether a `use` path comes from one of `crates`, Rullst crates named as
/// in [`super::alpha6_non_exhaustive`]: `rullst_<crate>::..`, or a `rullst`
/// path naming the crate (any `rullst` path for Core).
fn from_rullst(path: &[String], crates: &[&str]) -> bool {
    let Some((first, rest)) = path.split_first() else {
        return false;
    };
    crates.iter().any(|krate| {
        first.strip_prefix("rullst_") == Some(*krate)
            || (first == "rullst" && (*krate == "core" || rest.iter().any(|part| part == krate)))
    })
}

/// Where a name comes from, as far as the module tree tells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Origin {
    Rullst,
    Application,
    Unknown,
}

/// A literal reported for review, waiting for the package's module tree.
#[derive(Debug)]
struct Pending {
    file: PathBuf,
    line: usize,
    name: String,
    crates: Vec<&'static str>,
}

/// Collects one package's files and resolves its glob-imported literals.
#[derive(Debug)]
pub(super) struct GlobResolver {
    package_root: PathBuf,
    files: BTreeMap<PathBuf, ModuleFacts>,
    pending: Vec<Pending>,
}

impl GlobResolver {
    pub(super) fn new(package_root: &Path) -> Self {
        Self {
            package_root: normalize(package_root),
            files: BTreeMap::new(),
            pending: Vec::new(),
        }
    }

    /// Records a scanned file and its glob literals: `(line, struct name,
    /// candidate crates)`.
    pub(super) fn add(
        &mut self,
        file: &Path,
        facts: ModuleFacts,
        literals: Vec<(usize, String, Vec<&'static str>)>,
    ) {
        let file = normalize(file);
        self.pending
            .extend(literals.into_iter().map(|(line, name, crates)| Pending {
                file: file.clone(),
                line,
                name,
                crates,
            }));
        self.files.insert(file, facts);
    }

    /// Whether `file`'s `mod name;` items resolve beside it (a crate root or
    /// a `mod.rs`) rather than in a directory named after it.
    fn owns_directory(&self, file: &Path) -> bool {
        let named = |names: &[&str]| {
            file.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| names.contains(&name))
        };
        named(&["lib.rs", "main.rs", "mod.rs", "build.rs"])
            || file.parent().is_some_and(|parent| {
                ["src/bin", "tests", "examples", "benches"]
                    .iter()
                    .any(|folder| parent == normalize(&self.package_root.join(folder)))
            })
    }

    /// The files each `mod name;` of `file` can be, as Rust looks them up.
    fn children(&self, file: &Path, facts: &ModuleFacts) -> Vec<PathBuf> {
        let Some(directory) = file.parent() else {
            return Vec::new();
        };
        let base = match (self.owns_directory(file), file.file_stem()) {
            (true, _) => directory.to_path_buf(),
            (false, Some(stem)) => directory.join(stem),
            (false, None) => return Vec::new(),
        };
        let mut children = Vec::new();
        for (name, path) in &facts.modules {
            match path {
                Some(path) => children.push(normalize(&directory.join(path))),
                None => {
                    children.push(base.join(format!("{name}.rs")));
                    children.push(base.join(name).join("mod.rs"));
                }
            }
        }
        children
    }

    /// Where `name` comes from in the module of `file`: an explicit import or
    /// definition first, as Rust shadows glob imports with them, then the
    /// globs.
    fn origin(
        &self,
        file: &Path,
        name: &str,
        crates: &[&str],
        parents: &BTreeMap<PathBuf, Vec<PathBuf>>,
        depth: usize,
    ) -> Origin {
        let Some(facts) = self.files.get(file) else {
            return Origin::Unknown;
        };
        let parent = || match parents.get(file).map(Vec::as_slice) {
            Some([parent]) if depth < MAX_DEPTH => {
                self.origin(parent, name, crates, parents, depth + 1)
            }
            _ => Origin::Unknown,
        };
        if let Some(path) = facts.imports.get(name) {
            return match path.first().map(String::as_str) {
                Some("super") if path.len() == 2 => parent(),
                Some("crate" | "self" | "super") => Origin::Unknown,
                _ if from_rullst(path, crates) => Origin::Rullst,
                Some(first) if first == "rullst" || first.starts_with("rullst_") => Origin::Unknown,
                _ => Origin::Application,
            };
        }
        if facts.defines.contains(name) {
            return Origin::Application;
        }
        if facts.globs.iter().any(|glob| from_rullst(glob, crates)) {
            return Origin::Rullst;
        }
        if facts.globs.iter().any(|glob| glob == &["super"]) {
            return parent();
        }
        Origin::Unknown
    }

    /// Turns each pending review finding into a must-change finding when the
    /// name comes from the struct's Rullst crate, and drops it when it comes
    /// from the application. Findings a marker already removed stay removed.
    pub(super) fn finish(self, findings: &mut Vec<SourceFinding>) {
        let mut parents: BTreeMap<PathBuf, Vec<PathBuf>> = BTreeMap::new();
        for (file, facts) in &self.files {
            for child in self.children(file, facts) {
                parents.entry(child).or_default().push(file.clone());
            }
        }
        for files in parents.values_mut() {
            files.dedup();
        }
        for pending in &self.pending {
            let origin = self.origin(&pending.file, &pending.name, &pending.crates, &parents, 0);
            let at = |finding: &SourceFinding| {
                finding.rule.code == NON_EXHAUSTIVE_LITERAL_GLOB.code
                    && finding.line == pending.line
                    && normalize(&finding.path) == pending.file
            };
            match origin {
                Origin::Rullst => {
                    for finding in findings.iter_mut().filter(|finding| at(finding)) {
                        finding.rule = &NON_EXHAUSTIVE_LITERAL;
                    }
                }
                Origin::Application => findings.retain(|finding| !at(finding)),
                Origin::Unknown => {}
            }
        }
    }
}

/// `path` with `.` and `..` resolved lexically, as `#[path]` may use them.
fn normalize(path: &Path) -> PathBuf {
    let mut normal = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normal.pop();
            }
            other => normal.push(other.as_os_str()),
        }
    }
    normal
}
