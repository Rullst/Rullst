// cargo-rullst/src/generators/auth/mod.rs — Root of authentication generator module.

pub mod controllers;
pub mod mfa;
pub mod models;
pub(crate) mod passwords;
pub(crate) mod users_table;
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
use users_table::UsersTable;

/// Application files owned by the authentication scaffold.
const AUTH_OUTPUTS: [&str; 6] = [
    "src/models/user.rs",
    "src/models/password_reset.rs",
    "src/controllers/auth_controller.rs",
    "src/controllers/password_controller.rs",
    "src/middlewares/auth_middleware.rs",
    "src/pages/auth.rs",
];

pub fn scaffold_auth_system() -> Result<(), Box<dyn std::error::Error>> {
    scaffold_auth().map(|_| ())
}

/// Runs `cargo rullst auth`; returns whether it wrote files, so the project
/// context is refreshed only after a scaffold.
pub(crate) fn scaffold_auth() -> Result<bool, Box<dyn std::error::Error>> {
    if !is_rullst_project() {
        return Err(crate::ui::error_report::ProjectRequired.into());
    }
    // The SaaS starter, or an earlier run, already has every account file:
    // nothing to do is a success that writes nothing.
    if scaffold_present()? {
        println!(
            "{}",
            "✅ Authentication is already scaffolded: every file `cargo rullst auth` writes exists (the SaaS starter ships them), so nothing was written."
                .green()
        );
        println!(
            "👉 Its routes, the account model (src/models/user.rs) and the password flows are yours to extend; Tutorial 11 describes them."
        );
        return Ok(false);
    }
    reject_turso_primary("cargo rullst auth")?;
    require_migration_runner("cargo rullst auth")?;
    let users = reject_existing_outputs()?;
    let root_module = project_root_module()?;
    let manifest_path = Path::new("Cargo.toml");
    let manifest = fs::read_to_string(manifest_path)?;
    // `mail` sends the password reset links and password-changed notices.
    let updated_manifest = ensure_rullst_features(&manifest, &["orm", "auth", "mail"])?;

    println!(
        "{}",
        "🛡️  Starting scaffolding of Rullst authentication system..."
            .cyan()
            .bold()
    );

    // One timestamp: the users migration sorts before the reset tokens one.
    let timestamp = chrono::Local::now().format("%Y%m%d%H%M%S").to_string();
    models::write_user_model_and_migration(&users, &timestamp)?;
    passwords::generate_password_flows(&timestamp)?;
    controllers::generate_auth_controllers()?;
    views::generate_auth_views()?;
    fs::write(manifest_path, updated_manifest)?;
    for module in ["controllers", "middlewares", "models", "pages"] {
        register_mod_ast(&root_module, module)?;
    }
    // The starter's two-field `User` would be a second model of `users`.
    if users_table::replace_starter_user_model(&root_module)? {
        println!(
            "{}",
            format!(
                "  ✨ Replaced the starter's `User` in {} with the account model.",
                crate::generators::slash_path(&root_module)
            )
            .green()
        );
    }

    println!(
        "{}",
        "✅ Authentication system scaffolded successfully!"
            .green()
            .bold()
    );
    println!(
        "👉 Mount the login/register/logout routes behind the CSRF and security baseline, and protect private routes and POST /logout/everywhere (logout_everywhere) with auth_middleware."
    );
    println!(
        "👉 Mount password_controller's /password/forgot and /password/reset (GET view, POST submit) and, behind auth_middleware, /password/change; put credential_rate_limit on the POST routes."
    );
    println!(
        "👉 Set RULLST_PUBLIC_ORIGIN (the HTTPS origin reset links use), MAIL_DRIVER and MAIL_FROM before production, and APP_NAME (the product name in account emails)."
    );
    match users {
        UsersTable::Create => println!(
            "👉 Run `cargo rullst db:migrate` to create the users and password_reset_tokens tables. Call User::revoke_sessions when you deactivate an account."
        ),
        UsersTable::ExtendStarter(_) => println!(
            "👉 Run `cargo rullst db:migrate` to add the account columns to users (existing rows keep their data and get a placeholder user-<id>@no-email.invalid email and no password) and create password_reset_tokens. Call User::revoke_sessions when you deactivate an account."
        ),
    }
    Ok(true)
}

/// Whether every application file of the scaffold exists already.
fn scaffold_present() -> Result<bool, IoError> {
    for output in AUTH_OUTPUTS {
        match fs::symlink_metadata(output) {
            Ok(_) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(error),
        }
    }
    Ok(true)
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

/// Refuses existing account files and reset-token migrations, then plans the
/// users table.
fn reject_existing_outputs() -> Result<UsersTable, IoError> {
    let outputs = AUTH_OUTPUTS.map(PathBuf::from);
    reject_existing(
        "authentication scaffold",
        &outputs,
        "; integrate authentication into the existing files manually",
    )?;
    let resets = existing_migrations(&[passwords::MIGRATION_SUFFIX.to_string()])?;
    if let Some(existing) = resets.first() {
        return Err(IoError::new(
            ErrorKind::AlreadyExists,
            format!(
                "refusing to add a second password_reset_tokens migration: {} already creates it",
                existing.display()
            ),
        ));
    }
    users_table::plan()
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
