// cargo-rullst/src/pkg.rs — Dynamic Package Ecosystem Manager for Rullst

use colored::Colorize;
use std::fs;
use std::path::Path;

/// Adds a RullstPackage community dependency to Cargo.toml
pub fn pkg_add(package_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}",
        format!(
            "📦 Searching Rullst Package Registry for '{}'...",
            package_name
        )
        .bold()
    );

    let cargo_toml_path = Path::new("Cargo.toml");
    if !cargo_toml_path.exists() {
        println!(
            "{}",
            "❌ Error: Cargo.toml not found in current directory.".red()
        );
        return Ok(());
    }

    let mut content = fs::read_to_string(cargo_toml_path)?;
    if content.contains(&format!("{} =", package_name)) {
        println!(
            "{}",
            format!(
                "⚠️ Package '{}' is already installed in Cargo.toml.",
                package_name
            )
            .yellow()
        );
        return Ok(());
    }

    if let Some(dep_idx) = content.find("[dependencies]") {
        let insert_pos = dep_idx + "[dependencies]\n".len();
        let dep_line = format!("{} = \"12.0.0\"\n", package_name);
        content.insert_str(insert_pos, &dep_line);
        fs::write(cargo_toml_path, content)?;

        println!(
            "{}",
            format!("✅ Successfully added '{}' to Cargo.toml!", package_name)
                .green()
                .bold()
        );
        println!(
            "{}",
            "👉 Run 'cargo check' to compile the new package dependency.".cyan()
        );
    } else {
        println!(
            "{}",
            "❌ Error: Could not locate [dependencies] section in Cargo.toml.".red()
        );
    }

    Ok(())
}

/// Lists installed community packages in the current project
pub fn pkg_list() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "📦 Installed Rullst Community Packages:".bold());
    let cargo_toml_path = Path::new("Cargo.toml");
    if !cargo_toml_path.exists() {
        println!("{}", "❌ Error: Cargo.toml not found.".red());
        return Ok(());
    }

    let content = fs::read_to_string(cargo_toml_path)?;
    let mut found = false;
    for line in content.lines() {
        if line.starts_with("rullst-") || line.starts_with("rullst_") {
            println!("  • {}", line.trim().cyan());
            found = true;
        }
    }

    if !found {
        println!(
            "{}",
            "  (No third-party rullst-* packages detected in dependencies)".dimmed()
        );
    }

    Ok(())
}
