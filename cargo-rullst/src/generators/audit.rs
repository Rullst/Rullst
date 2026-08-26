use colored::Colorize;
use std::fs;
use std::path::Path;

use crate::generators::audit_compliance::{
    ComplianceEvidence, EvidenceStatus, write_compliance_report,
};
pub use crate::generators::audit_evidence::{generate_cyclonedx_sbom, scan_local_network_surface};

/// Recursively scans Rust source files for parameterized route paths lacking RBAC / Ownership enforcement.
pub fn scan_idor_vulnerabilities(src_dir: &Path) -> (usize, Vec<String>) {
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
                    for line in content.lines() {
                        let trimmed = line.trim();
                        if (trimmed.contains("\":id\"")
                            || trimmed.contains("/:id")
                            || trimmed.contains("/:user_id")
                            || trimmed.contains("/:order_id")
                            || trimmed.contains("/:item_id")
                            || trimmed.contains("/{id}")
                            || trimmed.contains("/{user_id}"))
                            && !content.contains("RbacGuard")
                            && !content.contains("authorize_owner")
                            && !content.contains("UserContext")
                        {
                            let msg = format!(
                                "File '{}': Parameterized route detected without RbacGuard ownership authorization",
                                path.display()
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
    let (idor_count, idor_warnings) = scan_idor_vulnerabilities(Path::new("src"));
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
            idor_scan: if !Path::new("src").is_dir() {
                EvidenceStatus::NotChecked("no src directory was available to inspect")
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

    Ok(())
}
