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
use crate::generators::audit_scope::{package_source_roots, scan_each};
use crate::generators::source_walk::rust_sources;

/// A scan whose source walk hit a bound is reported as a finding, not as clean.
pub(super) fn incomplete_walk_warning(root: &Path, reason: &str, subject: &str) -> String {
    format!(
        "Source walk under '{}' is incomplete ({reason}); {subject} beyond it were not scanned",
        root.display()
    )
}

/// Recursively scans Rust source files for `unsafe` blocks, functions, or implementations.
///
/// Symlinked files and directories are not followed; a walk that reaches its
/// bound adds a finding instead of reporting a clean scan.
pub fn scan_unsafe_code(src_dir: &Path) -> (usize, Vec<String>) {
    let sources = rust_sources(src_dir);
    let mut warnings = Vec::new();
    if let Some(reason) = sources.incomplete {
        warnings.push(incomplete_walk_warning(src_dir, &reason, "source files"));
    }
    for path in sources.files {
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        for (line_idx, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
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
                }
            }
        }
    }

    (warnings.len(), warnings)
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
    validate_audit_ignores(audit_ignores)?;
    let title = if ai_mode {
        "🛡️ Running Rullst Security Audit with deterministic recommendations..."
    } else {
        "🛡️ Running Rullst Security Audit..."
    };
    println!("{}", title.bright_cyan().bold());

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
                issues_found += 1;
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
            let mut command = std::process::Command::new("cargo");
            command.args(cargo_audit_arguments(audit_ignores));
            match command.output() {
                Ok(out) if out.status.success() => {
                    if audit_ignores.is_empty() {
                        println!(
                            "  {} No advisories reported by cargo-audit.",
                            "[OK]".green()
                        );
                        EvidenceStatus::NoFindings
                    } else {
                        println!(
                            "  {} No findings outside the governed exceptions; exceptions remain unresolved: {}.",
                            "[OK]".green(),
                            audit_ignores.join(", ")
                        );
                        EvidenceStatus::NoFindingsOutsideExceptions(audit_ignores.to_vec())
                    }
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
                Err(error) => {
                    issues_found += 1;
                    EvidenceStatus::Error(error.to_string())
                }
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
    // `src` and the `src` of workspace members below the current directory.
    let package_roots = package_source_roots(Path::new("."));
    let unsafe_source_available = package_roots.is_some();
    let (unsafe_count, unsafe_warnings) = scan_each(
        package_roots.as_deref().unwrap_or_default(),
        scan_unsafe_code,
    );
    if !unsafe_source_available {
        println!(
            "  {} No project src directory was available; the bounded unsafe scan was not executed.",
            "[INFO]".blue()
        );
    } else if unsafe_count == 0 {
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

        if geiger_status.is_ok_and(|output| output.status.success()) {
            println!(
                "  {} Running full dependency tree unsafe analysis (cargo geiger)...",
                "[GEIGER]".cyan()
            );
            match std::process::Command::new("cargo").arg("geiger").status() {
                Ok(status) if status.success() => {}
                Ok(status) => {
                    println!(
                        "  {} cargo-geiger reported findings or failed with status {}.",
                        "[ERROR]".red().bold(),
                        status
                    );
                    issues_found += 1;
                }
                Err(error) => {
                    println!(
                        "  {} cargo-geiger could not run: {}",
                        "[ERROR]".red().bold(),
                        error
                    );
                    issues_found += 1;
                }
            }
        } else {
            println!(
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
        println!(
            "  {} Checking IDOR / BOLA authorization on parameterized routes...",
            "[IDOR]".bright_yellow()
        );
        if !idor_source_available {
            println!(
                "  {} No scannable Rust source was available; the IDOR/BOLA check is incomplete.",
                "[ERROR]".red().bold()
            );
            issues_found += 1;
        } else if idor_count == 0 {
            println!(
                "  {} The bounded route heuristic found no missing access classifications or recognized guards.",
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
                issues_found += 1;
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
        let surface = inspect_local_network_surface();
        if surface.observations.is_empty() && surface.incomplete.is_empty() {
            println!(
                "  {} No open local listening ports detected.",
                "[OK]".green()
            );
        }
        for report in &surface.observations {
            if report.contains("should be '127.0.0.1'") {
                println!("  {} {}", "[NETWORK WARNING]".yellow().bold(), report);
            } else {
                println!("  {} {}", "[ACTIVE SERVICE]".bright_cyan(), report);
            }
        }
        for reason in &surface.incomplete {
            println!(
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
        println!(
            "\n🤖 {}",
            "Deterministic Security Recommendations:"
                .bright_purple()
                .bold()
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
        write_compliance_report(Path::new("SECURITY_COMPLIANCE.md"), &evidence)?;
        println!(
            "  {} Evidence report written to SECURITY_COMPLIANCE.md",
            "[SUCCESS]".green().bold()
        );
    }

    println!("\nAudit finished. Issues found: {}", issues_found);

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

fn validate_audit_ignores(audit_ignores: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    for advisory in audit_ignores {
        let bytes = advisory.as_bytes();
        let valid = bytes.len() == 17
            && bytes.starts_with(b"RUSTSEC-")
            && bytes[8..12].iter().all(u8::is_ascii_digit)
            && bytes[12] == b'-'
            && bytes[13..].iter().all(u8::is_ascii_digit);
        if !valid {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("invalid --audit-ignore value '{advisory}'; expected RUSTSEC-YYYY-NNNN"),
            )
            .into());
        }
    }
    Ok(())
}

fn cargo_audit_arguments(audit_ignores: &[String]) -> Vec<String> {
    let mut arguments = vec!["audit".to_string()];
    for advisory in audit_ignores {
        arguments.push("--ignore".to_string());
        arguments.push(advisory.clone());
    }
    arguments
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    #[test]
    fn advisory_exception_ids_are_strictly_validated() {
        assert!(validate_audit_ignores(&["RUSTSEC-2099-0001".to_string()]).is_ok());
        for invalid in [
            "rustsec-2099-0001",
            "RUSTSEC-99-0001",
            "RUSTSEC-2099-001",
            "RUSTSEC-2099-0001 --quiet",
        ] {
            assert!(validate_audit_ignores(&[invalid.to_string()]).is_err());
        }
    }

    #[test]
    fn advisory_exceptions_are_forwarded_as_distinct_cargo_audit_arguments() {
        assert_eq!(
            cargo_audit_arguments(&[
                "RUSTSEC-2099-0001".to_string(),
                "RUSTSEC-2099-0002".to_string(),
            ]),
            [
                "audit",
                "--ignore",
                "RUSTSEC-2099-0001",
                "--ignore",
                "RUSTSEC-2099-0002",
            ]
        );
    }

    #[cfg(unix)]
    #[test]
    fn source_scans_do_not_follow_a_symlinked_directory_loop() {
        let project = tempfile::tempdir().expect("temporary project");
        let src = project.path().join("src");
        fs::create_dir_all(&src).expect("source directory");
        fs::write(
            src.join("lib.rs"),
            "pub unsafe fn unchecked() {}\nfn routes() { get(\"/users/:id\" => show); }\n",
        )
        .expect("source fixture");
        // The old walk followed this link until the kernel's symlink bound and
        // reported one copy of every finding per level.
        std::os::unix::fs::symlink(".", src.join("loop")).expect("loop link");

        let (unsafe_count, _) = scan_unsafe_code(&src);
        assert_eq!(unsafe_count, 1);
        let (idor_count, warnings) = scan_idor_vulnerabilities(&src);
        assert_eq!(idor_count, 1, "{warnings:?}");
    }
}
