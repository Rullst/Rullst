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
    require_migration_runner("cargo rullst auth")?;
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

/// The account store's migration only runs when the crate root declares the
/// `migrations` module and an entry point passes `migrations::get_migrations()`
/// to the runner, as `cargo rullst new --database ...` generates. Without them
/// (for example `--no-database`) the scaffold would add a migration nothing
/// compiles or applies, so it refuses before writing.
pub(super) fn require_migration_runner(command: &str) -> Result<(), IoError> {
    let roots = ["src/main.rs", "src/lib.rs"]
        .into_iter()
        .filter_map(|path| fs::read_to_string(path).ok())
        .collect::<Vec<_>>();
    if has_migration_runner(&roots) {
        return Ok(());
    }
    Err(IoError::new(
        ErrorKind::InvalidInput,
        format!(
            "{command} adds a SQL migration, but this project has no migration runner: declare `pub mod migrations;` in src/main.rs or src/lib.rs and call `rullst::artisan!(crate::migrations::get_migrations());` at startup, with a DATABASE_URL in .env (projects created with `cargo rullst new <name> --database sqlite` include them)"
        ),
    ))
}

fn has_migration_runner(roots: &[String]) -> bool {
    let declares_migrations = roots.iter().any(|source| {
        syn::parse_file(source).is_ok_and(|file| {
            file.items
                .iter()
                .any(|item| matches!(item, syn::Item::Mod(module) if module.ident == "migrations"))
        })
    });
    declares_migrations
        && roots
            .iter()
            .any(|source| source.contains("get_migrations("))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_runner_requires_the_module_and_its_registration() {
        let main = |source: &str| vec![source.to_string()];
        assert!(!has_migration_runner(&main("fn main() {}\n")));
        assert!(!has_migration_runner(&main(
            "pub mod migrations;\nfn main() {}\n"
        )));
        assert!(!has_migration_runner(&main(
            "fn main() { rullst::artisan!(crate::migrations::get_migrations()); }\n"
        )));
        assert!(has_migration_runner(&main(
            "pub mod migrations;\nfn main() { rullst::artisan!(crate::migrations::get_migrations()); }\n"
        )));
        // Hot-reload starters declare the module in lib.rs and run it in main.rs.
        assert!(has_migration_runner(&[
            "pub mod migrations;\n".to_string(),
            "fn main() { rullst::artisan!(crate::migrations::get_migrations()); }\n".to_string(),
        ]));
        assert!(!has_migration_runner(&main("not rust get_migrations(")));
    }
}
