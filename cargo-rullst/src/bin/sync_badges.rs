use std::fs;
use std::path::Path;
use regex::Regex;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Locate the workspace root (parent of cargo-rullst directory)
    let cargo_toml_path = Path::new("Cargo.toml");
    let (cargo_toml_content, root_dir) = if cargo_toml_path.exists() {
        // We are already in the cargo-rullst directory or a subdirectory of Rullst
        let content = fs::read_to_string("Cargo.toml")?;
        if content.contains("name = \"cargo-rullst\"") {
            (content, Path::new("..").to_path_buf())
        } else {
            // We are in workspace root
            let sub_content = fs::read_to_string("cargo-rullst/Cargo.toml")?;
            (sub_content, Path::new(".").to_path_buf())
        }
    } else {
        // Fallback: check parents
        let mut current = std::env::current_dir()?;
        loop {
            if current.join("cargo-rullst/Cargo.toml").exists() {
                let content = fs::read_to_string(current.join("cargo-rullst/Cargo.toml"))?;
                break (content, current);
            }
            if !current.pop() {
                eprintln!("❌ Error: Could not locate cargo-rullst/Cargo.toml in parents.");
                std::process::exit(1);
            }
        }
    };

    // 2. Extract current version from cargo-rullst/Cargo.toml
    let version_regex = Regex::new(r#"(?m)^version\s*=\s*"([^"]+)""#)?;
    let version = version_regex
        .captures(&cargo_toml_content)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str())
        .ok_or("Could not find package version in cargo-rullst/Cargo.toml")?;

    println!("🔎 Detected version: {}", version.cyan().bold());

    // 3. Define target README paths
    let readmes = vec![
        root_dir.join("README.md"),
        root_dir.join("README.pt.md"),
    ];

    let badge_regex = Regex::new(r#"!\[Status:\s*v[^\]]+\]\(https://img\.shields\.io/badge/Status-v[^-]+-emerald\)"#)?;
    let new_badge = format!("![Status: v{}](https://img.shields.io/badge/Status-v{}-emerald)", version, version);

    let mut updated_count = 0;

    for readme_path in readmes {
        if !readme_path.exists() {
            println!("⚠️ Warning: File not found at {:?}", readme_path);
            continue;
        }

        let content = fs::read_to_string(&readme_path)?;
        if badge_regex.is_match(&content) {
            let updated_content = badge_regex.replace_all(&content, &new_badge);
            fs::write(&readme_path, updated_content.as_ref())?;
            println!("✅ Updated badges in {:?}", readme_path.file_name().unwrap());
            updated_count += 1;
        } else {
            println!("ℹ️ No status badge found or already up to date in {:?}", readme_path.file_name().unwrap());
        }
    }

    if updated_count > 0 {
        println!("🚀 All README badges successfully updated to v{}!", version.green().bold());
    } else {
        println!("✨ No badges needed updating.");
    }

    Ok(())
}

// Add terminal coloring support for self-contained output
trait Colorize {
    fn cyan(&self) -> String;
    fn green(&self) -> String;
    fn bold(&self) -> String;
}

impl Colorize for str {
    fn cyan(&self) -> String {
        format!("\x1b[36m{}\x1b[0m", self)
    }
    fn green(&self) -> String {
        format!("\x1b[32m{}\x1b[0m", self)
    }
    fn bold(&self) -> String {
        format!("\x1b[1m{}\x1b[0m", self)
    }
}
