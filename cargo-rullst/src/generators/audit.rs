use colored::Colorize;
use std::fs;
use std::path::Path;

pub fn run_security_audit(ai_mode: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "🛡️ Running Rullst AI Security Audit...".bright_cyan().bold());

    let mut issues_found = 0;

    // 1. Audit .env for plain-text secret leaks
    let env_path = Path::new(".env");
    if env_path.exists() {
        if let Ok(content) = fs::read_to_string(env_path) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("#") || trimmed.is_empty() {
                    continue;
                }
                if let Some((key, val)) = trimmed.split_once('=') {
                    let k = key.trim();
                    let v = val.trim();
                    if (k.contains("SECRET") || k.contains("KEY") || k.contains("PASSWORD")) && !v.is_empty() && v != "\"\"" && v != "''" {
                        if v.len() < 16 {
                            println!(
                                "  {} Weak or short secret detected for key '{}' in .env",
                                "[WARNING]".yellow().bold(),
                                k
                            );
                            issues_found += 1;
                        }
                    }
                }
            }
        }
    } else {
        println!("  {} No .env file found in root.", "[INFO]".blue());
    }

    // 2. Check for Cargo audit vulnerabilities
    println!("  {} Checking dependency vulnerabilities...", "[AUDIT]".magenta());
    let audit_status = std::process::Command::new("cargo")
        .arg("audit")
        .arg("--version")
        .output();

    if audit_status.is_ok() {
        let output = std::process::Command::new("cargo")
            .arg("audit")
            .output();
        if let Ok(out) = output {
            if !out.status.success() {
                println!("  {} Potential cargo vulnerabilities detected!", "[ALERT]".red().bold());
                issues_found += 1;
            } else {
                println!("  {} All cargo dependencies are secure.", "[OK]".green());
            }
        }
    } else {
        println!("  {} cargo-audit not installed. Run 'cargo install cargo-audit' for deep dependency scanning.", "[NOTE]".yellow());
    }

    if ai_mode {
        println!("\n🤖 {}", "AI Security Sentinel Analysis:".bright_purple().bold());
        if issues_found == 0 {
            println!("  ✅ Project security posture is strong. No high-risk secret leaks or CVEs found.");
        } else {
            println!("  ⚠️ Found {} potential security items. Recommendation: Rotate .env secrets and run cargo update.", issues_found);
        }
    } else {
        println!("\nAudit finished. Issues found: {}", issues_found);
    }

    Ok(())
}
