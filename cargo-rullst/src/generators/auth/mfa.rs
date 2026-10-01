// cargo-rullst/src/generators/auth/mfa.rs — Server-side TOTP second-factor scaffold.

use super::{project_root_module, reject_turso_primary, require_migration_runner};
use crate::generators::chat::ensure_rullst_features;
use crate::generators::migration::regenerate_migrations_mod;
use crate::generators::output_guard::{existing_migrations, reject_existing, write_new};
use crate::generators::{is_rullst_project, register_mod_ast};
use colored::*;
use std::fs;
use std::path::{Path, PathBuf};

const CONTROLLER_PATH: &str = "src/controllers/mfa.rs";
const MIGRATION_SUFFIX: &str = "_create_user_mfa_factors_table.rs";
const CONTROLLER_TEMPLATE: &str = include_str!("mfa_controller.rs.template");
const MIGRATION_TEMPLATE: &str = include_str!("mfa_migration.rs.template");

/// Returns the generated controller, which keeps every TOTP secret server-side.
pub(crate) fn render_mfa_controller() -> &'static str {
    CONTROLLER_TEMPLATE
}

/// Returns the reversible `user_mfa_factors` migration.
pub(crate) fn render_mfa_migration(migration_name: &str) -> String {
    MIGRATION_TEMPLATE.replace("__MIGRATION_NAME__", migration_name)
}

pub fn scaffold_mfa_system() -> Result<(), Box<dyn std::error::Error>> {
    if !is_rullst_project() {
        println!(
            "{}",
            "❌ Error: This command must be executed in the root of a valid Rullst project."
                .red()
                .bold()
        );
        std::process::exit(1);
    }
    reject_turso_primary("make:mfa")?;
    require_migration_runner("make:mfa")?;
    let mut outputs = vec![PathBuf::from(CONTROLLER_PATH)];
    outputs.extend(existing_migrations(&[MIGRATION_SUFFIX.to_string()])?);
    reject_existing(
        "second-factor scaffold",
        &outputs,
        "; integrate the second factor into the existing files manually",
    )?;
    let root_module = project_root_module()?;
    let manifest_path = Path::new("Cargo.toml");
    let manifest = fs::read_to_string(manifest_path)?;
    let updated_manifest = ensure_rullst_features(&manifest, &["orm", "security"])?;

    println!(
        "{}",
        "🔐 Scaffolding Rullst server-side TOTP second factor..."
            .cyan()
            .bold()
    );

    let timestamp = chrono::Local::now().format("%Y%m%d%H%M%S");
    let migration_name = format!("m{timestamp}_create_user_mfa_factors_table");
    let migrations_dir = Path::new("src/migrations");
    let controllers_dir = Path::new("src/controllers");
    fs::create_dir_all(migrations_dir)?;
    fs::create_dir_all(controllers_dir)?;
    write_new(
        Path::new(CONTROLLER_PATH),
        render_mfa_controller().as_bytes(),
    )?;
    write_new(
        &migrations_dir.join(format!("{migration_name}.rs")),
        render_mfa_migration(&migration_name).as_bytes(),
    )?;
    fs::write(manifest_path, updated_manifest)?;
    register_mod_ast(&controllers_dir.join("mod.rs"), "mfa")?;
    register_mod_ast(&root_module, "controllers")?;
    regenerate_migrations_mod()?;

    println!(
        "  {} Scaffolded {CONTROLLER_PATH} and the user_mfa_factors migration",
        "[CREATE]".green().bold(),
    );
    println!(
        "👉 Mount mfa_setup, mfa_confirm and mfa_verify as POST routes behind auth_middleware (`cargo rullst auth`), CSRF and rate limiting."
    );
    println!(
        "👉 Call verify_second_factor before granting a full session; secrets never leave the server after enrollment."
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mfa_scaffold_keeps_secrets_server_side_with_replay_protection() {
        let controller = render_mfa_controller();
        syn::parse_file(controller).expect("MFA controller must parse");
        let migration = render_mfa_migration("m20261001000000_create_user_mfa_factors_table");
        syn::parse_file(&migration).expect("MFA migration must parse");
        assert!(!migration.contains("__MIGRATION_NAME__"));

        assert!(controller.contains("Extension(user_id): Extension<i32>"));
        assert!(controller.contains("verify_totp_step_after"));
        assert!(controller.contains("last_accepted_step < ?"));
        assert!(controller.contains("rows_affected()"));
        assert!(controller.contains("use rullst::server::"));
        assert!(controller.contains("use rullst::security_runtime::"));
        // The verifying form never carries the secret.
        let form = controller
            .split("pub struct MfaCodeForm")
            .nth(1)
            .and_then(|rest| rest.split('}').next())
            .expect("code form");
        assert!(!form.contains("secret"));
        for forbidden in [
            "use axum",
            "rullst_security::",
            "verify_totp_code",
            "user@example.com",
            ".unwrap(",
            ".expect(",
            "panic!(",
        ] {
            assert!(!controller.contains(forbidden), "{forbidden}");
        }
    }
}
