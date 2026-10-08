//! Bounded, link-free traversal of project Rust sources for static scanners.
//!
//! The walk never follows a symlink or reads a special file, so a link cycle
//! (`src/shared -> ..`) or a link to `/` cannot make it unbounded. Reaching the
//! depth or entry bound is reported, so a security scan can count itself as
//! incomplete instead of silently clean.

use std::fs;
use std::path::{Path, PathBuf};

/// Directory names the scanners never enter.
const SKIPPED_DIRECTORIES: [&str; 4] = ["target", ".git", ".agents", ".codex"];

#[derive(Clone, Copy, Debug)]
pub(crate) struct WalkLimits {
    max_depth: usize,
    max_entries: usize,
}

impl WalkLimits {
    /// Generous for real projects while bounding a hostile or broken tree.
    pub(crate) const DEFAULT: Self = Self {
        max_depth: 64,
        max_entries: 250_000,
    };
}

#[derive(Debug, Default)]
pub(crate) struct RustSources {
    /// Regular `.rs` files found under the root, sorted.
    pub(crate) files: Vec<PathBuf>,
    /// Why the walk did not cover the whole tree, when it did not.
    pub(crate) incomplete: Option<String>,
}

/// Lists regular `.rs` files under `root` without following symlinks.
pub(crate) fn rust_sources(root: &Path) -> RustSources {
    rust_sources_with(root, WalkLimits::DEFAULT)
}

/// Lists regular files under `root` whose extension is one of `extensions`,
/// with the same bounds and link policy as [`rust_sources`].
pub(crate) fn files_with_extensions(root: &Path, extensions: &[&str]) -> RustSources {
    walk_with(root, WalkLimits::DEFAULT, extensions)
}

fn rust_sources_with(root: &Path, limits: WalkLimits) -> RustSources {
    walk_with(root, limits, &["rs"])
}

fn walk_with(root: &Path, limits: WalkLimits, extensions: &[&str]) -> RustSources {
    let mut walk = Walk {
        limits,
        extensions,
        entries: 0,
        sources: RustSources::default(),
    };
    walk.visit(root, 0);
    walk.sources.files.sort();
    walk.sources
}

struct Walk<'a> {
    limits: WalkLimits,
    extensions: &'a [&'a str],
    entries: usize,
    sources: RustSources,
}

impl Walk<'_> {
    /// Returns `false` once the entry bound stops the whole walk.
    fn visit(&mut self, directory: &Path, depth: usize) -> bool {
        let Ok(entries) = fs::read_dir(directory) else {
            return true;
        };
        for entry in entries.flatten() {
            self.entries = self.entries.saturating_add(1);
            if self.entries > self.limits.max_entries {
                self.sources.incomplete = Some(format!(
                    "the source walk stopped after {} directory entries",
                    self.limits.max_entries
                ));
                return false;
            }
            // `DirEntry::file_type` describes the entry itself: a symlink is
            // reported as a symlink and is neither followed nor read.
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            let path = entry.path();
            if file_type.is_dir() {
                if entry
                    .file_name()
                    .to_str()
                    .is_some_and(|name| SKIPPED_DIRECTORIES.contains(&name))
                {
                    continue;
                }
                if depth >= self.limits.max_depth {
                    self.sources.incomplete.get_or_insert_with(|| {
                        format!(
                            "directories below the {}-level depth bound were not scanned",
                            self.limits.max_depth
                        )
                    });
                    continue;
                }
                if !self.visit(&path, depth + 1) {
                    return false;
                }
            } else if file_type.is_file()
                && path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| self.extensions.contains(&extension))
            {
                self.sources.files.push(path);
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, contents: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn symlink_cycles_and_links_out_of_the_tree_are_not_followed() {
        let project = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let src = project.path().join("src");
        write(&src.join("main.rs"), "fn main() {}\n");
        write(&outside.path().join("secret.rs"), "fn outside() {}\n");
        // Two loop links made the old recursion exponential.
        std::os::unix::fs::symlink("..", src.join("shared")).unwrap();
        std::os::unix::fs::symlink(".", src.join("legacy")).unwrap();
        std::os::unix::fs::symlink(outside.path(), src.join("vendor")).unwrap();
        std::os::unix::fs::symlink(outside.path().join("secret.rs"), src.join("linked.rs"))
            .unwrap();

        let sources = rust_sources(&src);
        assert_eq!(sources.files, vec![src.join("main.rs")]);
        assert!(sources.incomplete.is_none());
    }

    #[test]
    fn skipped_directories_and_non_rust_files_are_excluded() {
        let project = tempfile::tempdir().unwrap();
        write(&project.path().join("src/lib.rs"), "");
        write(&project.path().join("src/nested/mod.rs"), "");
        write(&project.path().join("src/notes.md"), "");
        write(&project.path().join("target/debug/build.rs"), "");
        write(&project.path().join(".git/hooks/hook.rs"), "");

        let sources = rust_sources(project.path());
        assert_eq!(
            sources.files,
            vec![
                project.path().join("src/lib.rs"),
                project.path().join("src/nested/mod.rs"),
            ]
        );
        assert!(sources.incomplete.is_none());
        assert!(
            rust_sources(&project.path().join("missing"))
                .files
                .is_empty()
        );
    }

    #[test]
    fn depth_and_entry_bounds_mark_the_walk_incomplete() {
        let project = tempfile::tempdir().unwrap();
        write(&project.path().join("top.rs"), "");
        write(&project.path().join("a/b/c/deep.rs"), "");

        let shallow = rust_sources_with(
            project.path(),
            WalkLimits {
                max_depth: 1,
                max_entries: 100,
            },
        );
        assert!(shallow.files.contains(&project.path().join("top.rs")));
        assert!(
            !shallow
                .files
                .contains(&project.path().join("a/b/c/deep.rs"))
        );
        assert!(shallow.incomplete.unwrap().contains("1-level depth bound"));

        let crowded = rust_sources_with(
            project.path(),
            WalkLimits {
                max_depth: 64,
                max_entries: 2,
            },
        );
        assert!(crowded.incomplete.unwrap().contains("2 directory entries"));

        let complete = rust_sources(project.path());
        assert_eq!(complete.files.len(), 2);
        assert!(complete.incomplete.is_none());
    }
}
