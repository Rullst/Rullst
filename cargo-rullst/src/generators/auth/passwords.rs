// cargo-rullst/src/generators/auth/passwords.rs — Password change and reset
// flows shared by `cargo rullst auth` and the SaaS and LMS starters.

use crate::generators::migration::regenerate_migrations_mod;
use crate::generators::{output_guard::write_new, register_mod_ast};
use colored::*;
use std::fs;
use std::path::Path;

const CONTROLLER_TEMPLATE: &str = include_str!("password_controller.rs.template");
const MODEL_TEMPLATE: &str = include_str!("password_reset_model.rs.template");
const MIGRATION_TEMPLATE: &str = include_str!("password_reset_migration.rs.template");

/// File name suffix of the `password_reset_tokens` migration.
pub(crate) const MIGRATION_SUFFIX: &str = "_create_password_reset_tokens_table.rs";

/// `src/controllers/password_controller.rs`: change, forgot and reset handlers.
pub(crate) fn render_password_controller() -> String {
    CONTROLLER_TEMPLATE.to_string()
}

/// `src/models/password_reset.rs`: the digest-only, single-use token store.
pub(crate) fn render_password_reset_model() -> String {
    MODEL_TEMPLATE.to_string()
}

/// The reversible `password_reset_tokens` migration named `migration_name`.
pub(crate) fn render_password_reset_migration(migration_name: &str) -> String {
    MIGRATION_TEMPLATE.replace("__MIGRATION_NAME__", migration_name)
}

/// Writes the password flows of `cargo rullst auth` beside its users table.
pub(crate) fn generate_password_flows(timestamp: &str) -> Result<(), Box<dyn std::error::Error>> {
    let migration_name = format!("m{timestamp}_create_password_reset_tokens_table");
    write_new(
        &Path::new("src/migrations").join(format!("{migration_name}.rs")),
        render_password_reset_migration(&migration_name).as_bytes(),
    )?;
    regenerate_migrations_mod()?;
    println!(
        "{}",
        "  ✨ Created 'password_reset_tokens' table migration.".green()
    );

    let models_dir = Path::new("src/models");
    fs::create_dir_all(models_dir)?;
    write_new(
        &models_dir.join("password_reset.rs"),
        render_password_reset_model().as_bytes(),
    )?;
    register_mod_ast(&models_dir.join("mod.rs"), "password_reset")?;

    let controllers_dir = Path::new("src/controllers");
    fs::create_dir_all(controllers_dir)?;
    write_new(
        &controllers_dir.join("password_controller.rs"),
        render_password_controller().as_bytes(),
    )?;
    register_mod_ast(&controllers_dir.join("mod.rs"), "password_controller")?;
    println!(
        "{}",
        "  ✨ Created the password change and reset controller.".green()
    );
    Ok(())
}

#[cfg(test)]
#[path = "passwords_tests.rs"]
mod tests;
