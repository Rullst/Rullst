use super::manifest::ManifestUpgradePlan;
use chrono::Utc;
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

#[path = "backup/restore.rs"]
mod restore;

#[derive(Debug)]
pub(super) struct UpgradeBackup {
    project_root: PathBuf,
    root: PathBuf,
    report_path: PathBuf,
}

impl UpgradeBackup {
    pub(super) fn create(
        project_root: &Path,
        plans: &[ManifestUpgradePlan],
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let project_root = project_root.canonicalize()?;
        let run_id = format!(
            "{}-{:016x}",
            Utc::now().format("%Y%m%dT%H%M%SZ"),
            rand::random::<u64>()
        );
        let root = project_root
            .join("target")
            .join("rullst-upgrades")
            .join(run_id);
        let files_root = root.join("files");
        restore::validate_directory_creation(&project_root, Path::new("target/rullst-upgrades"))?;
        let mut builder = std::fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(&files_root)?;

        let mut originals = plans
            .iter()
            .map(|plan| plan.path.clone())
            .collect::<Vec<_>>();
        originals.push(project_root.join("Cargo.lock"));
        for package_root in plans
            .iter()
            .filter(|plan| plan.is_package)
            .filter_map(|plan| plan.path.parent())
        {
            originals.extend(rust_sources(package_root)?);
        }
        originals.sort();
        originals.dedup();
        if originals.len() > restore::MAX_ENTRIES {
            return Err("upgrade snapshot exceeds the file count limit".into());
        }

        let mut index = String::new();
        let mut total_bytes = 0u64;
        for original in originals {
            let relative = original.strip_prefix(&project_root)?;
            restore::validate_relative_restore_path(relative)?;
            let metadata = restore::validate_file(&project_root, relative)?;
            if let Some(metadata) = metadata {
                total_bytes = total_bytes
                    .checked_add(metadata.len())
                    .ok_or("upgrade snapshot size overflow")?;
                if metadata.len() > restore::MAX_FILE_BYTES
                    || total_bytes > restore::MAX_TOTAL_BYTES
                {
                    return Err("upgrade snapshot exceeds 64 MiB/file or 512 MiB/backup".into());
                }
                let snapshot = files_root.join(relative);
                if let Some(parent) = snapshot.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::copy(&original, &snapshot)?;
                index.push_str(&format!("present\t{}\n", relative.display()));
            } else {
                if relative != Path::new("Cargo.lock") {
                    return Err(
                        "only the root Cargo.lock may be absent from an upgrade snapshot".into(),
                    );
                }
                index.push_str(&format!("absent\t{}\n", relative.display()));
            }
            if index.len() as u64 > restore::MAX_INDEX_BYTES {
                return Err("upgrade snapshot index exceeds 8 MiB".into());
            }
        }
        std::fs::write(root.join("index.tsv"), index)?;
        let report_path = root.join("report.md");

        Ok(Self {
            project_root,
            root,
            report_path,
        })
    }

    pub(super) fn restore(&self) -> Result<(), Box<dyn std::error::Error>> {
        Self::restore_from(&self.project_root, &self.root)?;
        Ok(())
    }

    pub(super) fn restore_from(
        project_root: &Path,
        requested: &Path,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        Ok(restore::restore_from(project_root, requested)?)
    }

    pub(super) fn write_reports(
        &self,
        markdown: &str,
        json: &str,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        std::fs::write(&self.report_path, markdown)?;
        std::fs::write(self.root.join("report.json"), json)?;
        Ok(self.report_path.clone())
    }

    pub(super) fn root(&self) -> &Path {
        &self.root
    }

    pub(super) fn report_path(&self) -> &Path {
        &self.report_path
    }
}

fn rust_sources(project_root: &Path) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut sources = Vec::new();
    for entry in WalkDir::new(project_root)
        .follow_links(false)
        .into_iter()
        .filter_entry(included_entry)
    {
        let entry = entry?;
        let is_rust = entry.path().extension().and_then(|value| value.to_str()) == Some("rs");
        if is_rust && entry.file_type().is_symlink() {
            return Err(format!(
                "refusing to upgrade a workspace with symlinked Rust source {}",
                entry.path().display()
            )
            .into());
        }
        if is_rust && entry.file_type().is_file() {
            sources.push(entry.into_path());
        }
    }
    Ok(sources)
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
    fn restores_existing_files_and_removes_a_created_lockfile() {
        let root =
            std::env::temp_dir().join(format!("rullst-upgrade-backup-{}", rand::random::<u64>()));
        std::fs::create_dir_all(root.join("src")).unwrap();
        let manifest_path = root.join("Cargo.toml");
        std::fs::write(&manifest_path, "[dependencies]\nrullst = \"5\"\n").unwrap();
        std::fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();
        let plans = vec![ManifestUpgradePlan {
            path: manifest_path.clone(),
            original: String::new(),
            updated: String::new(),
            is_package: true,
            matched: 1,
            source_majors: std::collections::BTreeSet::new(),
            changes: Vec::new(),
            warnings: Vec::new(),
        }];

        let backup = UpgradeBackup::create(&root, &plans).unwrap();
        std::fs::write(&manifest_path, "changed").unwrap();
        std::fs::write(root.join("src/main.rs"), "changed").unwrap();
        std::fs::write(root.join("Cargo.lock"), "created").unwrap();
        backup.restore().unwrap();

        assert_eq!(
            std::fs::read_to_string(manifest_path).unwrap(),
            "[dependencies]\nrullst = \"5\"\n"
        );
        assert_eq!(
            std::fs::read_to_string(root.join("src/main.rs")).unwrap(),
            "fn main() {}\n"
        );
        assert!(!root.join("Cargo.lock").exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn persisted_backup_can_be_restored_but_cannot_escape_the_project() {
        let root = std::env::temp_dir().join(format!(
            "rullst-upgrade-persisted-backup-{}",
            rand::random::<u64>()
        ));
        std::fs::create_dir_all(root.join("src")).unwrap();
        let manifest_path = root.join("Cargo.toml");
        std::fs::write(&manifest_path, "[dependencies]\nrullst = \"5\"\n").unwrap();
        std::fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();
        let plans = vec![ManifestUpgradePlan {
            path: manifest_path.clone(),
            original: String::new(),
            updated: String::new(),
            is_package: true,
            matched: 1,
            source_majors: std::collections::BTreeSet::new(),
            changes: Vec::new(),
            warnings: Vec::new(),
        }];
        let backup = UpgradeBackup::create(&root, &plans).unwrap();
        let backup_root = backup.root().to_path_buf();
        std::fs::write(&manifest_path, "changed").unwrap();

        UpgradeBackup::restore_from(&root, &backup_root).unwrap();
        assert_eq!(
            std::fs::read_to_string(&manifest_path).unwrap(),
            "[dependencies]\nrullst = \"5\"\n"
        );
        assert!(UpgradeBackup::restore_from(&root, root.parent().unwrap()).is_err());
        std::fs::write(backup_root.join("index.tsv"), "present\t../Cargo.toml\n").unwrap();
        assert!(UpgradeBackup::restore_from(&root, &backup_root).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
