use std::path::PathBuf;
use walkdir::{DirEntry, WalkDir};

/// Catalog identifier recorded in plans and preparations. v12 and v13 origins
/// need no source-marker rules; v5/v6/v11-era rules were retired in v13.
pub(super) const RULE_CATALOG_VERSION: &str = "rullst-upgrade-rules-v3";

/// Refuses symlinked Rust sources before a transaction snapshots the workspace,
/// so a backup or restore never follows a link outside the project.
pub(super) fn reject_symlinked_sources(
    package_roots: &[PathBuf],
) -> Result<(), Box<dyn std::error::Error>> {
    for package_root in package_roots {
        for entry in WalkDir::new(package_root)
            .follow_links(false)
            .into_iter()
            .filter_entry(included_entry)
        {
            let entry = entry?;
            let is_rust = entry.path().extension().and_then(|value| value.to_str()) == Some("rs");
            if is_rust && entry.file_type().is_symlink() {
                return Err(format!(
                    "refusing to scan symlinked Rust source {}",
                    entry.path().display()
                )
                .into());
            }
        }
    }
    Ok(())
}

fn included_entry(entry: &DirEntry) -> bool {
    if entry.depth() == 0 || !entry.file_type().is_dir() {
        return true;
    }
    !entry.path().join("Cargo.toml").is_file()
        && !matches!(
            entry.file_name().to_str(),
            Some(".git" | ".rullst" | "node_modules" | "target" | "vendor")
        )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn accepts_regular_sources_and_skips_build_directories() {
        let root =
            std::env::temp_dir().join(format!("rullst-upgrade-scan-{}", rand::random::<u64>()));
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::create_dir_all(root.join("target/generated")).unwrap();
        std::fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();
        std::fs::write(root.join("target/generated/old.rs"), "fn old() {}\n").unwrap();

        reject_symlinked_sources(std::slice::from_ref(&root)).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinked_rust_sources() {
        let root =
            std::env::temp_dir().join(format!("rullst-upgrade-link-{}", rand::random::<u64>()));
        std::fs::create_dir_all(root.join("src")).unwrap();
        let outside = root.with_extension("outside.rs");
        std::fs::write(&outside, "fn outside() {}\n").unwrap();
        std::os::unix::fs::symlink(&outside, root.join("src/linked.rs")).unwrap();

        let error = reject_symlinked_sources(std::slice::from_ref(&root)).unwrap_err();
        assert!(error.to_string().contains("symlinked Rust source"));
        std::fs::remove_dir_all(root).unwrap();
        std::fs::remove_file(outside).unwrap();
    }
}
