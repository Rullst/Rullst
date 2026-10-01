// cargo-rullst/src/generators/auth/mod.rs — Root of authentication generator module.

pub mod controllers;
pub mod mfa;
pub mod models;
pub mod views;

use crate::generators::chat::ensure_rullst_features;
use crate::generators::output_guard::{existing_migrations, reject_existing};
use crate::generators::{
    ProjectOrmBackend, is_rullst_project, project_orm_backend, register_mod_ast,
};
use colored::*;
use std::fs;
use std::io::{Error as IoError, ErrorKind};
use std::path::{Path, PathBuf};

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
    reject_turso_primary("cargo rullst auth")?;
    reject_existing_outputs()?;
    let root_module = project_root_module()?;
    let manifest_path = Path::new("Cargo.toml");
    let manifest = fs::read_to_string(manifest_path)?;
    let updated_manifest = ensure_rullst_features(&manifest, &["orm", "auth"])?;

    println!(
        "{}",
        "🛡️  Starting scaffolding of Rullst authentication system..."
            .cyan()
            .bold()
    );

    models::generate_user_model_and_migration()?;
    controllers::generate_auth_controllers()?;
    views::generate_auth_views()?;
    fs::write(manifest_path, updated_manifest)?;
    for module in ["controllers", "middlewares", "models", "pages"] {
        register_mod_ast(&root_module, module)?;
    }

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

/// The authentication store uses SQLx query builders and transactions.
pub(super) fn reject_turso_primary(command: &str) -> Result<(), IoError> {
    if project_orm_backend() == ProjectOrmBackend::Turso {
        return Err(IoError::new(
            ErrorKind::InvalidInput,
            format!(
                "{command} generates a SQLx account store; Turso-primary projects are not supported"
            ),
        ));
    }
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

pub(super) fn project_root_module() -> Result<PathBuf, IoError> {
    ["src/lib.rs", "src/main.rs"]
        .into_iter()
        .map(PathBuf::from)
        .find(|path| path.exists())
        .ok_or_else(|| {
            IoError::new(
                ErrorKind::NotFound,
                "Rullst project has neither src/lib.rs nor src/main.rs",
            )
        })
}
