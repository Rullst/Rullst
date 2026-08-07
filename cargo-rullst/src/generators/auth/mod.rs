// cargo-rullst/src/generators/auth/mod.rs — Root of authentication generator module (< 60 lines).

pub mod controllers;
pub mod mfa;
pub mod models;
pub mod views;

use crate::generators::is_rullst_project;
use colored::*;

pub fn scaffold_auth_system() -> Result<(), Box<dyn std::error::Error>> {
    if !is_rullst_project() {
        println!(
            "{}",
            "❌ Error: This command must be executed in the root of a valid Rullst project."
                .red()
                .bold()
        );
        std::process::exit(1);
    }

    println!(
        "{}",
        "🛡️  Starting scaffolding of Rullst authentication system..."
            .cyan()
            .bold()
    );

    models::generate_user_model_and_migration()?;
    controllers::generate_auth_controllers()?;
    views::generate_auth_views()?;

    println!(
        "{}",
        "✅ Authentication system scaffolded successfully!".green().bold()
    );

    Ok(())
}
