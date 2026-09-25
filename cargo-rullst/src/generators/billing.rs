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
const SQLX_PERSIST: &str = include_str!("billing_persist_sqlx.rs.template");
const TURSO_PERSIST: &str = include_str!("billing_persist_turso.rs.template");

const FIXED_OUTPUTS: [&str; 15] = [
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
    "src/controllers/billing_paddle.rs",
    "src/controllers/billing_paddle_gateway.rs",
    "src/controllers/billing_paddle_events.rs",
    "src/controllers/billing_paddle_report.rs",
    "src/controllers/billing_paddle_store.rs",
];

pub(crate) fn render_billing_controller(foreign_key: &str, backend: ProjectOrmBackend) -> String {
    let owner_id_type = match backend {
        ProjectOrmBackend::Sqlx => "i32",
        ProjectOrmBackend::Turso => "i64",
    };
    BILLING_CONTROLLER_TEMPLATE
        .replace(
            "__PERSIST_BILLING_UPDATE__",
            match backend {
                ProjectOrmBackend::Sqlx => SQLX_PERSIST,
                ProjectOrmBackend::Turso => TURSO_PERSIST,
            },
        )
        .replace("__FOREIGN_KEY__", foreign_key)
        .replace("__OWNER_ID_TYPE__", owner_id_type)
}

pub(crate) fn live_billing_files(
    foreign_key: &str,
    backend: ProjectOrmBackend,
) -> Vec<(&'static str, String)> {
    let store = match backend {
        ProjectOrmBackend::Sqlx => include_str!("billing_store_sqlx.rs.template"),
        ProjectOrmBackend::Turso => include_str!("billing_store_turso.rs.template"),
    };
    let owner_type = if backend == ProjectOrmBackend::Sqlx {
        "i32"
    } else {
        "i64"
    };
    // Emit a separate module for each provider's concrete state type while
    // maintaining the transaction implementation in one backend template.
    let store = store
        .replace("__FOREIGN_KEY__", foreign_key)
        .replace("__OWNER_ID_TYPE__", owner_type)
        .replace(
            "__OWNER_CAST__",
            "i32::try_from(next.owner).map_err(|_| UNAVAILABLE)?",
        );
    let render_paddle = |template: &str| {
        template
            .replace(
                "__OWNER_TO_I64__",
                if backend == ProjectOrmBackend::Sqlx {
                    "i64::from(identity.owner_id)"
                } else {
                    "identity.owner_id"
                },
            )
            .replace(
                "__SUBJECT_KIND__",
                foreign_key.strip_suffix("_id").unwrap_or(foreign_key),
            )
    };
    vec![
        (
            "src/controllers/billing_paddle.rs",
            render_paddle(include_str!("billing_paddle.rs.template")),
        ),
        (
            "src/controllers/billing_paddle_gateway.rs",
            include_str!("billing_paddle_gateway.rs.template").into(),
        ),
        (
            "src/controllers/billing_paddle_events.rs",
            include_str!("billing_paddle_events.rs.template").into(),
        ),
        (
            "src/controllers/billing_paddle_report.rs",
            render_paddle(include_str!("billing_paddle_report.rs.template")),
        ),
        ("src/controllers/billing_paddle_store.rs", store.clone()),
        (
            "src/controllers/billing_report.rs",
            include_str!("billing_report.rs.template")
                .replace(
                    "__SUBJECT_KIND__",
                    foreign_key.strip_suffix("_id").unwrap_or(foreign_key),
                )
                .replace(
                    "__OWNER_TO_I64__",
                    if backend == ProjectOrmBackend::Sqlx {
                        "i64::from(identity.owner_id)"
                    } else {
                        "identity.owner_id"
                    },
                ),
        ),
        (
            "src/controllers/billing_gateway.rs",
            include_str!("billing_gateway.rs.template").into(),
        ),
        (
            "src/controllers/billing_live.rs",
            include_str!("billing_live.rs.template").replace(
                "__OWNER_TO_I64__",
                if backend == ProjectOrmBackend::Sqlx {
                    "i64::from(identity.owner_id)"
                } else {
                    "identity.owner_id"
                },
            ),
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
    for (path, contents) in live_billing_files(&foreign_key, backend) {
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
        format!(
            "💳 Billing scaffold created for {model} with the {} persistence profile.",
            backend_label(backend)
        )
        .green()
        .bold()
    );
    println!("👉 Mount authenticated checkout/portal routes and the exact signed webhook route.");
    println!("👉 BILLING_PROVIDER accepts stripe, paddle or lemonsqueezy.");
    println!(
        "👉 Stripe supports durable customer/checkout ownership and atomic webhook reconciliation."
    );
    println!(
        "👉 Configure BILLING_ACCOUNT_ID, HTTPS redirect, credentials and plans; follow BILLING.md. Paddle requires an explicit environment and approved payment page; Lemon Squeezy remains a fixture."
    );
    println!(
        "👉 Paddle persists single-dispatch attempts; uncertain outcomes require read-only recovery. See BILLING.md."
    );
    println!("👉 Lemon Squeezy also requires BILLING_STORE_ID and numeric variant IDs.");
    println!("👉 Set BILLING_ALLOWED_PLAN_IDS to a comma-separated server-owned allowlist.");
    println!(
        "👉 Review CSP form-action on the pricing page: Stripe requires https://checkout.stripe.com; other providers require their exact reviewed checkout origin."
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

fn backend_label(backend: ProjectOrmBackend) -> &'static str {
    match backend {
        ProjectOrmBackend::Sqlx => "SQLx",
        ProjectOrmBackend::Turso => "Turso-primary",
    }
}

#[cfg(test)]
mod tests;
