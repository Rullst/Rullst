//! Reuse the migration catalog/editor on an already isolated source snapshot.
use std::path::{Path, PathBuf};

pub(crate) fn prepare_manifests(
    root: &Path,
    paths: Vec<PathBuf>,
    target: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    plan_manifests(root, paths, target, true)
}

fn plan_manifests(
    root: &Path,
    paths: Vec<PathBuf>,
    target: &str,
    apply: bool,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let version = super::target_version(Some(target))?;
    let root = root.canonicalize()?;
    let mut plans = Vec::new();
    for path in paths {
        let path = path.canonicalize()?;
        if !path.starts_with(&root) || path.file_name().is_none_or(|name| name != "Cargo.toml") {
            return Err("workspace members must stay inside the prepared project".into());
        }
        plans.push(super::manifest::plan_manifest(path, target)?);
    }
    if plans.iter().map(|plan| plan.matched).sum::<usize>() == 0 {
        return Err("no versioned Rullst dependencies were found".into());
    }
    if plans.iter().any(|plan| !plan.warnings.is_empty()) {
        return Err("path/git or unsupported dependency declarations require manual review".into());
    }
    for change in plans.iter().flat_map(|plan| &plan.changes) {
        let requirement = semver::VersionReq::parse(&change.from)?;
        let [comparator] = requirement.comparators.as_slice() else {
            return Err("ambiguous dependency requirements require manual review".into());
        };
        if !matches!(
            comparator.op,
            semver::Op::Exact
                | semver::Op::GreaterEq
                | semver::Op::Caret
                | semver::Op::Tilde
                | semver::Op::Wildcard
        ) {
            return Err(
                "upper-only or exclusive dependency requirements require manual review".into(),
            );
        }
        let mut minimum = semver::Version::new(
            comparator.major,
            comparator.minor.unwrap_or(0),
            comparator.patch.unwrap_or(0),
        );
        minimum.pre = comparator.pre.clone();
        if minimum > version {
            return Err("project preparation cannot downgrade a dependency requirement".into());
        }
    }
    validate_lockfile(&root, &version)?;
    let majors = plans
        .iter()
        .flat_map(|plan| plan.source_majors.iter().copied())
        .collect();
    if std::collections::BTreeSet::is_empty(&majors)
        || majors
            .iter()
            .any(|major| !matches!(major, 5 | 6 | 11 | 12 | 13))
    {
        return Err("the migration catalog covers source majors 5, 6, 11, 12 and 13 only".into());
    }
    let roots = plans
        .iter()
        .filter(|plan| plan.is_package)
        .filter_map(|plan| plan.path.parent().map(Path::to_path_buf))
        .collect::<Vec<_>>();
    let findings = super::scan::scan_workspace(&roots, &majors, version.major)?;
    let mut report: serde_json::Value = serde_json::from_str(&super::render_json_report(
        &root, &version, &plans, &findings,
    )?)?;
    report["automatic_scope"] = serde_json::json!(["workspace dependency manifests in candidate/"]);
    if apply {
        super::manifest::apply_plans(&plans)?;
    }
    Ok(report)
}

pub(crate) fn validate_prepared_manifests(
    before: &Path,
    candidate: &Path,
    paths: &[String],
    target: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let paths: Vec<_> = paths.iter().map(|path| before.join(path)).collect();
    let plan = plan_manifests(before, paths.clone(), target, false)?;
    for path in paths {
        let relative = path.strip_prefix(before)?;
        let expected = super::manifest::plan_manifest(path.clone(), target)?;
        if std::fs::read(candidate.join(relative))? != expected.updated.as_bytes() {
            return Err("candidate manifest changed after preparation; review the original and prepare again".into());
        }
    }
    Ok(plan)
}

pub(crate) fn validate_prepared_resolution(
    root: &Path,
    target: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let document: toml::Value = toml::from_str(&std::fs::read_to_string(root.join("Cargo.lock"))?)?;
    let packages = document
        .get("package")
        .and_then(toml::Value::as_array)
        .ok_or("missing candidate lockfile packages")?;
    let mut matched = false;
    for package in packages {
        let name = package
            .get("name")
            .and_then(toml::Value::as_str)
            .ok_or("invalid candidate package name")?;
        if super::manifest::RULLST_PACKAGES.contains(&name) {
            matched = true;
            if package.get("version").and_then(toml::Value::as_str) != Some(target) {
                return Err("candidate lockfile contains a Rullst package outside the exact selected version".into());
            }
        }
    }
    if !matched {
        return Err("candidate lockfile contains no Rullst packages".into());
    }
    Ok(())
}

fn validate_lockfile(
    root: &Path,
    target: &semver::Version,
) -> Result<(), Box<dyn std::error::Error>> {
    let lock = match std::fs::read_to_string(root.join("Cargo.lock")) {
        Ok(lock) => lock,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    let document: toml::Value = toml::from_str(&lock)?;
    let packages = document
        .get("package")
        .and_then(toml::Value::as_array)
        .ok_or("project lockfile has no package inventory")?;
    for package in packages {
        let name = package
            .get("name")
            .and_then(toml::Value::as_str)
            .ok_or("invalid lockfile package name")?;
        if super::manifest::RULLST_PACKAGES.contains(&name) {
            let version = package
                .get("version")
                .and_then(toml::Value::as_str)
                .ok_or("invalid lockfile package version")?;
            if semver::Version::parse(version)?
                .cmp_precedence(target)
                .is_gt()
            {
                return Err("project preparation cannot downgrade a locked Rullst package".into());
            }
        }
    }
    Ok(())
}
