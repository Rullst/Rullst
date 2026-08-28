use semver::VersionReq;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use toml_edit::{DocumentMut, Formatted, Item, Table, Value};

const RULLST_PACKAGES: &[&str] = &[
    "rullst",
    "rullst-ai",
    "rullst-auth",
    "rullst-capital",
    "rullst-connect",
    "rullst-core",
    "rullst-iot",
    "rullst-macros",
    "rullst-mail",
    "rullst-nexus",
    "rullst-orm",
    "rullst-orm-macros",
    "rullst-security",
    "rullst-studio",
];

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub(super) struct DependencyChange {
    pub key: String,
    pub package: String,
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone)]
pub(super) struct ManifestUpgradePlan {
    pub path: PathBuf,
    pub original: String,
    pub updated: String,
    pub is_package: bool,
    pub matched: usize,
    pub source_majors: BTreeSet<u64>,
    pub changes: Vec<DependencyChange>,
    pub warnings: Vec<String>,
}

pub(super) fn plan_workspace(
    root: &Path,
    target: &str,
) -> Result<Vec<ManifestUpgradePlan>, Box<dyn std::error::Error>> {
    let paths = workspace_manifests(root)?;

    paths
        .into_iter()
        .map(|path| plan_manifest(path, target))
        .collect()
}

#[derive(serde::Deserialize)]
struct CargoMetadata {
    packages: Vec<MetadataPackage>,
    workspace_members: Vec<String>,
}

#[derive(serde::Deserialize)]
struct MetadataPackage {
    id: String,
    manifest_path: PathBuf,
}

fn workspace_manifests(root: &Path) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let root_manifest = root.join("Cargo.toml");
    let output = Command::new("cargo")
        .args([
            "metadata",
            "--no-deps",
            "--format-version",
            "1",
            "--manifest-path",
        ])
        .arg(&root_manifest)
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata could not enumerate the workspace: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )
        .into());
    }

    let metadata = serde_json::from_slice::<CargoMetadata>(&output.stdout)?;
    let members = metadata
        .workspace_members
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>();
    let mut paths = metadata
        .packages
        .into_iter()
        .filter(|package| members.contains(&package.id))
        .map(|package| package.manifest_path)
        .collect::<Vec<_>>();
    paths.push(root_manifest);
    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn plan_manifest(
    path: PathBuf,
    target: &str,
) -> Result<ManifestUpgradePlan, Box<dyn std::error::Error>> {
    if std::fs::symlink_metadata(&path)?.file_type().is_symlink() {
        return Err(format!(
            "refusing to edit symlinked workspace manifest {}",
            path.display()
        )
        .into());
    }
    let original = std::fs::read_to_string(&path)?;
    let mut document = original.parse::<DocumentMut>()?;
    let is_package = document.get("package").and_then(Item::as_table).is_some();
    let mut state = PlanState::new(target);

    update_root_tables(document.as_table_mut(), &mut state);

    Ok(ManifestUpgradePlan {
        path,
        original,
        updated: document.to_string(),
        is_package,
        matched: state.matched,
        source_majors: state.source_majors,
        changes: state.changes,
        warnings: state.warnings,
    })
}

pub(super) fn apply_plans(plans: &[ManifestUpgradePlan]) -> Result<(), Box<dyn std::error::Error>> {
    for plan in plans {
        if plan.original != plan.updated {
            std::fs::write(&plan.path, &plan.updated)?;
        }
    }
    Ok(())
}

struct PlanState<'a> {
    target: &'a str,
    matched: usize,
    source_majors: BTreeSet<u64>,
    changes: Vec<DependencyChange>,
    warnings: Vec<String>,
}

impl<'a> PlanState<'a> {
    fn new(target: &'a str) -> Self {
        Self {
            target,
            matched: 0,
            source_majors: BTreeSet::new(),
            changes: Vec::new(),
            warnings: Vec::new(),
        }
    }
}

fn update_root_tables(root: &mut Table, state: &mut PlanState<'_>) {
    update_dependency_sections(root, state);

    if let Some(workspace) = root.get_mut("workspace").and_then(Item::as_table_mut) {
        update_dependency_section(workspace, "dependencies", state);
    }

    if let Some(targets) = root.get_mut("target").and_then(Item::as_table_mut) {
        for (_, target) in targets.iter_mut() {
            if let Some(target_table) = target.as_table_mut() {
                update_dependency_sections(target_table, state);
            }
        }
    }
}

fn update_dependency_sections(parent: &mut Table, state: &mut PlanState<'_>) {
    for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
        update_dependency_section(parent, section, state);
    }
}

fn update_dependency_section(parent: &mut Table, section: &str, state: &mut PlanState<'_>) {
    let Some(dependencies) = parent.get_mut(section).and_then(Item::as_table_mut) else {
        return;
    };

    for (key, dependency) in dependencies.iter_mut() {
        update_dependency(&key, dependency, state);
    }
}

fn update_dependency(key: &str, dependency: &mut Item, state: &mut PlanState<'_>) {
    let package = dependency_package(key, dependency);
    if !RULLST_PACKAGES.contains(&package.as_str()) {
        return;
    }
    state.matched += 1;

    match dependency {
        Item::Value(Value::String(version)) => {
            replace_version(key, &package, version, state);
        }
        Item::Value(Value::InlineTable(table)) => {
            if let Some(version) = table.get_mut("version") {
                replace_value_version(key, &package, version, state);
            } else if !table
                .get("workspace")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                state.warnings.push(format!(
                    "dependency `{key}` resolves package `{package}` without a version; path/git dependencies require manual review"
                ));
            }
        }
        Item::Table(table) => {
            if let Some(version) = table.get_mut("version") {
                replace_item_version(key, &package, version, state);
            } else if !table
                .get("workspace")
                .and_then(Item::as_bool)
                .unwrap_or(false)
            {
                state.warnings.push(format!(
                    "dependency `{key}` resolves package `{package}` without a version; path/git dependencies require manual review"
                ));
            }
        }
        _ => state.warnings.push(format!(
            "dependency `{key}` uses an unsupported TOML form and was not changed"
        )),
    }
}

fn dependency_package(key: &str, dependency: &Item) -> String {
    match dependency {
        Item::Value(Value::InlineTable(table)) => table
            .get("package")
            .and_then(Value::as_str)
            .unwrap_or(key)
            .to_string(),
        Item::Table(table) => table
            .get("package")
            .and_then(Item::as_str)
            .unwrap_or(key)
            .to_string(),
        _ => key.to_string(),
    }
}

fn replace_value_version(key: &str, package: &str, value: &mut Value, state: &mut PlanState<'_>) {
    if let Value::String(version) = value {
        replace_version(key, package, version, state);
    } else {
        state
            .warnings
            .push(format!("dependency `{key}` has a non-string version"));
    }
}

fn replace_item_version(key: &str, package: &str, item: &mut Item, state: &mut PlanState<'_>) {
    if let Some(value) = item.as_value_mut() {
        replace_value_version(key, package, value, state);
    } else {
        state
            .warnings
            .push(format!("dependency `{key}` has a non-value version"));
    }
}

fn replace_version(
    key: &str,
    package: &str,
    current: &mut Formatted<String>,
    state: &mut PlanState<'_>,
) {
    if let Ok(requirement) = VersionReq::parse(current.value())
        && let Some(comparator) = requirement.comparators.first()
    {
        state.source_majors.insert(comparator.major);
    }
    if current.value() == state.target {
        return;
    }
    let previous = current.value().clone();
    let decoration = current.decor().clone();
    let mut replacement = Formatted::new(state.target.to_string());
    *replacement.decor_mut() = decoration;
    *current = replacement;
    state.changes.push(DependencyChange {
        key: key.to_string(),
        package: package.to_string(),
        from: previous,
        to: state.target.to_string(),
    });
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn plan(input: &str) -> ManifestUpgradePlan {
        let root =
            std::env::temp_dir().join(format!("rullst-upgrade-manifest-{}", rand::random::<u64>()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("Cargo.toml");
        std::fs::write(&path, input).unwrap();
        let plan = plan_manifest(path, "12.0.0-rc.1").unwrap();
        std::fs::remove_dir_all(root).unwrap();
        plan
    }

    #[test]
    fn updates_strings_inline_tables_renames_workspace_and_targets() {
        let result = plan(
            r#"[dependencies]
rullst = "5"
core_alias = { package = "rullst-core", version = "5.0", default-features = false }
local_ai = { package = "rullst-ai", path = "../ai" }

[workspace.dependencies]
rullst-auth = { version = "5.0.0" }

[target.'cfg(unix)'.dev-dependencies]
rullst-security = "5.0.0"
"#,
        );

        assert_eq!(result.matched, 5);
        assert_eq!(result.changes.len(), 4);
        assert!(result.updated.contains("rullst = \"12.0.0-rc.1\""));
        assert!(
            result
                .updated
                .contains("package = \"rullst-core\", version = \"12.0.0-rc.1\"")
        );
        assert!(
            result
                .updated
                .contains("rullst-auth = { version = \"12.0.0-rc.1\" }")
        );
        assert!(result.updated.contains("rullst-security = \"12.0.0-rc.1\""));
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("local_ai"));
    }

    #[test]
    fn preserves_comments_and_unrelated_dependencies() {
        let input =
            "[dependencies]\n# framework\nrullst = \"5\" # keep this comment\naxum = \"0.8\"\n";
        let result = plan(input);
        assert!(result.updated.contains("# framework"));
        assert!(result.updated.contains("# keep this comment"));
        assert!(result.updated.contains("axum = \"0.8\""));
    }
}
