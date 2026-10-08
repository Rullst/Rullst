use colored::Colorize;
use std::fs;
use std::path::{Path, PathBuf};

use crate::generators::audit_compliance::{
    ComplianceEvidence, EvidenceStatus, write_compliance_report,
};
use crate::generators::audit_evidence::inspect_local_network_surface;
pub use crate::generators::audit_evidence::{generate_cyclonedx_sbom, scan_local_network_surface};
use crate::generators::audit_idor::collect_rust_source_files;
pub use crate::generators::audit_idor::scan_idor_vulnerabilities;
#[cfg(test)]
use crate::generators::audit_scope::cargo_audit_arguments;
pub use crate::generators::audit_scope::scan_unsafe_code;
use crate::generators::audit_scope::{
    cargo_audit_status, package_source_roots, scan_each, validate_audit_ignores,
};

/// A scan whose source walk hit a bound is reported as a finding, not as clean.
pub(super) fn incomplete_walk_warning(root: &Path, reason: &str, subject: &str) -> String {
    format!(
        "Source walk under '{}' is incomplete ({reason}); {subject} beyond it were not scanned",
        root.display()
    )
}

pub fn run_security_audit(
    ai_mode: bool,
    compliance_mode: bool,
    idor_mode: bool,
    geiger_mode: bool,
    sbom_mode: bool,
    network_mode: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    run_security_audit_with_exceptions(
        ai_mode,
        compliance_mode,
        idor_mode,
        geiger_mode,
        sbom_mode,
        &[],
        network_mode,
    )
}

pub fn run_security_audit_with_exceptions(
    ai_mode: bool,
    compliance_mode: bool,
    idor_mode: bool,
    geiger_mode: bool,
    sbom_mode: bool,
    audit_ignores: &[String],
    network_mode: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    run_audit(AuditOptions {
        ai: ai_mode,
        compliance: compliance_mode,
        idor: idor_mode,
        geiger: geiger_mode,
        sbom: sbom_mode,
        ignores: audit_ignores,
        network: network_mode,
        json: false,
    })
}

/// The `audit` flags; `json` moves the human progress to stderr and prints
/// one `rullst.cli-audit.v1` summary on stdout.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct AuditOptions<'a> {
    pub ai: bool,
    pub compliance: bool,
    pub idor: bool,
    pub geiger: bool,
    pub sbom: bool,
    pub ignores: &'a [String],
    pub network: bool,
    pub json: bool,
}

/// Human progress: stdout, or stderr when stdout carries the JSON summary.
macro_rules! say {
    ($json:expr, $($arg:tt)*) => {
        if $json {
            eprintln!($($arg)*)
        } else {
            println!($($arg)*)
        }
    };
}

pub(crate) fn run_audit(options: AuditOptions<'_>) -> Result<(), Box<dyn std::error::Error>> {
    let AuditOptions {
        ai: ai_mode,
        compliance: compliance_mode,
        idor: idor_mode,
        geiger: geiger_mode,
        sbom: sbom_mode,
        ignores: audit_ignores,
        network: network_mode,
        json,
    } = options;
    validate_audit_ignores(audit_ignores)?;
    let title = if ai_mode {
        "🛡️ Running Rullst Security Audit with deterministic recommendations..."
    } else {
        "🛡️ Running Rullst Security Audit..."
    };
    say!(json, "{}", title.bright_cyan().bold());

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
                            say!(
                                json,
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
                say!(
                    json,
                    "  {} Could not read .env for the bounded secret scan: {}",
                    "[ERROR]".red().bold(),
                    error
                );
                secret_scan_error = Some(error.to_string());
                issues_found += 1;
            }
        }
    } else {
        say!(json, "  {} No .env file found in root.", "[INFO]".blue());
    }

    // 2. Check for Cargo audit vulnerabilities
    say!(
        json,
        "  {} Checking dependency vulnerabilities...",
        "[AUDIT]".magenta()
    );
    let dependency_audit =
        cargo_audit_status(std::ffi::OsStr::new("cargo"), Path::new("."), audit_ignores);
    match &dependency_audit {
        EvidenceStatus::NoFindings => say!(
            json,
            "  {} No advisories reported by cargo-audit.",
            "[OK]".green()
        ),
        EvidenceStatus::NoFindingsOutsideExceptions(exceptions) => say!(
            json,
            "  {} No findings outside the governed exceptions; exceptions remain unresolved: {}.",
            "[OK]".green(),
            exceptions.join(", ")
        ),
        EvidenceStatus::NotChecked(_) => say!(
            json,
            "  {} cargo-audit not installed. Run 'cargo install cargo-audit' for deep dependency scanning.",
            "[NOTE]".yellow()
        ),
        _ => {
            say!(
                json,
                "  {} cargo-audit did not complete successfully; inspect its output directly.",
                "[ERROR]".red().bold()
            );
            issues_found += 1;
        }
    }

    // 3. Memory Safety & Unsafe Code (Cargo Geiger)
    say!(
        json,
        "  {} Auditing memory safety & unsafe code blocks (Cargo Geiger)...",
        "[GEIGER]".bright_cyan()
    );
    // `src` and the `src` of workspace members below the current directory.
    let package_roots = package_source_roots(Path::new("."));
    let unsafe_source_available = package_roots.is_some();
    let (unsafe_count, unsafe_warnings) = scan_each(
        package_roots.as_deref().unwrap_or_default(),
        scan_unsafe_code,
    );
    if !unsafe_source_available {
        say!(
            json,
            "  {} No project src directory was available; the bounded unsafe scan was not executed.",
            "[INFO]".blue()
        );
    } else if unsafe_count == 0 {
        say!(
            json,
            "  {} The bounded project-source heuristic found no unsafe syntax.",
            "[OK]".green()
        );
    } else {
        for warn in &unsafe_warnings {
            say!(json, "  {} {}", "[UNSAFE WARNING]".red().bold(), warn);
        }
        issues_found += unsafe_count;
    }

    if geiger_mode {
        let geiger_status = std::process::Command::new("cargo")
            .arg("geiger")
            .arg("--version")
            .output();

        if geiger_status.is_ok_and(|output| output.status.success()) {
            say!(
                json,
                "  {} Running full dependency tree unsafe analysis (cargo geiger)...",
                "[GEIGER]".cyan()
            );
            let mut geiger = std::process::Command::new("cargo");
            geiger.arg("geiger");
            if json {
                geiger.stdout(std::process::Stdio::from(std::io::stderr()));
            }
            match geiger.status() {
                Ok(status) if status.success() => {}
                Ok(status) => {
                    say!(
                        json,
                        "  {} cargo-geiger reported findings or failed with status {}.",
                        "[ERROR]".red().bold(),
                        status
                    );
                    issues_found += 1;
                }
                Err(error) => {
                    say!(
                        json,
                        "  {} cargo-geiger could not run: {}",
                        "[ERROR]".red().bold(),
                        error
                    );
                    issues_found += 1;
                }
            }
        } else {
            say!(
                json,
                "  {} cargo-geiger is unavailable although --geiger was requested. Run 'cargo install cargo-geiger'.",
                "[ERROR]".red().bold()
            );
            issues_found += 1;
        }
    }

    // 4. IDOR / BOLA Route Scanner
    let idor_roots = package_roots.unwrap_or_else(|| vec![PathBuf::from(".")]);
    let idor_source_available = idor_roots
        .iter()
        .any(|root| !collect_rust_source_files(root).files.is_empty());
    let (idor_count, idor_warnings) = scan_each(&idor_roots, scan_idor_vulnerabilities);
    if idor_mode || idor_count > 0 {
        say!(
            json,
            "  {} Checking IDOR / BOLA authorization on parameterized routes...",
            "[IDOR]".bright_yellow()
        );
        if !idor_source_available {
            say!(
                json,
                "  {} No scannable Rust source was available; the IDOR/BOLA check is incomplete.",
                "[ERROR]".red().bold()
            );
            issues_found += 1;
        } else if idor_count == 0 {
            say!(
                json,
                "  {} The bounded route heuristic found no missing access classifications or recognized guards.",
                "[OK]".green()
            );
        } else {
            for warn in &idor_warnings {
                say!(json, "  {} {}", "[IDOR WARNING]".yellow().bold(), warn);
            }
            issues_found += idor_count;
        }
    }

    // 5. SBOM Generation
    let mut sbom_evidence = EvidenceStatus::NotChecked("SBOM generation was not requested");
    if sbom_mode {
        say!(
            json,
            "  {} Generating CycloneDX 1.5 Software Bill of Materials (SBOM)...",
            "[SBOM]".bright_blue()
        );
        match generate_cyclonedx_sbom(Path::new("Cargo.lock")) {
            Ok((count, file_name)) => {
                say!(
                    json,
                    "  {} Generated CycloneDX SBOM with {} components at '{}'",
                    "[SUCCESS]".green().bold(),
                    count,
                    file_name
                );
                sbom_evidence = EvidenceStatus::Generated(count);
            }
            Err(e) => {
                say!(
                    json,
                    "  {} Failed to generate SBOM: {}",
                    "[ERROR]".red().bold(),
                    e
                );
                sbom_evidence = EvidenceStatus::Error(e.to_string());
                issues_found += 1;
            }
        }
    }

    // 6. Network Surface Scanner (RustScan-inspired)
    let mut network_evidence = EvidenceStatus::NotChecked("network surface scan was not requested");
    if network_mode {
        say!(
            json,
            "  {} Scanning local network surface & interface bindings (RustScan mode)...",
            "[NETWORK]".bright_magenta()
        );
        let surface = inspect_local_network_surface();
        if surface.observations.is_empty() && surface.incomplete.is_empty() {
            say!(
                json,
                "  {} No open local listening ports detected.",
                "[OK]".green()
            );
        }
        for report in &surface.observations {
            if report.contains("should be '127.0.0.1'") {
                say!(json, "  {} {}", "[NETWORK WARNING]".yellow().bold(), report);
            } else {
                say!(json, "  {} {}", "[ACTIVE SERVICE]".bright_cyan(), report);
            }
        }
        for reason in &surface.incomplete {
            say!(
                json,
                "  {} Network check incomplete: {}",
                "[ERROR]".red().bold(),
                reason
            );
        }
        issues_found += surface.findings + surface.incomplete.len();
        network_evidence = if surface.incomplete.is_empty() {
            EvidenceStatus::Observed(surface.observations.len())
        } else {
            EvidenceStatus::Error(surface.incomplete.join("; "))
        };
    }

    if ai_mode {
        say!(
            json,
            "\n🤖 {}",
            "Deterministic Security Recommendations:"
                .bright_purple()
                .bold()
        );
        if issues_found == 0 {
            say!(
                json,
                "  ✅ Completed bounded checks reported no findings; skipped or unavailable checks remain outside this result."
            );
        } else {
            say!(
                json,
                "  ⚠️ Found {} potential security items. Recommendation: Eliminate unsafe blocks, enforce RbacGuard on parameterized routes, rotate secrets, and run cargo update.",
                issues_found
            );
        }
    }

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
        unsafe_scan: if !unsafe_source_available {
            EvidenceStatus::NotChecked("no src directory was available to inspect")
        } else if unsafe_count == 0 {
            EvidenceStatus::NoFindings
        } else {
            EvidenceStatus::Findings(unsafe_count)
        },
        idor_scan: if !idor_source_available {
            EvidenceStatus::NotChecked("no scannable Rust source was available")
        } else if idor_count == 0 {
            EvidenceStatus::NoFindings
        } else {
            EvidenceStatus::Findings(idor_count)
        },
        sbom: sbom_evidence,
        network_scan: network_evidence,
    };
    if compliance_mode {
        say!(
            json,
            "\n📊 {}",
            "Generating evidence-based SECURITY_COMPLIANCE.md report..."
                .bright_green()
                .bold()
        );
        write_compliance_report(Path::new("SECURITY_COMPLIANCE.md"), &evidence)?;
        say!(
            json,
            "  {} Evidence report written to SECURITY_COMPLIANCE.md",
            "[SUCCESS]".green().bold()
        );
    }

    if json {
        let summary = crate::generators::audit_compliance::summary(&evidence, issues_found);
        println!("{}", serde_json::to_string_pretty(&summary)?);
    }

    say!(json, "\nAudit finished. Issues found: {}", issues_found);

    if idor_mode && !idor_source_available {
        return Err(std::io::Error::other(
            "IDOR/BOLA audit could not find any scannable Rust source",
        )
        .into());
    }

    if idor_mode && idor_count > 0 {
        return Err(std::io::Error::other(format!(
            "IDOR/BOLA audit found {idor_count} unclassified or unguarded parameterized route(s)"
        ))
        .into());
    }

    if issues_found > 0 {
        return Err(std::io::Error::other(format!(
            "security audit reported {issues_found} finding(s) or incomplete requested check(s)"
        ))
        .into());
    }

    Ok(())
}

#[cfg(test)]
#[path = "audit_tests.rs"]
mod tests;
