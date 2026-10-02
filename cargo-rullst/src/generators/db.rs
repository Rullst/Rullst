// src/generators/db.rs — Database runner generator.

use crate::generators::is_rullst_project;
use colored::*;

pub fn run_project_db_command(command: &str) -> Result<(), Box<dyn std::error::Error>> {
    if !is_rullst_project() {
        return Err(crate::ui::error_report::ProjectRequired.into());
    }

    println!(
        "{}",
        format!("⏳ Running 'cargo run -- {}'...", command)
            .cyan()
            .bold()
    );

    let status = std::process::Command::new("cargo")
        .args(["run", "--", command])
        .status()?;

    if !status.success() {
        println!(
            "{}",
            format!("❌ Failed to execute db command: {}", command)
                .red()
                .bold()
        );
        std::process::exit(status.code().unwrap_or(1));
    }

    Ok(())
}
