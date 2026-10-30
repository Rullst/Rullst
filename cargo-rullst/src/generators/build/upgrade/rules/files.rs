//! Rules over Cargo manifests and generated project files. Only regular
//! files of bounded size are read (never through a symlink), and a finding
//! records a line number, never file contents.

use super::catalog::*;
use super::{ManifestUpgradePlan, Rule, SourceFinding, WorkspaceFacts};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[path = "files/features.rs"]
mod features;
use features::{enables_sqlx_macros, nexus_without_assistant};

const MAX_FILE_BYTES: u64 = 1024 * 1024;
const MAX_YAML_ENTRIES: usize = 5_000;
const DEPENDENCY_SECTIONS: [&str; 3] = ["dependencies", "dev-dependencies", "build-dependencies"];

/// The sorted names `root` lists. A fixed name is scanned only when the
/// directory lists it exactly: on a case-insensitive file system (macOS,
/// Windows) `Justfile` would otherwise open `justfile` a second time.
fn root_names(root: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        .take(MAX_YAML_ENTRIES)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();
    names.sort();
    names
}

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

/// A dependency that keeps its default features and selects no SQLx driver:
/// `rullst` and `rullst-orm` defaults included one up to 13.0.0-alpha.2.
fn relies_on_default_driver(declaration: &toml::Value) -> bool {
    let defaults = declaration
        .get("default-features")
        .or_else(|| declaration.get("default_features"))
        .and_then(toml::Value::as_bool)
        .unwrap_or(true);
    let selects_driver = declaration
        .get("features")
        .and_then(toml::Value::as_array)
        .is_some_and(|features| {
            features
                .iter()
                .filter_map(toml::Value::as_str)
                .any(|feature| {
                    matches!(
                        feature,
                        "drivers-all" | "strict-sqlite" | "strict-postgres" | "strict-mysql"
                    )
                })
        });
    defaults && !selects_driver
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

/// The features v13 removed from a Rullst package, with their rule:
/// `V13-POLYGLOT-FEATURE` and `V13-OFFLINE-SYNC-REMOVED`.
fn removed_features(package: &str) -> &'static [(&'static str, &'static Rule)] {
    match package {
        "rullst" => &[
            ("orm-polyglot", &POLYGLOT_FEATURE),
            ("offline-sync", &OFFLINE_SYNC_REMOVED),
        ],
        "rullst-core" => &[("offline-sync", &OFFLINE_SYNC_REMOVED)],
        "rullst-orm" => &[("polyglot", &POLYGLOT_FEATURE)],
        _ => &[],
    }
}

/// The rule of the first removed feature a dependency declaration enables.
fn enabled_removed_feature(package: &str, declaration: &toml::Value) -> Option<&'static Rule> {
    let features = declaration
        .get("features")
        .and_then(toml::Value::as_array)?;
    removed_features(package)
        .iter()
        .find(|(removed, _)| {
            features
                .iter()
                .any(|feature| feature.as_str() == Some(removed))
        })
        .map(|(_, rule)| *rule)
}

/// The line and rule of the first `[features]` entry that enables a removed
/// feature through a dependency key, such as `"rullst/orm-polyglot"`.
fn removed_feature_reference(
    document: &toml::Value,
    text: &str,
    packages: &BTreeMap<String, String>,
) -> Option<(usize, &'static Rule)> {
    let (reference, rule) = document
        .get("features")
        .and_then(toml::Value::as_table)?
        .values()
        .filter_map(toml::Value::as_array)
        .flatten()
        .filter_map(toml::Value::as_str)
        .find_map(|value| {
            let (key, feature) = value.split_once('/')?;
            let key = key.strip_suffix('?').unwrap_or(key);
            let package = packages.get(key)?;
            removed_features(package)
                .iter()
                .find(|(removed, _)| *removed == feature)
                .map(|(_, rule)| (value, *rule))
        })?;
    let line = line_of(text, |line| {
        !line.starts_with('#') && line.contains(reference)
    })
    .unwrap_or(1);
    Some((line, rule))
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
        let declared = dependencies(&document);
        let packages: BTreeMap<String, String> = declared
            .iter()
            .map(|(key, package, _)| (key.clone(), package.clone()))
            .collect();
        for (key, package, declaration) in declared {
            if let Some(rule) = enabled_removed_feature(&package, &declaration) {
                findings.push(finding(
                    rule,
                    plan.path.clone(),
                    dependency_line(&plan.original, &key),
                ));
            }
            let inherited = declaration
                .get("workspace")
                .and_then(toml::Value::as_bool)
                .unwrap_or(false);
            if !inherited && nexus_without_assistant(&package, &declaration) {
                findings.push(finding(
                    &NEXUS_AI_FEATURE,
                    plan.path.clone(),
                    dependency_line(&plan.original, &key),
                ));
            }
            match package.as_str() {
                "sqlx" => facts.sqlx_macros |= enables_sqlx_macros(&declaration),
                "rullst-labs-runner" => findings.push(finding(
                    &LABS_RUNNER,
                    plan.path.clone(),
                    dependency_line(&plan.original, &key),
                )),
                "rullst" | "rullst-orm" if !inherited && relies_on_default_driver(&declaration) => {
                    let rule = if package == "rullst" {
                        &RULLST_DEFAULT_FEATURES
                    } else {
                        &ORM_DEFAULT_FEATURES
                    };
                    findings.push(finding(
                        rule,
                        plan.path.clone(),
                        dependency_line(&plan.original, &key),
                    ));
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
        if let Some((line, rule)) = removed_feature_reference(&document, &plan.original, &packages)
        {
            findings.push(finding(rule, plan.path.clone(), line));
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

/// A script line running a command that 13.0.0-alpha.4 removed (`rullst ai`)
/// or renamed (`rullst footprint`, now `green`), as a whole word.
pub(super) fn runs_removed_cli_command(line: &str) -> bool {
    ["rullst footprint", "rullst ai"].iter().any(|command| {
        line.match_indices(command).any(|(index, _)| {
            line[index + command.len()..]
                .chars()
                .next()
                .is_none_or(|next| {
                    !(next.is_ascii_alphanumeric() || matches!(next, '-' | '_' | ':'))
                })
        })
    })
}

/// A script line running the self-updater removed in 13.0.0-alpha.4:
/// `rullst update check|verify|stage|install`, or `update guided` with its
/// CLI-installation flags.
pub(super) fn runs_removed_update_command(line: &str) -> bool {
    ["check", "verify", "stage", "install"]
        .iter()
        .any(|command| line.contains(&format!("rullst update {command}")))
        || (line.contains("rullst update guided")
            && (line.contains("--scope") || line.contains("--root")))
}

/// Whether `line` passes `flag` as a whole word (`--nix`, not `--nixpkgs`).
fn has_flag(line: &str, flag: &str) -> bool {
    line.split(|character: char| character.is_whitespace() || character == '=')
        .any(|word| word == flag)
}

/// A script line running `deploy` with a platform target removed in
/// 13.0.0-alpha.4 (Fly.io, Railway or Render).
pub(super) fn runs_removed_deploy_target(line: &str) -> bool {
    line.contains("rullst deploy")
        && ["fly", "railway", "render"].iter().any(|platform| {
            ["--platform ", "--platform=", "-p "]
                .iter()
                .any(|option| line.contains(&format!("{option}{platform}")))
        })
}

/// A script line running a generator removed in 13.0.0-alpha.4: Foundry,
/// `nixify`, Buildah, `make:grpc`, or `new --nix|--buildah`.
pub(super) fn runs_removed_generator(line: &str) -> bool {
    [
        "rullst foundry:",
        "rullst nixify",
        "generate:buildah",
        "make:grpc",
    ]
    .iter()
    .any(|command| line.contains(command))
        || (line.contains("rullst new") && (has_flag(line, "--nix") || has_flag(line, "--buildah")))
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
        push(&DEPLOY_GENERATORS_REMOVED, "Foundry.toml", 1);
    }
    // Files the removed deploy targets and generators wrote.
    for file in ["fly.toml", "render.yaml", "railway.json", "railway.toml"] {
        if read_small(&root.join(file)).is_some() {
            push(&DEPLOY_TARGETS_REMOVED, file, 1);
        }
    }
    if read_small(&root.join("flake.nix")).is_some_and(|text| {
        text.contains("Rullst Nix Development Environment")
            || text.contains("description = \"A Rullst Application\"")
    }) {
        push(&DEPLOY_GENERATORS_REMOVED, "flake.nix", 1);
    }
    for file in ["build_buildah.sh", "src/grpc/mod.rs"] {
        if read_small(&root.join(file)).is_some() {
            push(&DEPLOY_GENERATORS_REMOVED, file, 1);
        }
    }
    if let Some(text) = read_small(&root.join("docker-compose.prod.yml"))
        && !text.contains("172.31.250.10")
    {
        push(&VPS_PROXY, "docker-compose.prod.yml", 1);
    }
    // The shell `make:omni` generated: its Tauri configuration or runtime.
    if let Some(shell) = ["omni-app/tauri.conf.json", "omni-app/src/lib.rs"]
        .into_iter()
        .find(|path| read_small(&root.join(path)).is_some())
    {
        push(&OMNI_REMOVED, shell, 1);
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
        if let Some(line) = line_of(&text, removed_billing_setting) {
            push(&CAPITAL_REMOVED, name, line);
        }
    }
    if let Some(text) = read_small(&root.join("Rullst.toml"))
        && let Some(line) = removed_mail_driver_line(&text)
    {
        push(&MAIL_REMOVED, "Rullst.toml", line);
    }
    let mut scripts: Vec<String> = root_names(root)
        .into_iter()
        .filter(|name| matches!(name.as_str(), "Makefile" | "justfile" | "Justfile"))
        .collect();
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
        let Some(text) = read_small(&root.join(&script)) else {
            continue;
        };
        if let Some(line) = line_of(&text, |line| {
            line.contains("rullst omni") || line.contains("make:omni")
        }) {
            push(&OMNI_REMOVED, &script, line);
        }
        if let Some(line) = line_of(&text, runs_removed_cli_command) {
            push(&CLI_AI_REMOVED, &script, line);
        }
        if let Some(line) = line_of(&text, runs_removed_update_command) {
            push(&CLI_SELF_UPDATE, &script, line);
        }
        if let Some(line) = line_of(&text, runs_removed_deploy_target) {
            push(&DEPLOY_TARGETS_REMOVED, &script, line);
        }
        if let Some(line) = line_of(&text, runs_removed_generator) {
            push(&DEPLOY_GENERATORS_REMOVED, &script, line);
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
