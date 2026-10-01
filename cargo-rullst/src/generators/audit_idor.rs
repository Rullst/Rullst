//! Bounded source heuristic for IDOR/BOLA access classification of
//! parameterized routes (`cargo rullst audit --idor`).

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::generators::audit::incomplete_walk_warning;
use crate::generators::audit_source::production_source;
use crate::generators::source_walk::{RustSources, rust_sources};

const ACCESS_MARKER: &str = "rullst-access:";

/// Recursively scans Rust source files for parameterized routes without an
/// explicit public, owner, role, or administrator access classification.
///
/// This is a bounded source heuristic. It deliberately reports a finding when
/// it cannot recognize the route boundary; a clean scan is not a proof that a
/// domain resource lookup enforces ownership correctly at runtime. Top-level
/// `#[cfg(test)]` items are skipped individually, not the rest of the file.
pub fn scan_idor_vulnerabilities(src_dir: &Path) -> (usize, Vec<String>) {
    let source_files = collect_rust_source_files(src_dir);
    let mut crate_evidence = HashMap::<std::path::PathBuf, GuardEvidence>::new();
    let mut sources = Vec::new();
    for path in source_files.files {
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        let production = production_source(&content);
        let source_root = source_root_for(&path).unwrap_or_else(|| src_dir.to_path_buf());
        crate_evidence
            .entry(source_root.clone())
            .or_default()
            .include(&production);
        sources.push((path, source_root, production));
    }

    let mut warnings = Vec::new();
    if let Some(reason) = source_files.incomplete {
        warnings.push(incomplete_walk_warning(
            src_dir,
            &reason,
            "parameterized routes",
        ));
    }
    for (path, source_root, content) in sources {
        let evidence = crate_evidence
            .get(&source_root)
            .copied()
            .unwrap_or_default();
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
    let production = production_source(content);
    scan_idor_source_with_evidence(path, &production, GuardEvidence::from_content(&production))
}

#[derive(Clone, Copy, Default)]
struct GuardEvidence {
    owner: bool,
    role: bool,
    admin: bool,
}

impl GuardEvidence {
    #[cfg(test)]
    fn from_content(content: &str) -> Self {
        let mut evidence = Self::default();
        evidence.include(content);
        evidence
    }

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
    evidence: GuardEvidence,
) -> Vec<String> {
    let lines = content.lines().collect::<Vec<_>>();
    let mut findings = Vec::new();
    let mut route_call_continues = false;

    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        let declares_route = contains_route_call(trimmed) || route_call_continues;
        route_call_continues = trimmed.ends_with(".route(") || trimmed == "route(";
        if !declares_route {
            continue;
        }

        let Some(route) = quoted_parameterized_route(trimmed) else {
            continue;
        };
        let marker_line = access_marker_line(&lines, index, trimmed);
        let classification = marker_line.and_then(access_classification);
        let reason = if marker_line.is_some_and(|marker| !access_marker_has_reason(marker)) {
            Some("classification must include a concrete reason after its access level")
        } else {
            match classification {
                Some("public") if route_is_read_only(trimmed) => None,
                Some("public") => Some(
                    "public classification is accepted only for a recognized GET route; classify the mutation as owner, role, or admin",
                ),
                Some("owner") if evidence.owner => None,
                Some("owner") => Some(
                    "owner classification requires RbacGuard::authorize_owner_or_role in this source file",
                ),
                Some("role") if evidence.role => None,
                Some("role") => Some(
                    "role classification requires RbacGuard::authorize, RequireRoleLayer, or protect_router in this source file",
                ),
                Some("admin") if evidence.admin => None,
                Some("admin") => Some(
                    "admin classification requires RequireRoleLayer or NexusAuthPolicy::protect_router in this source file",
                ),
                Some(_) => Some("unknown rullst-access classification"),
                None => Some(
                    "missing an adjacent `// rullst-access: public|owner|role|admin — reason` classification",
                ),
            }
        };

        if let Some(reason) = reason {
            findings.push(format!(
                "File '{}:{}': parameterized route `{route}` {reason}",
                path.display(),
                index + 1
            ));
        }
    }

    findings
}

fn contains_route_call(line: &str) -> bool {
    [
        "get(", "post(", "put(", "patch(", "delete(", "ws(", ".route(",
    ]
    .iter()
    .any(|needle| line.contains(needle))
}

fn quoted_parameterized_route(line: &str) -> Option<&str> {
    let quote_start = line.find('"')?;
    let tail = &line[quote_start + 1..];
    let quote_end = tail.find('"')?;
    let candidate = &tail[..quote_end];
    if candidate.starts_with('/') && candidate.split('/').any(is_parameter_segment) {
        Some(candidate)
    } else {
        None
    }
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
mod tests {
    use super::*;
    fn findings(source: &str) -> Vec<String> {
        scan_idor_source(Path::new("src/routes.rs"), source)
    }

    #[test]
    fn public_read_routes_require_an_explicit_adjacent_classification() {
        let classified = r#"
// rullst-access: public — published articles are intentionally public.
get("/posts/{slug}" => show),
"#;
        assert!(findings(classified).is_empty());

        let unclassified = r#"get("/posts/{slug}" => show),"#;
        let result = findings(unclassified);
        assert_eq!(result.len(), 1);
        assert!(result[0].contains("missing an adjacent"));
        assert!(result[0].contains("/posts/{slug}"));
    }

    #[test]
    fn public_classification_never_suppresses_a_mutating_route() {
        let source = r#"
// rullst-access: public — this annotation is unsafe for a mutation.
delete("/documents/{id}" => destroy),
"#;
        let result = findings(source);
        assert_eq!(result.len(), 1);
        assert!(result[0].contains("accepted only for a recognized GET"));
    }

    #[test]
    fn classifications_require_a_concrete_adjacent_reason() {
        let source = r#"
// rullst-access: public
get("/posts/{slug}" => show),
"#;
        let result = findings(source);
        assert_eq!(result.len(), 1);
        assert!(result[0].contains("must include a concrete reason"));
    }

    #[test]
    fn owner_routes_require_the_owner_or_role_guard() {
        let guarded = r#"
// rullst-access: owner — the handler compares the authenticated subject.
get("/accounts/:account_id" => show_account),

fn authorize(ctx: &UserContext, owner: &str) {
    let _ = RbacGuard::authorize_owner_or_role(ctx, owner, "admin");
}
"#;
        assert!(findings(guarded).is_empty());

        let missing_guard = r#"
// rullst-access: owner — no ownership check actually exists.
get("/accounts/:account_id" => show_account),
"#;
        assert_eq!(findings(missing_guard).len(), 1);
    }

    #[test]
    fn admin_routes_require_a_recognized_server_boundary() {
        let protected = r#"
// rullst-access: admin — protected as a group below.
post("/products/{{id}}/add-stock" => add_stock),
let protected = admin_access.protect_router(admin_routes.into_axum())?;
"#;
        assert!(findings(protected).is_empty());

        let annotation_only = r#"
// rullst-access: admin — a comment alone must not suppress the finding.
post("/products/{id}/add-stock" => add_stock),
"#;
        assert_eq!(findings(annotation_only).len(), 1);
    }

    #[test]
    fn unrelated_user_context_no_longer_hides_an_unclassified_route() {
        let source = r#"
get("/orders/{order_id}" => show_order),
fn unrelated(_context: UserContext) {}
"#;
        assert_eq!(findings(source).len(), 1);
    }

    #[test]
    fn routes_after_an_early_test_declaration_are_still_scanned() {
        // The old scan truncated the file at the first top-level
        // `#[cfg(test)]` and reported this unclassified route as clean.
        let source = r#"use rullst::prelude::*;
#[cfg(test)]
mod tests;

fn routes() -> Router {
    routes! {
        get("/accounts/{id}" => show_account),
    }
}

#[cfg(test)]
mod inline {
    fn fixture() { get("/fixtures/{id}" => show) }
}
"#;
        let result = findings(source);
        assert_eq!(result.len(), 1, "{result:?}");
        assert!(result[0].contains("routes.rs:7"));
        assert!(result[0].contains("/accounts/{id}"));
    }

    #[test]
    fn guard_evidence_is_crate_wide_and_ignores_test_items() {
        let project = tempfile::tempdir().expect("temporary project");
        let src = project.path().join("src");
        fs::create_dir_all(&src).expect("source directory");
        fs::write(
            src.join("routes.rs"),
            "// rullst-access: owner — checked by the guard module.\nget(\"/orders/{id}\" => show),\n",
        )
        .expect("route fixture");
        fs::write(
            src.join("guard.rs"),
            "#[cfg(test)]\nmod tests { fn t() { RbacGuard::authorize_owner_or_role(c, o, \"admin\"); } }\n",
        )
        .expect("test-only guard fixture");
        let (count, warnings) = scan_idor_vulnerabilities(&src);
        assert_eq!(count, 1, "{warnings:?}");

        fs::write(
            src.join("guard.rs"),
            "fn guard() { RbacGuard::authorize_owner_or_role(c, o, \"admin\"); }\n",
        )
        .expect("guard fixture");
        let (count, warnings) = scan_idor_vulnerabilities(&src);
        assert_eq!(count, 0, "{warnings:?}");
    }

    #[test]
    fn multiline_axum_routes_and_generic_parameter_names_are_detected() {
        let source = r#"
Router::new().route(
    "/invoices/{invoice_number}",
    delete(remove_invoice),
)
"#;
        let result = findings(source);
        assert_eq!(result.len(), 1);
        assert!(result[0].contains("/invoices/{invoice_number}"));
    }
}
