//! Fail-closed billing scaffold for relational and Turso-primary projects.

use crate::generators::chat::ensure_rullst_features;
use crate::generators::migration::regenerate_migrations_mod;
use crate::generators::{
    ProjectOrmBackend, is_rullst_project, is_valid_rust_identifier, model_to_snake_case,
    project_orm_backend, register_mod_ast,
};
use colored::*;
use std::fs;
use std::io::{Error as IoError, ErrorKind};
use std::path::{Path, PathBuf};

const BILLING_CONTROLLER_TEMPLATE: &str = include_str!("billing_controller.rs.template");
const SQLX_SUBSCRIPTION_MODEL: &str = include_str!("billing_subscription_sqlx.rs.template");
const TURSO_SUBSCRIPTION_MODEL: &str = include_str!("billing_subscription_turso.rs.template");
const SQLX_CUSTOMER_MODEL: &str = include_str!("billing_customer_sqlx.rs.template");
const TURSO_CUSTOMER_MODEL: &str = include_str!("billing_customer_turso.rs.template");
const SQLX_MIGRATION: &str = include_str!("billing_migration_sqlx.rs.template");
const TURSO_MIGRATION: &str = include_str!("billing_migration_turso.rs.template");

const FIXED_OUTPUTS: [&str; 4] = [
    "src/models/subscription.rs",
    "src/models/billing_customer.rs",
    "src/pages/billing.rs",
    "src/controllers/billing_controller.rs",
];

pub(crate) fn render_billing_controller(foreign_key: &str, backend: ProjectOrmBackend) -> String {
    let owner_id_type = match backend {
        ProjectOrmBackend::Sqlx => "i32",
        ProjectOrmBackend::Turso => "i64",
    };
    BILLING_CONTROLLER_TEMPLATE
        .replace("__FOREIGN_KEY__", foreign_key)
        .replace("__OWNER_ID_TYPE__", owner_id_type)
}

pub(crate) fn render_billing_models(
    foreign_key: &str,
    backend: ProjectOrmBackend,
) -> (String, String) {
    let (subscription, customer) = match backend {
        ProjectOrmBackend::Sqlx => (SQLX_SUBSCRIPTION_MODEL, SQLX_CUSTOMER_MODEL),
        ProjectOrmBackend::Turso => (TURSO_SUBSCRIPTION_MODEL, TURSO_CUSTOMER_MODEL),
    };
    (
        subscription.replace("__FOREIGN_KEY__", foreign_key),
        customer.replace("__FOREIGN_KEY__", foreign_key),
    )
}

pub(crate) fn render_billing_migration(
    migration_name: &str,
    foreign_key: &str,
    backend: ProjectOrmBackend,
) -> String {
    let template = match backend {
        ProjectOrmBackend::Sqlx => SQLX_MIGRATION,
        ProjectOrmBackend::Turso => TURSO_MIGRATION,
    };
    template
        .replace("__MIGRATION_NAME__", migration_name)
        .replace("__FOREIGN_KEY__", foreign_key)
}

/// Generates billing persistence, pricing, checkout, portal and signed-webhook code.
pub fn scaffold_billing_system(model: &str) -> Result<(), Box<dyn std::error::Error>> {
    if !is_rullst_project() {
        return Err(IoError::new(
            ErrorKind::InvalidInput,
            "make:billing must be run inside a Rullst project",
        )
        .into());
    }

    let model_name = model_to_snake_case(model);
    let foreign_key = format!("{model_name}_id");
    if model_name.is_empty() || !is_valid_rust_identifier(&foreign_key) {
        return Err(IoError::new(
            ErrorKind::InvalidInput,
            "billable model must produce a valid Rust foreign-key identifier",
        )
        .into());
    }

    reject_existing_outputs()?;
    let root_module = project_root_module()?;
    let backend = project_orm_backend();
    let timestamp = chrono::Local::now().format("%Y%m%d%H%M%S");
    let migration_name = format!("m{timestamp}_create_subscriptions_table");
    let migration_path = Path::new("src/migrations").join(format!("{migration_name}.rs"));
    if migration_path.exists() {
        return Err(existing_output_error(&migration_path).into());
    }

    let manifest_path = Path::new("Cargo.toml");
    let manifest = fs::read_to_string(manifest_path)?;
    let updated_manifest = ensure_rullst_features(&manifest, &["orm", "capital"])?;
    let (subscription_model, customer_model) = render_billing_models(&foreign_key, backend);
    let controller = render_billing_controller(&foreign_key, backend);
    let migration = render_billing_migration(&migration_name, &foreign_key, backend);

    fs::create_dir_all("src/models")?;
    fs::create_dir_all("src/pages")?;
    fs::create_dir_all("src/controllers")?;
    fs::create_dir_all("src/migrations")?;
    fs::write(FIXED_OUTPUTS[0], subscription_model)?;
    fs::write(FIXED_OUTPUTS[1], customer_model)?;
    fs::write(FIXED_OUTPUTS[2], include_str!("billing_page.rs.template"))?;
    fs::write(FIXED_OUTPUTS[3], controller)?;
    fs::write(&migration_path, migration)?;
    fs::write(manifest_path, updated_manifest)?;

    register_mod_ast(Path::new("src/models/mod.rs"), "subscription")?;
    register_mod_ast(Path::new("src/models/mod.rs"), "billing_customer")?;
    register_mod_ast(Path::new("src/pages/mod.rs"), "billing")?;
    register_mod_ast(Path::new("src/controllers/mod.rs"), "billing_controller")?;
    for module in ["controllers", "models", "pages"] {
        register_mod_ast(&root_module, module)?;
    }
    regenerate_migrations_mod()?;

    println!(
        "{}",
        format!(
            "💳 Billing scaffold created for {model} with the {} persistence profile.",
            backend_label(backend)
        )
        .green()
        .bold()
    );
    println!("👉 Mount authenticated checkout/portal routes and the exact signed webhook route.");
    println!("👉 BILLING_PROVIDER accepts stripe or lemonsqueezy.");
    println!("👉 Configure BILLING_API_KEY, BILLING_WEBHOOK_SECRET, and BILLING_REDIRECT_URL.");
    println!("👉 Set BILLING_ALLOWED_PLAN_IDS to a comma-separated server-owned allowlist.");
    Ok(())
}

fn reject_existing_outputs() -> Result<(), IoError> {
    let mut collisions = FIXED_OUTPUTS
        .into_iter()
        .filter(|path| Path::new(path).exists())
        .map(str::to_string)
        .collect::<Vec<_>>();
    let migrations = Path::new("src/migrations");
    if migrations.exists() {
        for entry in fs::read_dir(migrations)? {
            let entry = entry?;
            let file_name = entry.file_name();
            if file_name
                .to_str()
                .is_some_and(|name| name.ends_with("_create_subscriptions_table.rs"))
            {
                collisions.push(entry.path().display().to_string());
            }
        }
    }
    if collisions.is_empty() {
        return Ok(());
    }
    collisions.sort();
    Err(IoError::new(
        ErrorKind::AlreadyExists,
        format!(
            "refusing to overwrite existing billing scaffold: {}",
            collisions.join(", ")
        ),
    ))
}

fn project_root_module() -> Result<PathBuf, IoError> {
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

fn existing_output_error(path: &Path) -> IoError {
    IoError::new(
        ErrorKind::AlreadyExists,
        format!("refusing to overwrite {}", path.display()),
    )
}

fn backend_label(backend: ProjectOrmBackend) -> &'static str {
    match backend {
        ProjectOrmBackend::Sqlx => "SQLx",
        ProjectOrmBackend::Turso => "Turso-primary",
    }
}

#[cfg(test)]
mod tests;
