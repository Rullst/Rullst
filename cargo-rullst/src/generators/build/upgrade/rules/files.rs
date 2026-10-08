//! Rules over Cargo manifests and generated project files. Only regular
//! files of bounded size are read (never through a symlink), and a finding
//! records a line number, never file contents.

use super::catalog::*;
use super::{ManifestUpgradePlan, Rule, SourceFinding, WorkspaceFacts};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const MAX_FILE_BYTES: u64 = 1024 * 1024;
const MAX_YAML_ENTRIES: usize = 5_000;
const DEPENDENCY_SECTIONS: [&str; 3] = ["dependencies", "dev-dependencies", "build-dependencies"];

fn finding(rule: &'static Rule, path: PathBuf, line: usize) -> SourceFinding {
    SourceFinding {
        rule,
        path,
        line: line.max(1),
    }
}

/// A regular, bounded UTF-8 file; `None` for anything else.
fn read_small(path: &Path) -> Option<String> {
    let metadata = std::fs::symlink_metadata(path).ok()?;
    if !metadata.is_file() || metadata.len() > MAX_FILE_BYTES {
        return None;
    }
    std::fs::read_to_string(path).ok()
}

/// The 1-based line of the first line satisfying `matches`.
fn line_of(text: &str, matches: impl Fn(&str) -> bool) -> Option<usize> {
    text.lines()
        .position(|line| matches(line.trim()))
        .map(|index| index + 1)
}

/// The line declaring dependency `key` (`key = ...` or `[...dependencies.key]`).
fn dependency_line(text: &str, key: &str) -> usize {
    line_of(text, |line| {
        let declared = line
            .strip_prefix(key)
            .or_else(|| {
                line.strip_prefix('"')
                    .and_then(|rest| rest.strip_prefix(key))
                    .and_then(|rest| rest.strip_prefix('"'))
            })
            .is_some_and(|rest| rest.trim_start().starts_with('='));
        declared || (line.starts_with('[') && line.ends_with(&format!("dependencies.{key}]")))
    })
    .unwrap_or(1)
}

/// `(key, package, declaration)` for every dependency of a manifest.
fn dependencies(document: &toml::Value) -> Vec<(String, String, toml::Value)> {
    let mut tables = Vec::new();
    for section in DEPENDENCY_SECTIONS {
        tables.extend(document.get(section).and_then(toml::Value::as_table));
    }
    if let Some(targets) = document.get("target").and_then(toml::Value::as_table) {
        for target in targets.values() {
            for section in DEPENDENCY_SECTIONS {
                tables.extend(target.get(section).and_then(toml::Value::as_table));
            }
        }
    }
    tables.extend(
        document
            .get("workspace")
            .and_then(|workspace| workspace.get("dependencies"))
            .and_then(toml::Value::as_table),
    );
    tables
        .into_iter()
        .flat_map(|table| table.iter())
        .map(|(key, value)| {
            let package = value
                .get("package")
                .and_then(toml::Value::as_str)
                .unwrap_or(key)
                .to_string();
            (key.clone(), package, value.clone())
        })
        .collect()
}

/// The lowest major and minor a declaration's requirement admits.
fn minimum(declaration: &toml::Value) -> Option<(u64, u64)> {
    let requirement = declaration
        .as_str()
        .or_else(|| declaration.get("version").and_then(toml::Value::as_str))?;
    let parsed = semver::VersionReq::parse(requirement).ok()?;
    let comparator = parsed.comparators.first()?;
    Some((comparator.major, comparator.minor.unwrap_or(0)))
}

/// Manifest findings and the workspace facts that Rust rules depend on.
pub(super) fn manifest_findings(
    plans: &[ManifestUpgradePlan],
) -> (Vec<SourceFinding>, WorkspaceFacts) {
    let mut findings = Vec::new();
    let mut facts = WorkspaceFacts::default();
    for plan in plans {
        let Ok(document) = toml::from_str::<toml::Value>(&plan.original) else {
            continue;
        };
        for (key, package, declaration) in dependencies(&document) {
            let inherited = declaration
                .get("workspace")
                .and_then(toml::Value::as_bool)
                .unwrap_or(false);
            match package.as_str() {
                "rullst-labs-runner" => findings.push(finding(
                    &LABS_RUNNER,
                    plan.path.clone(),
                    dependency_line(&plan.original, &key),
                )),
                "rullst-orm" if !inherited => {
                    let defaults = declaration
                        .get("default-features")
                        .or_else(|| declaration.get("default_features"))
                        .and_then(toml::Value::as_bool)
                        .unwrap_or(true);
                    if defaults {
                        findings.push(finding(
                            &ORM_DEFAULT_FEATURES,
                            plan.path.clone(),
                            dependency_line(&plan.original, &key),
                        ));
                    }
                }
                "utoipa" => {
                    facts.utoipa_below_6 |=
                        minimum(&declaration).is_some_and(|(major, _)| major < 6);
                }
                "utoipa-axum" => {
                    facts.utoipa_below_6 |=
                        minimum(&declaration).is_some_and(|(major, minor)| major == 0 && minor < 3);
                }
                _ => {}
            }
        }
    }
    (findings, facts)
}

/// The root package name, when the root manifest declares a package.
fn package_name(root: &Path, plans: &[ManifestUpgradePlan]) -> Option<(PathBuf, String, usize)> {
    let manifest = root.join("Cargo.toml");
    let plan = plans.iter().find(|plan| plan.path == manifest)?;
    let document = toml::from_str::<toml::Value>(&plan.original).ok()?;
    let name = document.get("package")?.get("name")?.as_str()?.to_string();
    let line = line_of(&plan.original, |line| {
        line.strip_prefix("name")
            .is_some_and(|rest| rest.trim_start().starts_with('='))
    })
    .unwrap_or(1);
    Some((manifest, name, line))
}

/// The line of a `[mail]` or `[mailer]` `driver` that v13 removed, read like
/// the `Mail` facade reads `Rullst.toml`.
fn removed_mail_driver_line(text: &str) -> Option<usize> {
    let mut in_mail = false;
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.starts_with('[') {
            in_mail = line == "[mail]" || line == "[mailer]";
        } else if in_mail
            && let Some((key, value)) = line.split_once('=')
            && key.trim() == "driver"
            && is_removed_mail_driver(value.split('#').next().unwrap_or(value))
        {
            return Some(index + 1);
        }
    }
    None
}

/// A lowercase DNS label, as Kubernetes object names require.
fn dns_label(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 55
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && !name.starts_with('-')
        && !name.ends_with('-')
}

/// Findings in generated project files at the workspace root.
pub(super) fn project_findings(root: &Path, plans: &[ManifestUpgradePlan]) -> Vec<SourceFinding> {
    let mut findings = Vec::new();
    let mut push = |rule: &'static Rule, relative: &str, line: usize| {
        findings.push(finding(rule, root.join(relative), line));
    };
    let gitignore = read_small(&root.join(".gitignore"));
    // A generated database block (`*.sqlite`) without the journal files.
    if let Some(text) = &gitignore
        && text.contains("*.sqlite")
        && !text.contains("*.sqlite-wal")
    {
        push(&GITIGNORE_DATABASES, ".gitignore", 1);
    }
    if let Some(text) = read_small(&root.join(".cargo/config.toml"))
        && let Some(line) = line_of(&text, |line| line.contains("fuse-ld"))
        && !gitignore
            .as_deref()
            .is_some_and(|ignored| ignored.contains(".cargo/config.toml"))
    {
        push(&LINKER_CONFIG, ".cargo/config.toml", line);
    }
    // Files that only the 12.0 CLI generated: an MSVC flag the linker does
    // not support, an ignored lockfile and an unlocked container build.
    if let Some(text) = read_small(&root.join(".cargo/config.toml"))
        && let Some(line) = line_of(&text, |line| {
            !line.trim_start().starts_with('#') && line.contains("/DEBUG:FASTLINK")
        })
    {
        push(&MSVC_FASTLINK, ".cargo/config.toml", line);
    }
    if let Some(text) = &gitignore
        && let Some(line) = line_of(text, |line| {
            matches!(line.trim(), "Cargo.lock" | "/Cargo.lock" | "**/Cargo.lock")
        })
    {
        push(&CARGO_LOCK_IGNORED, ".gitignore", line);
    }
    if let Some(text) = read_small(&root.join("Dockerfile"))
        && let Some(line) = line_of(&text, |line| {
            let line = line.trim_start();
            !line.starts_with('#')
                && line.contains("cargo build")
                && !line.contains("--locked")
                && !line.contains("--frozen")
        })
    {
        push(&DOCKER_UNLOCKED_BUILD, "Dockerfile", line);
    }
    if let Some(text) = read_small(&root.join(".dockerignore"))
        && !(text.contains(".env.*") && text.contains("*.sqlite"))
    {
        push(&DOCKER_CONTEXT, ".dockerignore", 1);
    }
    if read_small(&root.join("Foundry.toml")).is_some() {
        push(&FOUNDRY_SERVICE, "Foundry.toml", 1);
    }
    if let Some(text) = read_small(&root.join("docker-compose.prod.yml"))
        && !text.contains("172.31.250.10")
    {
        push(&VPS_PROXY, "docker-compose.prod.yml", 1);
    }
    if let Some(text) = read_small(&root.join("omni-app/src/lib.rs")) {
        if !text.contains("Launching Omni interface")
            && let Some(line) = line_of(&text, |line| line.contains("tauri::Builder"))
        {
            push(&OMNI_RUNNER, "omni-app/src/lib.rs", line);
        }
        if let Some(line) = line_of(&text, |line| line.contains("\"../Cargo.toml\"")) {
            push(&OMNI_BACKEND, "omni-app/src/lib.rs", line);
        }
    }
    if let Some(text) = read_small(&root.join("rullst-client.ts"))
        && !text.contains("X-CSRF-Token")
    {
        push(&TS_CLIENT_REGENERATE, "rullst-client.ts", 1);
    }
    for name in [".env", ".env.example"] {
        let Some(text) = read_small(&root.join(name)) else {
            continue;
        };
        if let Some(line) = line_of(&text, |line| line.starts_with("OTEL_EXPORTER_OTLP_")) {
            push(&OTLP_ENVIRONMENT, name, line);
        }
        if let Some(line) = line_of(&text, removed_mail_setting) {
            push(&MAIL_REMOVED, name, line);
        }
    }
    if let Some(text) = read_small(&root.join("Rullst.toml"))
        && let Some(line) = removed_mail_driver_line(&text)
    {
        push(&MAIL_REMOVED, "Rullst.toml", line);
    }
    let mut scripts = vec![
        "Makefile".to_string(),
        "justfile".to_string(),
        "Justfile".to_string(),
    ];
    let mut kubernetes = read_small(&root.join("build_buildah.sh")).is_some();
    for entry in WalkDir::new(root)
        .max_depth(4)
        .follow_links(false)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|entry| {
            entry.depth() == 0
                || !matches!(
                    entry.file_name().to_str(),
                    Some("target" | ".git" | "node_modules" | "vendor" | ".rullst")
                )
        })
        .filter_map(Result::ok)
        .take(MAX_YAML_ENTRIES)
    {
        let Ok(relative) = entry.path().strip_prefix(root) else {
            continue;
        };
        let relative = relative.to_string_lossy().replace('\\', "/");
        let extension = entry.path().extension().and_then(|value| value.to_str());
        let in_workflows = relative.starts_with(".github/workflows/");
        if matches!(extension, Some("yml" | "yaml")) && !in_workflows {
            let Some(text) = read_small(entry.path()) else {
                continue;
            };
            kubernetes |= text.contains("apiVersion:") && text.contains("kind:");
            if let Some(line) = line_of(&text, |line| line.contains("kubernetes.io/ingress.class"))
            {
                push(&K8S_INGRESS_TLS, &relative, line);
            }
        } else if in_workflows || (entry.depth() == 1 && extension == Some("sh")) {
            scripts.push(relative);
        }
    }
    for script in scripts {
        if let Some(text) = read_small(&root.join(&script))
            && let Some(line) = line_of(&text, |line| {
                line.contains("omni android") && line.contains("--release")
            })
        {
            push(&ANDROID_RELEASE, &script, line);
        }
    }
    if kubernetes
        && let Some((manifest, name, line)) = package_name(root, plans)
        && !dns_label(&name)
    {
        findings.push(finding(&K8S_NAMES, manifest, line));
    }
    findings
}
