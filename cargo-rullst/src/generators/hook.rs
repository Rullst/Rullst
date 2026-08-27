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
    echo "❌ IDOR / BOLA authorization check failed. Classify each parameterized route and enforce its owner, role, or admin boundary."
    exit 1
fi

echo "✅ All Rullst pre-commit checks passed cleanly!"
exit 0
"#;

    fs::write(&pre_commit_path, script_content)?;

    let commit_msg_path = hooks_dir.join("commit-msg");
    let commit_msg_script = r#"#!/bin/sh
# Rullst Framework Conventional Commits Hook
commit_regex='^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([a-z0-9_-]+\))?!?: .{1,80}$'
merge_regex='^Merge branch .*'

commit_message=$(head -n 1 "$1")

if echo "$commit_message" | grep -qE "$merge_regex"; then
    exit 0
fi

if ! echo "$commit_message" | grep -qE "$commit_regex"; then
    echo "❌ ERROR: Invalid commit message format."
    echo "   Commit message must follow Conventional Commits: <type>(<scope>): <description>"
    echo "   Allowed types: feat, fix, docs, style, refactor, perf, test, build, ci, chore, revert"
    echo "   Example: fix(auth): use spawn_blocking for async password hashing"
    exit 1
fi
"#;

    fs::write(&commit_msg_path, commit_msg_script)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = fs::metadata(&pre_commit_path) {
            let mut perms = metadata.permissions();
            perms.set_mode(0o755);
            let _ = fs::set_permissions(&pre_commit_path, perms);
        }
        if let Ok(metadata) = fs::metadata(&commit_msg_path) {
            let mut perms = metadata.permissions();
            perms.set_mode(0o755);
            let _ = fs::set_permissions(&commit_msg_path, perms);
        }
    }

    println!(
        "  {} Git hooks installed successfully at '{}' and '{}'",
        "[SUCCESS]".green().bold(),
        pre_commit_path.display(),
        commit_msg_path.display()
    );
    println!(
        "  {} The hooks will enforce Conventional Commits, 'cargo fmt', 'cargo clippy -D warnings', and IDOR scans.",
        "[INFO]".blue()
    );

    Ok(())
}
