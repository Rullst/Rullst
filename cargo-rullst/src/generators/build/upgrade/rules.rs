//! v12 → v13 source rules (`rullst-upgrade-rules-v4`).
//!
//! Rust sources are parsed, never matched as text: [`tokens`] reads the
//! token stream (so comments and doc comments never match, and string
//! literals only match string rules), including macro bodies such as
//! `html!` and `routes!`; [`rust`] walks the syntax tree for structural
//! rules (derive attributes, `match` arms, struct literals) and [`markup`]
//! the `html!` → `Markup` rules, which also read `html!` bodies. [`files`]
//! inspects Cargo manifests and generated project files. Each finding names a
//! stable rule code, a kind, a location and the first-column title of its
//! row in `docs/src/migration-v13.md`. A [`reviewed`] marker on or above a
//! REVIEW finding's line leaves that location out of the plan.

mod alpha5_runtime;
mod alpha5_slimming;
mod api_cleanups;
mod catalog;
mod files;
mod markup;
mod removed_adapters;
mod reviewed;
mod rust;
mod tokens;

use super::manifest::ManifestUpgradePlan;
use semver::Version;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub(crate) use catalog::{CATALOG, REMOVED_API_RULES};
pub(crate) use reviewed::MARKER as REVIEWED_MARKER;

/// Catalog identifier recorded in plans and preparations.
pub(crate) const RULE_CATALOG_VERSION: &str = "rullst-upgrade-rules-v4";
/// The table every finding's `row` refers to.
pub(crate) const MIGRATION_GUIDE_URL: &str = "https://rullst.github.io/Rullst/book/migration-v13.html#changes-from-the-published-1210-source";
/// The target major these rules describe.
const TARGET_MAJOR: u64 = 13;
const MAX_FINDINGS: usize = 10_000;
/// Larger Rust files are listed as unscanned instead of being parsed.
const MAX_SOURCE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_WALK_ENTRIES: usize = 250_000;

/// Whether a finding needs a source change or a behavioural review.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FindingKind {
    /// A compile-breaking or API-shape change in the application's code.
    MustChange,
    /// Changed behaviour of an API, feature or configuration the code uses.
    Review,
}

impl FindingKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::MustChange => "must-change",
            Self::Review => "review",
        }
    }

    /// The label of the human plan.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::MustChange => "MUST-CHANGE",
            Self::Review => "REVIEW",
        }
    }

    /// The `severity` value earlier `rullst.upgrade-plan.v1` consumers read.
    pub(crate) fn severity(self) -> &'static str {
        match self {
            Self::MustChange => "BLOCKER",
            Self::Review => "REVIEW",
        }
    }
}

/// One rule of the catalog.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Rule {
    /// Stable identifier, also the `code` of a finding.
    pub code: &'static str,
    pub kind: FindingKind,
    /// The release that introduced the change, an exact SemVer version.
    pub introduced: &'static str,
    /// First-column title of the row in `docs/src/migration-v13.md`.
    pub row: &'static str,
    /// One line shown in the plan.
    pub message: &'static str,
    /// How to fix it; a failed Cargo gate repeats it for removed-API findings.
    pub guidance: &'static str,
}

impl Rule {
    /// Whether the rule's change is newer than `version`. A release that does
    /// not parse counts as newer, so the rule is never skipped by mistake.
    pub(crate) fn introduced_after(&self, version: &Version) -> bool {
        Version::parse(self.introduced).map_or(true, |introduced| {
            introduced.cmp_precedence(version).is_gt()
        })
    }
}

/// A rule matched at a location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceFinding {
    pub rule: &'static Rule,
    pub path: PathBuf,
    pub line: usize,
}

/// The findings of one scan and the Rust files it could not inspect.
#[derive(Debug, Default)]
pub(crate) struct SourceScan {
    pub findings: Vec<SourceFinding>,
    pub unscanned: Vec<PathBuf>,
    /// Which rules the findings come from.
    pub selection: RuleSelection,
    /// The rule of each review location a `rullst-upgrade: reviewed` marker
    /// left out.
    pub reviewed: Vec<&'static Rule>,
}

/// The rules a plan applies: by default only those introduced after the
/// Rullst version the application already uses.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RuleSelection {
    /// The oldest Rullst version the manifests require or `Cargo.lock` holds.
    pub current: Option<Version>,
    /// Every rule applies, whatever `current` is (`--all-rules`).
    pub all_rules: bool,
    /// Catalog rules introduced in or before `current`, not applied.
    pub skipped_rules: usize,
    /// Findings of the skipped rules, left out of the plan.
    pub hidden_findings: usize,
}

impl SourceScan {
    pub(crate) fn count(&self, kind: FindingKind) -> usize {
        self.findings
            .iter()
            .filter(|finding| finding.rule.kind == kind)
            .count()
    }

    /// Keeps the findings of rules introduced after `current` (all of them
    /// with `all_rules` or an unknown version) and records what was skipped.
    pub(crate) fn select(&mut self, current: Option<Version>, all_rules: bool) {
        let applies = |rule: &Rule| {
            all_rules
                || current
                    .as_ref()
                    .is_none_or(|current| rule.introduced_after(current))
        };
        let found = self.findings.len();
        self.findings.retain(|finding| applies(finding.rule));
        self.reviewed.retain(|rule| applies(rule));
        self.selection = RuleSelection {
            skipped_rules: CATALOG.iter().filter(|rule| !applies(rule)).count(),
            hidden_findings: found - self.findings.len(),
            current,
            all_rules,
        };
    }
}

/// Review rules reported at every location, because each one needs its own
/// decision: a `#[orm(skip)]` field may or may not be a column.
const EVERY_LOCATION: &[&str] = &["V13-ORM-SKIP-MANAGED-COLUMNS"];

/// Collects findings for one file: every must-change location, but only the
/// first location of a review rule (except [`EVERY_LOCATION`]), which keeps
/// the plan readable.
#[derive(Default)]
struct Collector {
    lines: BTreeMap<(&'static str, usize), &'static Rule>,
}

impl Collector {
    fn add(&mut self, rule: &'static Rule, line: usize) {
        self.lines.entry((rule.code, line.max(1))).or_insert(rule);
    }

    /// Removes the review locations a marker in `text` covers, before only
    /// the first location of each rule is kept, and returns their rules.
    fn drop_reviewed(&mut self, text: &str) -> Vec<&'static Rule> {
        let lines: Vec<&str> = text.lines().collect();
        let mut dropped = Vec::new();
        self.lines.retain(|(_, line), rule| {
            let covered = reviewed::covers(&lines, *line, rule);
            if covered {
                dropped.push(*rule);
            }
            !covered
        });
        dropped
    }

    fn finish(self, path: &Path, findings: &mut Vec<SourceFinding>) {
        let mut reviewed = BTreeSet::new();
        let mut hits: Vec<_> = self.lines.into_iter().collect();
        hits.sort_by_key(|((code, line), _)| (*line, *code));
        for ((_, line), rule) in hits {
            if rule.kind == FindingKind::Review
                && !EVERY_LOCATION.contains(&rule.code)
                && !reviewed.insert(rule.code)
            {
                continue;
            }
            findings.push(SourceFinding {
                rule,
                path: path.to_path_buf(),
                line,
            });
        }
    }
}

/// Facts about the whole workspace that some rules depend on.
#[derive(Debug, Default)]
pub(super) struct WorkspaceFacts {
    /// A manifest declares `utoipa` older than 6 (or `utoipa-axum` before 0.3).
    pub utoipa_below_6: bool,
    /// A manifest enables SQLx's derive or query macros itself: `derive` or
    /// `macros`, or SQLx's default features, which include `macros`.
    pub sqlx_macros: bool,
}

/// Scans the workspace when the plan targets v13.
pub(super) fn scan_workspace(
    root: &Path,
    plans: &[ManifestUpgradePlan],
    target_major: u64,
) -> Result<SourceScan, Box<dyn std::error::Error>> {
    let mut scan = SourceScan::default();
    if target_major != TARGET_MAJOR {
        return Ok(scan);
    }
    let (mut manifest_findings, facts) = files::manifest_findings(plans);
    scan.reviewed
        .extend(reviewed::drop_reviewed(&mut manifest_findings));
    scan.findings.extend(manifest_findings);
    let mut package_roots = plans
        .iter()
        .filter(|plan| plan.is_package)
        .filter_map(|plan| plan.path.parent().map(Path::to_path_buf))
        .collect::<Vec<_>>();
    package_roots.sort();
    package_roots.dedup();
    for package_root in &package_roots {
        scan_package(package_root, &facts, &mut scan)?;
    }
    let (mut project, marked) = files::project_findings(root, plans);
    scan.reviewed.extend(marked);
    scan.reviewed.extend(reviewed::drop_reviewed(&mut project));
    scan.findings.extend(project);
    if scan.findings.len() > MAX_FINDINGS {
        return Err(
            "migration findings exceed 10,000 entries; review a smaller project first".into(),
        );
    }
    scan.findings.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then(left.line.cmp(&right.line))
            .then(left.rule.code.cmp(right.rule.code))
    });
    scan.findings.dedup();
    Ok(scan)
}

/// One package's Rust sources (nested packages and build output excluded).
fn scan_package(
    package_root: &Path,
    facts: &WorkspaceFacts,
    scan: &mut SourceScan,
) -> Result<(), Box<dyn std::error::Error>> {
    // The first `Server::new` and whether a marker covers its health rule.
    let mut server_start: Option<(PathBuf, usize, bool)> = None;
    let mut mounts_health = false;
    let mut entries = 0usize;
    for entry in WalkDir::new(package_root)
        .follow_links(false)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(super::scan::included_entry)
    {
        let entry = entry?;
        entries += 1;
        if entries > MAX_WALK_ENTRIES {
            return Err("the source tree exceeds 250,000 entries; review a smaller project".into());
        }
        let path = entry.path();
        if !entry.file_type().is_file()
            || path.extension().and_then(|value| value.to_str()) != Some("rs")
        {
            continue;
        }
        let text = (entry.metadata()?.len() <= MAX_SOURCE_BYTES)
            .then(|| std::fs::read_to_string(path).ok())
            .flatten();
        let Some(text) = text else {
            scan.unscanned.push(path.to_path_buf());
            continue;
        };
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        let context = rust::FileContext {
            text: &text,
            file_name,
            path: path.strip_prefix(package_root).unwrap_or(path),
            facts,
        };
        let mut collector = Collector::default();
        match rust::scan_source(&context, &mut collector) {
            Ok(summary) => {
                if server_start.is_none()
                    && let Some(line) = summary.server_start
                {
                    let lines: Vec<&str> = text.lines().collect();
                    let covered = reviewed::covers(&lines, line, &catalog::HEALTH_PROBES);
                    server_start = Some((path.to_path_buf(), line, covered));
                }
                mounts_health |= summary.mounts_health;
            }
            Err(_) => scan.unscanned.push(path.to_path_buf()),
        }
        scan.reviewed.extend(collector.drop_reviewed(&text));
        collector.finish(path, &mut scan.findings);
    }
    match server_start {
        Some(_) if mounts_health => {}
        Some((_, _, true)) => scan.reviewed.push(&catalog::HEALTH_PROBES),
        Some((path, line, false)) => scan.findings.push(SourceFinding {
            rule: &catalog::HEALTH_PROBES,
            path,
            line,
        }),
        None => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests;
