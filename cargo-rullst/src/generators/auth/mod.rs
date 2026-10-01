// cargo-rullst/src/generators/auth/mod.rs — Root of authentication generator module.

pub mod controllers;
pub mod mfa;
pub mod models;
pub mod views;

use crate::generators::is_rullst_project;
use crate::generators::output_guard::{existing_migrations, reject_existing};
use colored::*;
use std::io::{Error as IoError, ErrorKind};
use std::path::PathBuf;

/// Application files owned by the authentication scaffold.
const AUTH_OUTPUTS: [&str; 4] = [
    "src/models/user.rs",
    "src/controllers/auth_controller.rs",
    "src/middlewares/auth_middleware.rs",
    "src/pages/auth.rs",
];

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
    reject_existing_outputs()?;

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
        "✅ Authentication system scaffolded successfully!"
            .green()
            .bold()
    );
    println!(
        "👉 Mount the login/register/logout routes behind the CSRF and security baseline, and protect private routes with auth_middleware."
    );
    println!("👉 Run `cargo rullst db:migrate` to create the users table.");
    Ok(())
}

fn reject_existing_outputs() -> Result<(), IoError> {
    let outputs = AUTH_OUTPUTS.map(PathBuf::from);
    reject_existing(
        "authentication scaffold",
        &outputs,
        "; integrate authentication into the existing files manually",
    )?;
    let users = existing_migrations(&[
        "_create_users.rs".to_string(),
        "_create_users_table.rs".to_string(),
    ])?;
    if users.is_empty() {
        return Ok(());
    }
    let users = users
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();
    Err(IoError::new(
        ErrorKind::AlreadyExists,
        format!(
            "refusing to add a second users table migration: {} already creates users. Add the email, password_hash, oauth_provider and oauth_id columns with `cargo rullst make:migration` and integrate authentication manually, or remove that migration before its table holds data",
            users.join(", ")
        ),
    ))
}
