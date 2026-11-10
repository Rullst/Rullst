//! Fail-closed billing scaffold for projects on the SQLx database layer
//! (SQLite, PostgreSQL, MySQL or MariaDB). Turso-primary projects are refused
//! before anything is written.

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
const SUBSCRIPTION_MODEL: &str = include_str!("billing_subscription_sqlx.rs.template");
const CUSTOMER_MODEL: &str = include_str!("billing_customer_sqlx.rs.template");
const MIGRATION: &str = include_str!("billing_migration_sqlx.rs.template");
const PERSIST: &str = include_str!("billing_persist_sqlx.rs.template");
const TIMESTAMP: &str = include_str!("billing_timestamp_sqlx.rs.template");
const STORE: &str = include_str!("billing_store_sqlx.rs.template");
/// SQLx billing tables key their owner with a 32-bit integer.
const OWNER_ID_TYPE: &str = "i32";
const OWNER_TO_I64: &str = "i64::from(identity.owner_id)";

/// Why `make:billing` refuses a Turso-primary project (since 13.0.0-alpha.5).
pub(crate) const TURSO_REFUSAL: &str = "make:billing needs the SQLx database layer (SQLite, PostgreSQL, MySQL or MariaDB): its store uses SQLx transactions and query builders. Turso is supported only as the primary database of the Blank and API starters, so billing cannot be generated in this Turso-primary project; nothing was written";

const FIXED_OUTPUTS: [&str; 10] = [
    "src/models/subscription.rs",
    "src/models/billing_customer.rs",
    "src/pages/billing.rs",
    "src/controllers/billing_controller.rs",
    "src/controllers/billing_live.rs",
    "src/controllers/billing_store.rs",
    "src/controllers/billing_events.rs",
    "BILLING.md",
    "src/controllers/billing_gateway.rs",
    "src/controllers/billing_report.rs",
];

pub(crate) fn render_billing_controller(foreign_key: &str) -> String {
    BILLING_CONTROLLER_TEMPLATE
        .replace("__PERSIST_BILLING_UPDATE__", PERSIST)
        // `timestamps()` columns hold `chrono` instants on SQLx pools.
        .replace("__UTC_TIMESTAMP_FN__", TIMESTAMP)
        // New records also start an empty dirty-tracking snapshot.
        .replace(
            "__UPDATED_AT__",
            "updated_at: created_at,\n                snapshot: rullst::db::RowSnapshot::default()",
        )
        .replace("__FOREIGN_KEY__", foreign_key)
        .replace("__OWNER_ID_TYPE__", OWNER_ID_TYPE)
}

pub(crate) fn live_billing_files(foreign_key: &str) -> Vec<(&'static str, String)> {
    let store = STORE
        .replace("__FOREIGN_KEY__", foreign_key)
        .replace("__OWNER_ID_TYPE__", OWNER_ID_TYPE)
        .replace(
            "__OWNER_CAST__",
            "i32::try_from(next.owner).map_err(|_| UNAVAILABLE)?",
        );
    vec![
        (
            "src/controllers/billing_report.rs",
            include_str!("billing_report.rs.template")
                .replace(
                    "__SUBJECT_KIND__",
                    foreign_key.strip_suffix("_id").unwrap_or(foreign_key),
                )
                .replace("__OWNER_TO_I64__", OWNER_TO_I64),
        ),
        (
            "src/controllers/billing_gateway.rs",
            include_str!("billing_gateway.rs.template").into(),
        ),
        (
            "src/controllers/billing_live.rs",
            include_str!("billing_live.rs.template").replace("__OWNER_TO_I64__", OWNER_TO_I64),
        ),
        (
            "src/controllers/billing_events.rs",
            include_str!("billing_events.rs.template").into(),
        ),
        ("src/controllers/billing_store.rs", store),
        (
            "BILLING.md",
            include_str!("billing_readme.md.template").into(),
        ),
    ]
}

pub(crate) fn render_billing_models(foreign_key: &str) -> (String, String) {
    (
        SUBSCRIPTION_MODEL.replace("__FOREIGN_KEY__", foreign_key),
        CUSTOMER_MODEL.replace("__FOREIGN_KEY__", foreign_key),
    )
}

pub(crate) fn render_billing_migration(migration_name: &str, foreign_key: &str) -> String {
    MIGRATION
        .replace("__MIGRATION_NAME__", migration_name)
        .replace("__FOREIGN_KEY__", foreign_key)
}

/// Refuses a Turso-primary project with [`TURSO_REFUSAL`].
fn require_sqlx(backend: ProjectOrmBackend) -> Result<(), IoError> {
    match backend {
        ProjectOrmBackend::Sqlx => Ok(()),
        ProjectOrmBackend::Turso => Err(IoError::new(ErrorKind::Unsupported, TURSO_REFUSAL)),
    }
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

    require_sqlx(project_orm_backend())?;
    reject_existing_outputs()?;
    let root_module = project_root_module()?;
    let timestamp = chrono::Local::now().format("%Y%m%d%H%M%S");
    let migration_name = format!("m{timestamp}_create_subscriptions_table");
    let migration_path = Path::new("src/migrations").join(format!("{migration_name}.rs"));
    if migration_path.exists() {
        return Err(existing_output_error(&migration_path).into());
    }

    let manifest_path = Path::new("Cargo.toml");
    let manifest = fs::read_to_string(manifest_path)?;
    let updated_manifest = ensure_rullst_features(&manifest, &["orm", "capital"])?;
    let (subscription_model, customer_model) = render_billing_models(&foreign_key);
    let controller = render_billing_controller(&foreign_key);
    let migration = render_billing_migration(&migration_name, &foreign_key);

    fs::create_dir_all("src/models")?;
    fs::create_dir_all("src/pages")?;
    fs::create_dir_all("src/controllers")?;
    fs::create_dir_all("src/migrations")?;
    fs::write(FIXED_OUTPUTS[0], subscription_model)?;
    fs::write(FIXED_OUTPUTS[1], customer_model)?;
    fs::write(FIXED_OUTPUTS[2], include_str!("billing_page.rs.template"))?;
    fs::write(FIXED_OUTPUTS[3], controller)?;
    for (path, contents) in live_billing_files(&foreign_key) {
        fs::write(path, contents)?;
    }
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
        format!("💳 Billing scaffold created for {model} with the SQLx persistence profile.")
            .green()
            .bold()
    );
    println!("👉 Mount authenticated checkout/portal routes and the exact signed webhook route.");
    println!(
        "👉 BILLING_PROVIDER accepts stripe; implement the Capital provider traits for other gateways."
    );
    println!(
        "👉 Stripe supports durable customer/checkout ownership and atomic webhook reconciliation."
    );
    println!(
        "👉 Configure BILLING_ACCOUNT_ID, HTTPS redirect, credentials and plans; follow BILLING.md."
    );
    println!("👉 Set BILLING_ALLOWED_PLAN_IDS to a comma-separated server-owned allowlist.");
    println!(
        "👉 Review CSP form-action on the pricing page: Stripe requires https://checkout.stripe.com."
    );
    println!(
        "👉 Validate returned checkout URLs on the server and test the POST/303 handoff in a real browser. Existing security policy is not rewritten."
    );
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

#[cfg(test)]
mod tests;
