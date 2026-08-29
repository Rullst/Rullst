use super::manifest::ManifestUpgradePlan;
use chrono::Utc;
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

#[derive(Debug)]
struct BackupEntry {
    original: PathBuf,
    snapshot: Option<PathBuf>,
}

#[derive(Debug)]
pub(super) struct UpgradeBackup {
    root: PathBuf,
    report_path: PathBuf,
    entries: Vec<BackupEntry>,
}

impl UpgradeBackup {
    pub(super) fn create(
        project_root: &Path,
        plans: &[ManifestUpgradePlan],
    ) -> Result<Self, Box<dyn std::error::Error>> {
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
        std::fs::create_dir_all(&files_root)?;

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

        let mut entries = Vec::with_capacity(originals.len());
        let mut index = String::new();
        for original in originals {
            let relative = original.strip_prefix(project_root)?;
            if std::fs::symlink_metadata(&original)
                .is_ok_and(|metadata| metadata.file_type().is_symlink())
            {
                return Err(format!(
                    "refusing to snapshot symlinked upgrade input {}",
                    relative.display()
                )
                .into());
            }
            if original.is_file() {
                let snapshot = files_root.join(relative);
                if let Some(parent) = snapshot.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::copy(&original, &snapshot)?;
                index.push_str(&format!("present\t{}\n", relative.display()));
                entries.push(BackupEntry {
                    original,
                    snapshot: Some(snapshot),
                });
            } else {
                index.push_str(&format!("absent\t{}\n", relative.display()));
                entries.push(BackupEntry {
                    original,
                    snapshot: None,
                });
            }
        }
        std::fs::write(root.join("index.tsv"), index)?;
        let report_path = root.join("report.md");

        Ok(Self {
            root,
            report_path,
            entries,
        })
    }

    pub(super) fn restore(&self) -> Result<(), Box<dyn std::error::Error>> {
        for entry in &self.entries {
            match &entry.snapshot {
                Some(snapshot) => {
                    if let Some(parent) = entry.original.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    std::fs::copy(snapshot, &entry.original)?;
                }
                None if entry.original.is_file() => {
                    std::fs::remove_file(&entry.original)?;
                }
                None => {}
            }
        }
        Ok(())
    }

    pub(super) fn restore_from(
        project_root: &Path,
        requested: &Path,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let project_root = project_root.canonicalize()?;
        let allowed_root = project_root.join("target").join("rullst-upgrades");
        let requested = if requested.is_absolute() {
            requested.to_path_buf()
        } else {
            project_root.join(requested)
        };
        let allowed_root = allowed_root.canonicalize()?;
        let backup_root = requested.canonicalize()?;
        if !backup_root.starts_with(&allowed_root) {
            return Err(
                "backup must be inside this project's target/rullst-upgrades directory".into(),
            );
        }

        let index = std::fs::read_to_string(backup_root.join("index.tsv"))?;
        if index.lines().count() > 100_000 {
            return Err("backup index exceeds the restore entry limit".into());
        }
        let files_root = backup_root.join("files").canonicalize()?;
        for line in index.lines() {
            let (state, relative) = line
                .split_once('\t')
                .ok_or("backup index contains a malformed entry")?;
            let relative = Path::new(relative);
            validate_relative_restore_path(relative)?;
            let original = project_root.join(relative);

            match state {
                "present" => {
                    let snapshot = backup_root.join("files").join(relative).canonicalize()?;
                    if !snapshot.starts_with(&files_root) || !snapshot.is_file() {
                        return Err("backup snapshot escapes the approved files directory".into());
                    }
                    if let Some(parent) = original.parent() {
                        let canonical_parent = parent.canonicalize()?;
                        if !canonical_parent.starts_with(&project_root) {
                            return Err("restore target escapes the project root".into());
                        }
                    }
                    if original.exists() && !original.canonicalize()?.starts_with(&project_root) {
                        return Err("restore target resolves outside the project root".into());
                    }
                    std::fs::copy(snapshot, original)?;
                }
                "absent" if relative == Path::new("Cargo.lock") => {
                    if original.is_file() {
                        std::fs::remove_file(original)?;
                    }
                }
                "absent" => {
                    return Err("only a newly created root Cargo.lock may be removed".into());
                }
                _ => return Err("backup index contains an unknown entry state".into()),
            }
        }
        Ok(backup_root)
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

fn validate_relative_restore_path(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || !path
            .components()
            .all(|component| matches!(component, std::path::Component::Normal(_)))
    {
        return Err("backup index contains an unsafe relative path".into());
    }
    let allowed = path == Path::new("Cargo.lock")
        || path.file_name().and_then(|name| name.to_str()) == Some("Cargo.toml")
        || path.extension().and_then(|extension| extension.to_str()) == Some("rs");
    if !allowed {
        return Err("backup index contains a file outside the upgrade snapshot contract".into());
    }
    Ok(())
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
