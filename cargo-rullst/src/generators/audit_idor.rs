//! Bounded source heuristic for IDOR/BOLA access classification of
//! parameterized routes (`cargo rullst audit --idor`).

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::generators::audit::incomplete_walk_warning;
use crate::generators::audit_source::audit_source;
use crate::generators::source_walk::{RustSources, rust_sources};

#[path = "audit_idor_machine.rs"]
mod machine;
use machine::{Constants, MachineRoutes, route_methods};

const ACCESS_MARKER: &str = "rullst-access:";

/// Recursively scans Rust source files for parameterized routes without an
/// explicit public, owner, role, administrator or machine access
/// classification.
///
/// This is a bounded source heuristic. It deliberately reports a finding when
/// it cannot recognize the route boundary; a clean scan is not a proof that a
/// domain resource lookup enforces ownership correctly at runtime.
///
/// Top-level `#[cfg(test)]` items are skipped individually. Guard evidence for
/// an owner/role/admin classification, and the `MachineEndpoint`
/// registrations a machine classification needs, are collected per crate
/// (`src` tree) from code outside those items; comments and doc comments do
/// not count.
pub fn scan_idor_vulnerabilities(src_dir: &Path) -> (usize, Vec<String>) {
    let source_files = collect_rust_source_files(src_dir);
    let mut crate_code = HashMap::<std::path::PathBuf, Vec<String>>::new();
    let mut sources = Vec::new();
    for path in source_files.files {
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        let views = audit_source(&content);
        let source_root = source_root_for(&path).unwrap_or_else(|| src_dir.to_path_buf());
        crate_code
            .entry(source_root.clone())
            .or_default()
            .push(views.code);
        sources.push((path, source_root, views.production));
    }
    let crate_evidence = crate_code
        .into_iter()
        .map(|(root, code)| (root, CrateEvidence::from_code(&code)))
        .collect::<HashMap<_, _>>();

    let mut warnings = Vec::new();
    if let Some(reason) = source_files.incomplete {
        warnings.push(incomplete_walk_warning(
            src_dir,
            &reason,
            "parameterized routes",
        ));
    }
    let none = CrateEvidence::default();
    for (path, source_root, content) in sources {
        let evidence = crate_evidence.get(&source_root).unwrap_or(&none);
        warnings.extend(scan_idor_source_with_evidence(&path, &content, evidence));
    }
    (warnings.len(), warnings)
}

/// Production Rust sources for the route scan; symlinks are never followed.
pub(super) fn collect_rust_source_files(src_dir: &Path) -> RustSources {
    let require_src_component = src_dir.file_name().and_then(|name| name.to_str()) != Some("src");
    let mut sources = rust_sources(src_dir);
    sources.files.retain(|path| {
        path.file_name().and_then(|name| name.to_str()) != Some("tests.rs")
            && !path
                .components()
                .any(|component| component.as_os_str() == "tests")
            && (!require_src_component
                || path
                    .components()
                    .any(|component| component.as_os_str() == "src"))
    });
    sources
}

#[cfg(test)]
fn scan_idor_source(path: &Path, content: &str) -> Vec<String> {
    let views = audit_source(content);
    scan_idor_source_with_evidence(
        path,
        &views.production,
        &CrateEvidence::from_code(&[views.code]),
    )
}

/// Guards and machine registrations of one crate.
#[derive(Default)]
struct CrateEvidence {
    guards: GuardEvidence,
    machine: MachineRoutes,
}

impl CrateEvidence {
    fn from_code(code: &[String]) -> Self {
        let mut guards = GuardEvidence::default();
        let mut constants = Constants::default();
        for file in code {
            guards.include(file);
            constants.include(file);
        }
        let mut machine = MachineRoutes::default();
        for file in code {
            machine.include(file, &constants);
        }
        Self { guards, machine }
    }
}

#[derive(Clone, Copy, Default)]
struct GuardEvidence {
    owner: bool,
    role: bool,
    admin: bool,
}

impl GuardEvidence {
    fn include(&mut self, content: &str) {
        self.owner |= content.contains("authorize_owner_or_role");
        self.role |= self.owner
            || content.contains("RbacGuard::authorize(")
            || content.contains("RequireRoleLayer")
            || content.contains("protect_router(");
        self.admin |= content.contains("RequireRoleLayer") || content.contains("protect_router(");
    }
}

fn source_root_for(path: &Path) -> Option<std::path::PathBuf> {
    path.ancestors()
        .find(|ancestor| ancestor.file_name().and_then(|name| name.to_str()) == Some("src"))
        .map(Path::to_path_buf)
}

fn scan_idor_source_with_evidence(
    path: &Path,
    content: &str,
    evidence: &CrateEvidence,
) -> Vec<String> {
    let lines = content.lines().collect::<Vec<_>>();
    let mut findings = Vec::new();
    let mut route_call_continues = false;

    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        // A comment (such as the access marker) between `.route(` and its path
        // literal must not end the multi-line call.
        if route_call_continues && trimmed.starts_with("//") {
            continue;
        }
        let declares_route = contains_route_call(trimmed) || route_call_continues;
        route_call_continues = trimmed.ends_with(".route(") || trimmed == "route(";
        if !declares_route {
            continue;
        }

        let routes = quoted_parameterized_routes(trimmed);
        if routes.is_empty() {
            continue;
        }
        let marker_line = access_marker_line(&lines, index, trimmed);
        let classification = marker_line.and_then(access_classification);
        let methods = declared_methods(&lines, index, trimmed);
        for route in routes {
            let reason = if marker_line.is_some_and(|marker| !access_marker_has_reason(marker)) {
                Some("classification must include a concrete reason after its access level")
            } else {
                classification_problem(classification, trimmed, route, &methods, evidence)
            };
            if let Some(reason) = reason {
                let hint = if classification.is_none() && evidence.machine.declares(route) {
                    "; a MachineEndpoint registers this path, so classify it as machine"
                } else {
                    ""
                };
                findings.push(format!(
                    "File '{}:{}': parameterized route `{route}` {reason}{hint}",
                    super::slash_path(path),
                    index + 1
                ));
            }
        }
    }

    findings
}

/// The methods of the route declared on `line`, or on the next code line
/// when a multi-line `.route(` call puts them after the path literal.
fn declared_methods(lines: &[&str], index: usize, line: &str) -> Vec<&'static str> {
    let methods = route_methods(line);
    if !methods.is_empty() {
        return methods;
    }
    lines
        .iter()
        .skip(index + 1)
        .map(|next| next.trim())
        .find(|next| !next.starts_with("//"))
        .map(route_methods)
        .unwrap_or_default()
}

/// Why `classification` does not cover `route`, or `None` when it does.
fn classification_problem(
    classification: Option<&str>,
    line: &str,
    route: &str,
    methods: &[&str],
    evidence: &CrateEvidence,
) -> Option<&'static str> {
    let guards = evidence.guards;
    match classification {
        Some("public") if route_is_read_only(line) => None,
        Some("public") => Some(
            "public classification is accepted only for a recognized GET route; classify the mutation as owner, role, admin or machine",
        ),
        Some("machine") if evidence.machine.covers(methods, route) => None,
        Some("machine") => Some(
            "machine classification requires a MachineEndpoint registration (bearer, verified_bearer, signed_webhook, address_token or mutual_tls) with each of this route's write methods and this path, given as a string literal or a `const` &str, in this crate's non-test code",
        ),
        Some("owner") if guards.owner => None,
        Some("owner") => Some(
            "owner classification requires RbacGuard::authorize_owner_or_role in this crate's non-test code (comments do not count)",
        ),
        Some("role") if guards.role => None,
        Some("role") => Some(
            "role classification requires RbacGuard::authorize, RequireRoleLayer, or protect_router in this crate's non-test code (comments do not count)",
        ),
        Some("admin") if guards.admin => None,
        Some("admin") => Some(
            "admin classification requires RequireRoleLayer or NexusAuthPolicy::protect_router in this crate's non-test code (comments do not count)",
        ),
        Some(_) => Some("unknown rullst-access classification"),
        None => Some(
            "missing an adjacent `// rullst-access: public|owner|role|admin|machine — reason` classification",
        ),
    }
}

fn contains_route_call(line: &str) -> bool {
    [
        "get(", "post(", "put(", "patch(", "delete(", "ws(", ".route(",
    ]
    .iter()
    .any(|needle| line.contains(needle))
}

/// Every quoted literal on the line that is a parameterized path, so a
/// later route on a one-line `routes!` list is not hidden by an earlier one.
fn quoted_parameterized_routes(line: &str) -> Vec<&str> {
    quoted_literals(line)
        .filter(|candidate| {
            candidate.starts_with('/') && candidate.split('/').any(is_parameter_segment)
        })
        .collect()
}

/// The contents of each `"..."` on the line; `\"` does not end a literal.
fn quoted_literals(line: &str) -> impl Iterator<Item = &str> {
    let mut rest = line;
    std::iter::from_fn(move || {
        let start = rest.find('"')? + 1;
        let body = rest.get(start..)?;
        let mut escaped = false;
        let end = body.char_indices().find_map(|(offset, character)| {
            let closes = character == '"' && !escaped;
            escaped = character == '\\' && !escaped;
            closes.then_some(offset)
        })?;
        rest = body.get(end + 1..)?;
        body.get(..end)
    })
}

fn is_parameter_segment(segment: &str) -> bool {
    if segment.starts_with(':') {
        return segment.len() > 1;
    }
    let normalized = segment
        .strip_prefix("{{")
        .and_then(|value| value.strip_suffix("}}"))
        .or_else(|| {
            segment
                .strip_prefix('{')
                .and_then(|value| value.strip_suffix('}'))
        });
    normalized.is_some_and(|value| !value.is_empty() && !value.starts_with('*'))
}

fn access_marker_line<'a>(lines: &[&'a str], index: usize, current: &'a str) -> Option<&'a str> {
    if current.contains(ACCESS_MARKER) {
        return Some(current);
    }
    index
        .checked_sub(1)
        .and_then(|previous| lines.get(previous).copied())
        .filter(|line| line.contains(ACCESS_MARKER))
}

fn access_classification(marker: &str) -> Option<&str> {
    marker
        .split_once(ACCESS_MARKER)
        .and_then(|(_, suffix)| suffix.split_whitespace().next())
        .map(|classification| {
            classification.trim_matches(|character: char| !character.is_alphanumeric())
        })
}

fn access_marker_has_reason(marker: &str) -> bool {
    marker
        .split_once(ACCESS_MARKER)
        .map(|(_, suffix)| {
            suffix
                .split_whitespace()
                .skip(1)
                .any(|word| word.chars().any(char::is_alphanumeric))
        })
        .unwrap_or(false)
}

fn route_is_read_only(line: &str) -> bool {
    line.contains("get(")
        && !["post(", "put(", "patch(", "delete("]
            .iter()
            .any(|method| line.contains(method))
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
#[path = "audit_idor_tests.rs"]
mod scan_tests;
