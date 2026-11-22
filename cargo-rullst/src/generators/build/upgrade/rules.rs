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
//! REVIEW finding's line leaves that location out of the plan, and the rules
//! about production behaviour skip [`test_code`].

mod alpha5_runtime;
mod alpha5_slimming;
mod alpha6_cron;
mod alpha6_defaults;
mod alpha6_deprecated;
mod alpha6_gaps;
mod alpha6_jiff;
mod alpha6_media;
mod alpha6_non_exhaustive;
mod alpha6_passkeys;
mod alpha6_public_deps;
mod api_cleanups;
mod catalog;
mod files;
mod markup;
mod non_exhaustive_globs;
mod rc1_experimental;
mod removed_adapters;
mod reviewed;
mod rust;
mod selection;
mod test_code;
mod tokens;

use super::manifest::ManifestUpgradePlan;
use semver::Version;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub(crate) use catalog::{CATALOG, REMOVED_API_RULES};
pub(crate) use reviewed::MARKER as REVIEWED_MARKER;
pub(crate) use selection::{RuleSelection, release_before};

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
    /// What this location needs beyond the rule's guidance, such as the
    /// rewritten expression a rule could compute.
    pub note: Option<String>,
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

impl SourceScan {
    pub(crate) fn count(&self, kind: FindingKind) -> usize {
        self.findings
            .iter()
            .filter(|finding| finding.rule.kind == kind)
            .count()
    }
}

/// Collects findings for one file: every location of every rule, one finding
/// per rule and line, so a rule used three times in a file is reported three
/// times. The plan lists a rule's lines in one file on one entry.
#[derive(Default)]
struct Collector {
    lines: BTreeMap<(&'static str, usize), &'static Rule>,
    /// A finding's note ([`SourceFinding::note`]), by code and line.
    notes: BTreeMap<(&'static str, usize), String>,
    /// `V13-CREDENTIAL-RATE-LIMIT` at calls of an application function named
    /// like a hasher, by line: reported unless the package defines that
    /// function with its hashing bounded ([`Self::add_bounded`]).
    calls: BTreeMap<usize, &'static str>,
    /// Hasher-named functions of this file whose hashing is bounded.
    bounded: BTreeSet<&'static str>,
    /// `Type::new` calls with their lines: `V13-CONNECT-INFALLIBLE-NEW-REMOVED`
    /// when the package defines `Type` with `define_provider!`
    /// ([`Self::add_provider`]), in this file or another.
    constructors: Vec<(usize, String)>,
    /// Types this file defines with `define_provider!`.
    providers: BTreeSet<String>,
    /// `V13-NON-EXHAUSTIVE-LITERAL-GLOB` literals by line, with the struct's
    /// name and candidate crates, for [`non_exhaustive_globs`].
    glob_literals: Vec<(usize, String, Vec<&'static str>)>,
}

impl Collector {
    fn add(&mut self, rule: &'static Rule, line: usize) {
        self.lines.entry((rule.code, line.max(1))).or_insert(rule);
    }

    /// [`Self::add`] with a note for the location; the first note stays.
    fn add_note(&mut self, rule: &'static Rule, line: usize, note: String) {
        self.add(rule, line);
        self.notes.entry((rule.code, line.max(1))).or_insert(note);
    }

    /// A call of the application's own `callee` at `line`, which needs a
    /// bound unless `callee` is defined with one.
    fn add_call(&mut self, line: usize, callee: &'static str) {
        self.calls.entry(line.max(1)).or_insert(callee);
    }

    /// This file defines `name` (a hasher's name) with its hashing bounded.
    fn add_bounded(&mut self, name: &'static str) {
        self.bounded.insert(name);
    }

    /// A `ty::new(..)` call at `line`, a removed constructor when `ty` is a
    /// `define_provider!` type of the package.
    fn add_constructor(&mut self, ty: &str, line: usize) {
        self.constructors.push((line.max(1), ty.to_string()));
    }

    /// This file defines `name` with `define_provider!`.
    fn add_provider(&mut self, name: String) {
        self.providers.insert(name);
    }

    /// A literal or call of the struct `name`, which one of `crates` may
    /// define, reported for review through a glob import at `line`.
    fn add_glob_literal(&mut self, line: usize, name: String, crates: Vec<&'static str>) {
        self.glob_literals.push((line.max(1), name, crates));
    }

    /// The constructor calls a `define_provider!` in another file may still
    /// resolve, and the providers this file defines; [`Self::finish`] then
    /// resolves none.
    fn take_constructors(&mut self) -> (Vec<(usize, String)>, BTreeSet<String>) {
        (
            std::mem::take(&mut self.constructors),
            std::mem::take(&mut self.providers),
        )
    }

    /// The calls a definition in another file may still resolve, and the
    /// bounded definitions of this file, which then stay out of
    /// [`Self::finish`].
    fn take_calls(&mut self) -> (BTreeMap<usize, &'static str>, BTreeSet<&'static str>) {
        (
            std::mem::take(&mut self.calls),
            std::mem::take(&mut self.bounded),
        )
    }

    /// Removes the findings of the rules about production behaviour on the
    /// lines `test` covers.
    fn drop_test_code(&mut self, test: &test_code::TestLines) {
        self.lines
            .retain(|(_, line), rule| !(test_code::production_only(rule) && test.covers(*line)));
        if test_code::production_only(&catalog::CREDENTIAL_RATE_LIMIT) {
            self.calls.retain(|line, _| !test.covers(*line));
        }
    }

    /// Removes the review locations a marker in `text` covers and returns
    /// their rules; the rule's other locations stay.
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
        self.calls.retain(|line, _| {
            let covered = reviewed::covers(&lines, *line, &catalog::CREDENTIAL_RATE_LIMIT);
            if covered {
                dropped.push(&catalog::CREDENTIAL_RATE_LIMIT);
            }
            !covered
        });
        dropped
    }

    /// Appends the findings by line, then code, with the calls this file's
    /// own bounded definitions do not resolve.
    fn finish(mut self, path: &Path, findings: &mut Vec<SourceFinding>) {
        for (line, callee) in std::mem::take(&mut self.calls) {
            if !self.bounded.contains(callee) {
                self.add(&catalog::CREDENTIAL_RATE_LIMIT, line);
            }
        }
        for (line, ty) in std::mem::take(&mut self.constructors) {
            if self.providers.contains(&ty) {
                self.add(&catalog::CONNECT_INFALLIBLE_NEW_REMOVED, line);
            }
        }
        let mut hits: Vec<_> = self.lines.into_iter().collect();
        hits.sort_by_key(|((code, line), _)| (*line, *code));
        let mut notes = self.notes;
        findings.extend(hits.into_iter().map(|((code, line), rule)| SourceFinding {
            rule,
            path: path.to_path_buf(),
            line,
            note: notes.remove(&(code, line)),
        }));
    }
}

/// Facts about the whole workspace that some rules depend on.
#[derive(Debug, Default)]
pub(super) struct WorkspaceFacts {
    /// A manifest enables SQLx's derive or query macros itself: `derive` or
    /// `macros`, or SQLx's default features, which include `macros`.
    pub sqlx_macros: bool,
    /// A manifest enables Rullst's passkeys (13.0.0-alpha.6): `auth-passkeys`
    /// or a feature that includes it on `rullst`, or `passkeys` (or `sqlite`,
    /// `passkey-postgres`) on `rullst-auth`, in a declaration or `[features]`.
    pub passkeys: bool,
    /// A manifest enables a server-side session registry (13.0.0-alpha.6):
    /// `auth-session-registry-sqlite`, `-postgres` or `-mysql` on `rullst`,
    /// or `session-registry-*` on `rullst-auth`, in a declaration or
    /// `[features]`.
    pub session_registry: bool,
    /// A manifest declares `jsonwebtoken` older than 12, whose `from_secret`
    /// key constructors do not return a `Result`.
    pub jsonwebtoken_below_12: bool,
    /// A manifest declares `chrono` (13.0.0-alpha.6 removed Rullst's
    /// re-export), so a bare `chrono::` path can be the application's own.
    pub chrono_dependency: bool,
    /// A manifest enables SQLx's `chrono` feature itself, which Rullst's ORM
    /// no longer enables since 13.0.0-alpha.6.
    pub sqlx_chrono: bool,
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
    // Calls of application hashers, resolved once every file is read.
    let mut calls: Vec<(PathBuf, usize, &'static str)> = Vec::new();
    let mut bounded: BTreeSet<&'static str> = BTreeSet::new();
    // `Type::new` calls and the `define_provider!` types, likewise.
    let mut constructors: Vec<(PathBuf, usize, String)> = Vec::new();
    let mut providers: BTreeSet<String> = BTreeSet::new();
    let mut mounts_health = false;
    let mut globs = non_exhaustive_globs::GlobResolver::new(package_root);
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
                let literals = std::mem::take(&mut collector.glob_literals);
                globs.add(path, summary.module, literals);
            }
            Err(_) => scan.unscanned.push(path.to_path_buf()),
        }
        scan.reviewed.extend(collector.drop_reviewed(&text));
        let (file_calls, file_bounded) = collector.take_calls();
        calls.extend(
            file_calls
                .into_iter()
                .map(|(line, callee)| (path.to_path_buf(), line, callee)),
        );
        bounded.extend(file_bounded);
        let (file_constructors, file_providers) = collector.take_constructors();
        constructors.extend(
            file_constructors
                .into_iter()
                .map(|(line, ty)| (path.to_path_buf(), line, ty)),
        );
        providers.extend(file_providers);
        collector.finish(path, &mut scan.findings);
    }
    globs.finish(&mut scan.findings);
    scan.findings.extend(
        calls
            .into_iter()
            .filter(|(_, _, callee)| !bounded.contains(callee))
            .map(|(path, line, _)| SourceFinding {
                rule: &catalog::CREDENTIAL_RATE_LIMIT,
                path,
                line,
                note: None,
            }),
    );
    scan.findings.extend(
        constructors
            .into_iter()
            .filter(|(_, _, ty)| providers.contains(ty))
            .map(|(path, line, _)| SourceFinding {
                rule: &catalog::CONNECT_INFALLIBLE_NEW_REMOVED,
                path,
                line,
                note: None,
            }),
    );
    match server_start {
        Some(_) if mounts_health => {}
        Some((_, _, true)) => scan.reviewed.push(&catalog::HEALTH_PROBES),
        Some((path, line, false)) => scan.findings.push(SourceFinding {
            rule: &catalog::HEALTH_PROBES,
            path,
            line,
            note: None,
        }),
        None => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests;
