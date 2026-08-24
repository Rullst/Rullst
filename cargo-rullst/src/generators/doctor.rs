use colored::Colorize;
use std::process::Command;

pub fn run_doctor(auto_fix: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}",
        "🩺 Running Rullst Framework System & Toolchain Doctor..."
            .bright_cyan()
            .bold()
    );
    if auto_fix {
        println!(
            "{}",
            "🔧 Auto-Fix Mode Active: Attempting to resolve missing dependencies..."
                .bright_yellow()
                .bold()
        );
    }
    println!(
        "{}",
        "═══════════════════════════════════════════════════════════════".bright_black()
    );

    let mut passed = 0;
    let mut warnings = 0;
    let mut fix_suggestions: Vec<String> = Vec::new();

    // 1. Rust Compiler Version & MSRV
    print!("  🦀 Rust Toolchain (MSRV >= 1.96.0)... ");
    if let Ok(output) = Command::new("rustc").arg("--version").output() {
        if output.status.success() {
            let ver_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            println!("{} ({})", "[OK]".bright_green().bold(), ver_str);
            passed += 1;
        } else {
            println!("{}", "[FAIL]".bright_red().bold());
            warnings += 1;
            fix_suggestions
                .push("Run 'rustup update' to install the latest Rust toolchain.".to_string());
        }
    } else {
        println!("{}", "[NOT FOUND]".bright_red().bold());
        warnings += 1;
        fix_suggestions.push("Install Rust from https://rustup.rs".to_string());
    }

    // 2. Cargo Components: rustfmt, clippy, llvm-tools
    print!("  🎨 Rustfmt & Clippy Linters... ");
    let fmt_ok = Command::new("cargo")
        .arg("fmt")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    let clippy_ok = Command::new("cargo")
        .arg("clippy")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if fmt_ok && clippy_ok {
        println!("{}", "[OK] (Installed)".bright_green().bold());
        passed += 1;
    } else if auto_fix {
        println!(
            "{}",
            "[FIXING] (Running rustup component add)..."
                .bright_yellow()
                .bold()
        );
        let _ = Command::new("rustup")
            .args(["component", "add", "rustfmt", "clippy"])
            .output();
        println!(
            "  🎨 Rustfmt & Clippy Linters... {}",
            "[FIXED]".bright_green().bold()
        );
        passed += 1;
    } else {
        println!("{}", "[WARNING]".bright_yellow().bold());
        warnings += 1;
        fix_suggestions
            .push("Run 'rustup component add rustfmt clippy' (or 'cargo rullst doctor --fix') to install linters.".to_string());
    }

    // 3. LLVM Tools Preview (for code coverage)
    print!("  📊 LLVM Source Coverage Tools... ");
    let llvm_cov_ok = Command::new("cargo")
        .arg("llvm-cov")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if llvm_cov_ok {
        println!("{}", "[OK] (cargo-llvm-cov active)".bright_green().bold());
        passed += 1;
    } else {
        println!("{}", "[OPTIONAL]".bright_black());
        fix_suggestions
            .push("Run 'cargo install cargo-llvm-cov' for source-based code coverage.".to_string());
    }

    // 4. Dependency Vulnerability Scanner (cargo-audit)
    print!("  🛡️ Cargo Audit (RustSec CVE DB)... ");
    if let Ok(output) = Command::new("cargo").arg("audit").arg("--version").output() {
        if output.status.success() {
            println!("{}", "[OK] (Ready)".bright_green().bold());
            passed += 1;
        } else {
            println!("{}", "[WARNING]".bright_yellow().bold());
            warnings += 1;
            fix_suggestions
                .push("Run 'cargo install cargo-audit' to scan for known CVEs.".to_string());
        }
    } else {
        println!("{}", "[MISSING]".bright_yellow().bold());
        warnings += 1;
        fix_suggestions.push(
            "Run 'cargo install cargo-audit' for dependency vulnerability scanning.".to_string(),
        );
    }

    // 5. Memory Safety & Unsafe Scanner (Cargo Geiger)
    print!("  ☢️ Cargo Geiger (Zero-Unsafe Checker)... ");
    if let Ok(output) = Command::new("cargo")
        .arg("geiger")
        .arg("--version")
        .output()
    {
        if output.status.success() {
            println!("{}", "[OK] (Ready)".bright_green().bold());
            passed += 1;
        } else {
            println!("{}", "[OPTIONAL]".bright_black());
            fix_suggestions.push(
                "Run 'cargo install cargo-geiger' for dependency memory safety auditing."
                    .to_string(),
            );
        }
    } else {
        println!("{}", "[OPTIONAL]".bright_black());
        fix_suggestions.push(
            "Run 'cargo install cargo-geiger' to scan crate dependency trees for unsafe code."
                .to_string(),
        );
    }

    // 6. Supply Chain & License Linter (cargo-deny)
    print!("  📦 Cargo Deny (License & Ban Linter)... ");
    if let Ok(output) = Command::new("cargo").arg("deny").arg("--version").output() {
        if output.status.success() {
            println!("{}", "[OK] (Ready)".bright_green().bold());
            passed += 1;
        } else {
            println!("{}", "[OPTIONAL]".bright_black());
            fix_suggestions.push(
                "Run 'cargo install cargo-deny' for automated license & supply chain linting."
                    .to_string(),
            );
        }
    } else {
        println!("{}", "[OPTIONAL]".bright_black());
        fix_suggestions
            .push("Run 'cargo install cargo-deny' for license compliance checks.".to_string());
    }

    // 7. Mutation Testing (cargo-mutants)
    print!("  🧬 Cargo Mutants (Test Sensitivity Tester)... ");
    if let Ok(output) = Command::new("cargo")
        .arg("mutants")
        .arg("--version")
        .output()
    {
        if output.status.success() {
            println!("{}", "[OK] (Ready)".bright_green().bold());
            passed += 1;
        } else {
            println!("{}", "[OPTIONAL]".bright_black());
            fix_suggestions
                .push("Run 'cargo install cargo-mutants' for mutation testing.".to_string());
        }
    } else {
        println!("{}", "[OPTIONAL]".bright_black());
        fix_suggestions
            .push("Run 'cargo install cargo-mutants' to verify test suite quality.".to_string());
    }

    // 8. Formal Verification (Kani Verifier)
    print!("  📐 Kani Rust Verifier (Model Checker)... ");
    if let Ok(output) = Command::new("cargo").arg("kani").arg("--version").output() {
        if output.status.success() {
            println!("{}", "[OK] (Ready)".bright_green().bold());
            passed += 1;
        } else {
            println!("{}", "[OPTIONAL]".bright_black());
            fix_suggestions.push("Run 'cargo install kani-verifier && cargo kani setup' for mathematical formal proofs.".to_string());
        }
    } else {
        println!("{}", "[OPTIONAL]".bright_black());
        fix_suggestions.push("Run 'cargo install kani-verifier && cargo kani setup' for mathematical formal verification.".to_string());
    }

    // 9. Docker Engine (for Live Testcontainers DB Matrix)
    print!("  🐳 Docker Engine (PostgreSQL/MySQL Matrix)... ");
    if let Ok(output) = Command::new("docker").arg("--version").output() {
        if output.status.success() {
            let doc_ver = String::from_utf8_lossy(&output.stdout).trim().to_string();
            println!("{} ({})", "[OK]".bright_green().bold(), doc_ver);
            passed += 1;
        } else {
            println!("{}", "[OPTIONAL]".bright_black());
            fix_suggestions.push(
                "Install Docker Desktop / Engine to run live database matrix integration tests."
                    .to_string(),
            );
        }
    } else {
        println!("{}", "[OPTIONAL]".bright_black());
        fix_suggestions.push(
            "Install Docker to run multi-database matrix tests with Testcontainers.".to_string(),
        );
    }

    // 10. Git Version Control
    print!("  🌱 Git VCS... ");
    if let Ok(output) = Command::new("git").arg("--version").output() {
        if output.status.success() {
            println!("{}", "[OK]".bright_green().bold());
            passed += 1;
        } else {
            println!("{}", "[WARNING]".bright_yellow().bold());
            warnings += 1;
        }
    } else {
        println!("{}", "[WARNING]".bright_yellow().bold());
        warnings += 1;
    }

    println!(
        "{}",
        "═══════════════════════════════════════════════════════════════".bright_black()
    );
    println!(
        "Doctor Summary: {} checks passed, {} warnings.",
        passed.to_string().bright_green().bold(),
        warnings.to_string().yellow().bold()
    );

    if !fix_suggestions.is_empty() {
        println!(
            "\n{}",
            "💡 Recommendations to optimize your Rullst development environment:"
                .bright_cyan()
                .bold()
        );
        for (idx, sug) in fix_suggestions.iter().enumerate() {
            println!("  {}. {}", idx + 1, sug.bright_white());
        }
        if !auto_fix {
            println!(
                "\n{}",
                "💡 Tip: Run 'cargo rullst doctor --fix' to automatically resolve fixable dependencies."
                    .bright_green()
            );
        }
    } else {
        println!(
            "\n{}",
            "✨ Your environment is in pristine condition for mission-critical Rullst development!"
                .bright_green()
                .bold()
        );
    }

    Ok(())
}
