use colored::Colorize;
use std::fs;
use std::path::Path;

pub fn install_git_pre_commit_hook() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}",
        "⚓ Installing Rullst Git Pre-Commit Quality & Security Hook..."
            .bright_cyan()
            .bold()
    );

    let git_dir = Path::new(".git");
    if !git_dir.exists() {
        println!(
            "  {} .git directory not found in the current working directory. Please initialize a Git repository first ('git init').",
            "[ERROR]".red().bold()
        );
        return Ok(());
    }

    let hooks_dir = git_dir.join("hooks");
    if !hooks_dir.exists() {
        fs::create_dir_all(&hooks_dir)?;
    }

    let pre_commit_path = hooks_dir.join("pre-commit");

    let script_content = r#"#!/bin/sh
# Rullst Framework Pre-commit Quality & Security Gatekeeper 🛡️
echo "🛡️ Running Rullst pre-commit quality & security checks..."

# 1. Format check
echo "  🎨 Checking code formatting (rustfmt)..."
cargo fmt --all -- --check
if [ $? -ne 0 ]; then
    echo "❌ rustfmt formatting check failed. Run 'cargo fmt --all' to fix formatting."
    exit 1
fi

# 2. Strict Clippy Linter
echo "  🔍 Running Clippy with zero-warnings policy..."
cargo clippy --workspace --all-targets -- -D warnings
if [ $? -ne 0 ]; then
    echo "❌ Clippy warnings detected. Fix all warnings before committing."
    exit 1
fi

# 3. Static IDOR / BOLA Route Scanner
echo "  🛡️ Running Rullst static IDOR / BOLA route audit..."
if [ -f "cargo-rullst/Cargo.toml" ]; then
    cargo run --quiet -p cargo-rullst --bin rullst -- audit --idor
else
    cargo rullst audit --idor
fi
if [ $? -ne 0 ]; then
    echo "❌ IDOR / BOLA authorization check failed. Enforce RbacGuard on parameterized routes."
    exit 1
fi

echo "✅ All Rullst pre-commit checks passed cleanly!"
exit 0
"#;

    fs::write(&pre_commit_path, script_content)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = fs::metadata(&pre_commit_path) {
            let mut perms = metadata.permissions();
            perms.set_mode(0o755);
            let _ = fs::set_permissions(&pre_commit_path, perms);
        }
    }

    println!(
        "  {} Git pre-commit hook installed successfully at '{}'",
        "[SUCCESS]".green().bold(),
        pre_commit_path.display()
    );
    println!(
        "  {} The hook will automatically run 'cargo fmt', 'cargo clippy -D warnings', and 'cargo rullst audit --idor' on every 'git commit'.",
        "[INFO]".blue()
    );

    Ok(())
}
