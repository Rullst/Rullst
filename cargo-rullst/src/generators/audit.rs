use colored::Colorize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::generators::audit_compliance::{
    ComplianceEvidence, EvidenceStatus, write_compliance_report,
};
pub use crate::generators::audit_evidence::{generate_cyclonedx_sbom, scan_local_network_surface};

const ACCESS_MARKER: &str = "rullst-access:";

/// Recursively scans Rust source files for parameterized routes without an
/// explicit public, owner, role, or administrator access classification.
///
/// This is a bounded source heuristic. It deliberately reports a finding when
/// it cannot recognize the route boundary; a clean scan is not a proof that a
/// domain resource lookup enforces ownership correctly at runtime.
pub fn scan_idor_vulnerabilities(src_dir: &Path) -> (usize, Vec<String>) {
    let require_src_component = src_dir.file_name().and_then(|name| name.to_str()) != Some("src");
    let mut source_files = Vec::new();

    fn visit_dirs(
        dir: &Path,
        require_src_component: bool,
        source_files: &mut Vec<std::path::PathBuf>,
    ) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let directory_name = path.file_name().and_then(|name| name.to_str());
                    if matches!(
                        directory_name,
                        Some("target" | ".git" | ".agents" | ".codex")
                    ) {
                        continue;
                    }
                    visit_dirs(&path, require_src_component, source_files);
                } else if path.extension().and_then(|s| s.to_str()) == Some("rs")
                    && path.file_name().and_then(|name| name.to_str()) != Some("tests.rs")
                    && !path
                        .components()
                        .any(|component| component.as_os_str() == "tests")
                    && (!require_src_component
                        || path
                            .components()
                            .any(|component| component.as_os_str() == "src"))
                {
                    source_files.push(path);
                }
            }
        }
    }

    if src_dir.exists() {
        visit_dirs(src_dir, require_src_component, &mut source_files);
    }

    let mut crate_evidence = HashMap::<std::path::PathBuf, GuardEvidence>::new();
    let mut sources = Vec::new();
    for path in source_files {
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        let production = production_source(&content);
        let source_root = source_root_for(&path).unwrap_or_else(|| src_dir.to_path_buf());
        crate_evidence
            .entry(source_root.clone())
            .or_default()
            .include(production);
        sources.push((path, source_root, production.to_string()));
    }

    let mut warnings = Vec::new();
    for (path, source_root, content) in sources {
        let evidence = crate_evidence
            .get(&source_root)
            .copied()
            .unwrap_or_default();
        warnings.extend(scan_idor_source_with_evidence(&path, &content, evidence));
    }
    (warnings.len(), warnings)
}

#[cfg(test)]
fn scan_idor_source(path: &Path, content: &str) -> Vec<String> {
    let production = production_source(content);
    scan_idor_source_with_evidence(path, production, GuardEvidence::from_content(production))
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

fn production_source(content: &str) -> &str {
    content
        .split_once("\n#[cfg(test)]")
        .map_or(content, |(production, _)| production)
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
        let reason = match classification {
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

fn route_is_read_only(line: &str) -> bool {
    line.contains("get(")
        && !["post(", "put(", "patch(", "delete("]
            .iter()
            .any(|method| line.contains(method))
}

/// Recursively scans Rust source files for `unsafe` blocks, functions, or implementations.
pub fn scan_unsafe_code(src_dir: &Path) -> (usize, Vec<String>) {
    let mut warnings = Vec::new();
    let mut count = 0;

    fn visit_dirs(dir: &Path, warnings: &mut Vec<String>, count: &mut usize) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    visit_dirs(&path, warnings, count);
                } else if path.extension().and_then(|s| s.to_str()) == Some("rs")
                    && let Ok(content) = fs::read_to_string(&path)
                {
                    for (line_idx, line) in content.lines().enumerate() {
                        let trimmed = line.trim();
                        if trimmed.starts_with("//")
                            || trimmed.starts_with("/*")
                            || trimmed.starts_with('*')
                        {
                            continue;
                        }
                        if trimmed.contains("unsafe {")
                            || trimmed.contains("unsafe fn")
                            || trimmed.contains("unsafe impl")
                            || trimmed.starts_with("unsafe ")
                        {
                            let msg = format!(
                                "File '{}:{}': Unsafe Rust detected: `{}`",
                                path.display(),
                                line_idx + 1,
                                trimmed
                            );
                            if !warnings.contains(&msg) {
                                warnings.push(msg);
                                *count += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    if src_dir.exists() {
        visit_dirs(src_dir, &mut warnings, &mut count);
    }

    (count, warnings)
}

pub fn run_security_audit(
    ai_mode: bool,
    compliance_mode: bool,
    idor_mode: bool,
    geiger_mode: bool,
    sbom_mode: bool,
    network_mode: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}",
        "🛡️ Running Rullst AI Security Audit..."
            .bright_cyan()
            .bold()
    );

    let mut issues_found = 0;
    let mut weak_secret_findings = 0usize;
    let mut secret_scan_completed = false;
    let mut secret_scan_error = None;

    // 1. Audit .env for plain-text secret leaks
    let env_path = Path::new(".env");
    if env_path.exists() {
        match fs::read_to_string(env_path) {
            Ok(content) => {
                secret_scan_completed = true;
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with('#') || trimmed.is_empty() {
                        continue;
                    }
                    if let Some((key, val)) = trimmed.split_once('=') {
                        let k = key.trim();
                        let v = val.trim();
                        if (k.contains("SECRET") || k.contains("KEY") || k.contains("PASSWORD"))
                            && !v.is_empty()
                            && v != "\"\""
                            && v != "''"
                            && v.len() < 16
                        {
                            println!(
                                "  {} Weak or short secret detected for key '{}' in .env",
                                "[WARNING]".yellow().bold(),
                                k
                            );
                            issues_found += 1;
                            weak_secret_findings = weak_secret_findings.saturating_add(1);
                        }
                    }
                }
            }
            Err(error) => {
                println!(
                    "  {} Could not read .env for the bounded secret scan: {}",
                    "[ERROR]".red().bold(),
                    error
                );
                secret_scan_error = Some(error.to_string());
            }
        }
    } else {
        println!("  {} No .env file found in root.", "[INFO]".blue());
    }

    // 2. Check for Cargo audit vulnerabilities
    println!(
        "  {} Checking dependency vulnerabilities...",
        "[AUDIT]".magenta()
    );
    let audit_tool = std::process::Command::new("cargo")
        .arg("audit")
        .arg("--version")
        .output();
    let dependency_audit = match audit_tool {
        Ok(tool) if tool.status.success() => {
            match std::process::Command::new("cargo").arg("audit").output() {
                Ok(out) if out.status.success() => {
                    println!(
                        "  {} No advisories reported by cargo-audit.",
                        "[OK]".green()
                    );
                    EvidenceStatus::NoFindings
                }
                Ok(out) => {
                    println!(
                        "  {} cargo-audit did not complete successfully; inspect its output directly.",
                        "[ERROR]".red().bold()
                    );
                    issues_found += 1;
                    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
                    EvidenceStatus::Error(if stderr.is_empty() {
                        format!("cargo-audit exited with status {}", out.status)
                    } else {
                        stderr
                    })
                }
                Err(error) => EvidenceStatus::Error(error.to_string()),
            }
        }
        Ok(_) | Err(_) => {
            println!(
                "  {} cargo-audit not installed. Run 'cargo install cargo-audit' for deep dependency scanning.",
                "[NOTE]".yellow()
            );
            EvidenceStatus::NotChecked("cargo-audit is unavailable")
        }
    };

    // 3. Memory Safety & Unsafe Code (Cargo Geiger)
    println!(
        "  {} Auditing memory safety & unsafe code blocks (Cargo Geiger)...",
        "[GEIGER]".bright_cyan()
    );
    let (unsafe_count, unsafe_warnings) = scan_unsafe_code(Path::new("src"));
    if unsafe_count == 0 {
        println!(
            "  {} The bounded project-source heuristic found no unsafe syntax.",
            "[OK]".green()
        );
    } else {
        for warn in &unsafe_warnings {
            println!("  {} {}", "[UNSAFE WARNING]".red().bold(), warn);
        }
        issues_found += unsafe_count;
    }

    if geiger_mode {
        let geiger_status = std::process::Command::new("cargo")
            .arg("geiger")
            .arg("--version")
            .output();

        if geiger_status.is_ok() {
            println!(
                "  {} Running full dependency tree unsafe analysis (cargo geiger)...",
                "[GEIGER]".cyan()
            );
            let _ = std::process::Command::new("cargo").arg("geiger").status();
        } else {
            println!(
                "  {} cargo-geiger not installed. Run 'cargo install cargo-geiger' for dependency tree unsafe scanning.",
                "[NOTE]".yellow()
            );
        }
    }

    // 4. IDOR / BOLA Route Scanner
    let idor_root = if Path::new("src").is_dir() {
        Path::new("src")
    } else {
        Path::new(".")
    };
    let (idor_count, idor_warnings) = scan_idor_vulnerabilities(idor_root);
    if idor_mode || idor_count > 0 {
        println!(
            "  {} Checking IDOR / BOLA authorization on parameterized routes...",
            "[IDOR]".bright_yellow()
        );
        if idor_count == 0 {
            println!(
                "  {} The bounded route heuristic found no missing ownership guards.",
                "[OK]".green()
            );
        } else {
            for warn in &idor_warnings {
                println!("  {} {}", "[IDOR WARNING]".yellow().bold(), warn);
            }
            issues_found += idor_count;
        }
    }

    // 5. SBOM Generation
    let mut sbom_evidence = EvidenceStatus::NotChecked("SBOM generation was not requested");
    if sbom_mode {
        println!(
            "  {} Generating CycloneDX 1.5 Software Bill of Materials (SBOM)...",
            "[SBOM]".bright_blue()
        );
        match generate_cyclonedx_sbom(Path::new("Cargo.lock")) {
            Ok((count, file_name)) => {
                println!(
                    "  {} Generated CycloneDX SBOM with {} components at '{}'",
                    "[SUCCESS]".green().bold(),
                    count,
                    file_name
                );
                sbom_evidence = EvidenceStatus::Generated(count);
            }
            Err(e) => {
                println!(
                    "  {} Failed to generate SBOM: {}",
                    "[ERROR]".red().bold(),
                    e
                );
                sbom_evidence = EvidenceStatus::Error(e.to_string());
            }
        }
    }

    // 6. Network Surface Scanner (RustScan-inspired)
    let mut network_evidence = EvidenceStatus::NotChecked("network surface scan was not requested");
    if network_mode {
        println!(
            "  {} Scanning local network surface & interface bindings (RustScan mode)...",
            "[NETWORK]".bright_magenta()
        );
        let (net_issues, net_reports) = scan_local_network_surface();
        if net_reports.is_empty() {
            println!(
                "  {} No open local listening ports detected.",
                "[OK]".green()
            );
        } else {
            for report in &net_reports {
                if report.contains("should be '127.0.0.1'") {
                    println!("  {} {}", "[NETWORK WARNING]".yellow().bold(), report);
                } else {
                    println!("  {} {}", "[ACTIVE SERVICE]".bright_cyan(), report);
                }
            }
        }
        issues_found += net_issues;
        network_evidence = EvidenceStatus::Observed(net_reports.len());
    }

    if ai_mode {
        println!(
            "\n🤖 {}",
            "AI Security Sentinel Analysis:".bright_purple().bold()
        );
        if issues_found == 0 {
            println!(
                "  ✅ Completed bounded checks reported no findings; skipped or unavailable checks remain outside this result."
            );
        } else {
            println!(
                "  ⚠️ Found {} potential security items. Recommendation: Eliminate unsafe blocks, enforce RbacGuard on parameterized routes, rotate secrets, and run cargo update.",
                issues_found
            );
        }
    }

    if compliance_mode {
        println!(
            "\n📊 {}",
            "Generating evidence-based SECURITY_COMPLIANCE.md report..."
                .bright_green()
                .bold()
        );
        let evidence = ComplianceEvidence {
            secret_scan: if let Some(error) = secret_scan_error {
                EvidenceStatus::Error(error)
            } else if !secret_scan_completed {
                EvidenceStatus::NotChecked("no .env file was available to inspect")
            } else if weak_secret_findings == 0 {
                EvidenceStatus::NoFindings
            } else {
                EvidenceStatus::Findings(weak_secret_findings)
            },
            dependency_audit,
            unsafe_scan: if !Path::new("src").is_dir() {
                EvidenceStatus::NotChecked("no src directory was available to inspect")
            } else if unsafe_count == 0 {
                EvidenceStatus::NoFindings
            } else {
                EvidenceStatus::Findings(unsafe_count)
            },
            idor_scan: if idor_count == 0 {
                EvidenceStatus::NoFindings
            } else {
                EvidenceStatus::Findings(idor_count)
            },
            sbom: sbom_evidence,
            network_scan: network_evidence,
        };
        write_compliance_report(Path::new("SECURITY_COMPLIANCE.md"), &evidence)?;
        println!(
            "  {} Evidence report written to SECURITY_COMPLIANCE.md",
            "[SUCCESS]".green().bold()
        );
    }

    println!("\nAudit finished. Issues found: {}", issues_found);

    if idor_mode && idor_count > 0 {
        return Err(std::io::Error::other(format!(
            "IDOR/BOLA audit found {idor_count} unclassified or unguarded parameterized route(s)"
        ))
        .into());
    }

    Ok(())
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
