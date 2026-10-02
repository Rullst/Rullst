//! v12 → v13 source rules (`rullst-upgrade-rules-v4`).
//!
//! Rust sources are parsed, never matched as text: [`tokens`] reads the
//! token stream (so comments and doc comments never match, and string
//! literals only match string rules), including macro bodies such as
//! `html!` and `routes!`; [`rust`] walks the syntax tree for structural
//! rules (derive attributes, `match` arms, struct literals). [`files`]
//! inspects Cargo manifests and generated project files. Each finding names a
//! stable rule code, a kind, a location and the first-column title of its
//! row in `docs/src/migration-v13.md`.

mod catalog;
mod files;
mod rust;
mod tokens;

use super::manifest::ManifestUpgradePlan;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[cfg(test)]
pub(crate) use catalog::CATALOG;

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
    /// First-column title of the row in `docs/src/migration-v13.md`.
    pub row: &'static str,
    /// One line shown in the plan.
    pub message: &'static str,
    /// How to fix it; given to `cargo rullst ai upgrade`.
    pub guidance: &'static str,
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
}

impl SourceScan {
    pub(crate) fn count(&self, kind: FindingKind) -> usize {
        self.findings
            .iter()
            .filter(|finding| finding.rule.kind == kind)
            .count()
    }
}

/// Collects findings for one file: every must-change location, but only the
/// first location of a review rule, which keeps the plan readable.
#[derive(Default)]
struct Collector {
    lines: BTreeMap<(&'static str, usize), &'static Rule>,
}

impl Collector {
    fn add(&mut self, rule: &'static Rule, line: usize) {
        self.lines.entry((rule.code, line.max(1))).or_insert(rule);
    }

    fn finish(self, path: &Path, findings: &mut Vec<SourceFinding>) {
        let mut reviewed = BTreeSet::new();
        let mut hits: Vec<_> = self.lines.into_iter().collect();
        hits.sort_by_key(|((code, line), _)| (*line, *code));
        for ((_, line), rule) in hits {
            if rule.kind == FindingKind::Review && !reviewed.insert(rule.code) {
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
    let (manifest_findings, facts) = files::manifest_findings(plans);
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
    scan.findings.extend(files::project_findings(root, plans));
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
    let mut server_start: Option<(PathBuf, usize)> = None;
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
            facts,
        };
        let mut collector = Collector::default();
        match rust::scan_source(&context, &mut collector) {
            Ok(summary) => {
                if server_start.is_none()
                    && let Some(line) = summary.server_start
                {
                    server_start = Some((path.to_path_buf(), line));
                }
                mounts_health |= summary.mounts_health;
            }
            Err(_) => scan.unscanned.push(path.to_path_buf()),
        }
        collector.finish(path, &mut scan.findings);
    }
    if !mounts_health && let Some((path, line)) = server_start {
        scan.findings.push(SourceFinding {
            rule: &catalog::HEALTH_PROBES,
            path,
            line,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests;
