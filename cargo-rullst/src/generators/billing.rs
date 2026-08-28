// src/generators/billing.rs — Billing generator.

use crate::generators::is_rullst_project;
use crate::generators::migration::regenerate_migrations_mod;
use crate::generators::model_to_snake_case;
use colored::*;
use std::fs;
use std::path::Path;

const BILLING_CONTROLLER_TEMPLATE: &str = include_str!("billing_controller.rs.template");

pub(crate) fn render_billing_controller(foreign_key: &str) -> String {
    BILLING_CONTROLLER_TEMPLATE.replace("__FOREIGN_KEY__", foreign_key)
}

pub fn scaffold_billing_system(model: &str) -> Result<(), Box<dyn std::error::Error>> {
    if !is_rullst_project() {
        println!(
            "{}{}",
            "❌ Error: This command must be executed in the root of a valid Rullst project."
                .red()
                .bold(),
            "\nMake sure the current folder contains a 'Cargo.toml' file with a 'rullst' dependency."
                .yellow()
        );
        std::process::exit(1);
    }

    println!(
        "{}",
        format!("💳 Starting scaffolding of Rullst billing system (Stripe & LemonSqueezy) for model '{}'...", model)
            .cyan()
            .bold()
    );

    let model_name = model_to_snake_case(model);
    if model_name.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "billable model name cannot be empty",
        )
        .into());
    }
    let foreign_key = format!("{}_id", model_name);

    // 1. Create Subscriptions Migration
    let migrations_dir = Path::new("src/migrations");
    fs::create_dir_all(migrations_dir)?;
    let now = chrono::Local::now();
    let timestamp = now.format("%Y%m%d%H%M%S").to_string();
    let file_stem = format!("m{}_create_subscriptions_table", timestamp);
    let migration_path = migrations_dir.join(format!("{}.rs", file_stem));

    let migration_template = format!(
        r##"use rullst::db::{{Orm, sqlx}};
use rullst::db::schema::{{Schema, Migration}};
use rullst::db::async_trait;

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {{
    fn name(&self) -> &'static str {{
        "{file_stem}"
    }}

    async fn up(&self) -> Result<(), rullst_orm::error::RullstError> {{
        Schema::create("billing_customers", |table| {{
            table.id();
            table.integer("{foreign_key}").not_null();
            table.string("email").not_null();
            table.string("customer_id").nullable();
            table.timestamps();
        }}).await?;
        Schema::create("subscriptions", |table| {{
            table.id();
            table.integer("{foreign_key}").not_null();
            table.string("customer_id").not_null();
            table.string("subscription_id").not_null();
            table.string("plan_id").not_null();
            table.string("status").not_null();
            table.integer("ends_at").nullable();
            table.timestamps();
        }}).await?;
        let pool = Orm::pool()?;
        sqlx::query(
            "CREATE UNIQUE INDEX billing_customers_email_unique ON billing_customers(email)",
        )
        .execute(pool)
        .await?;
        sqlx::query(
            "CREATE UNIQUE INDEX subscriptions_subscription_id_unique ON subscriptions(subscription_id)",
        )
        .execute(pool)
        .await?;
        Ok(())
    }}

    async fn down(&self) -> Result<(), rullst_orm::error::RullstError> {{
        Schema::drop_if_exists("subscriptions").await?;
        Schema::drop_if_exists("billing_customers").await
    }}
}}
"##,
        file_stem = file_stem,
        foreign_key = foreign_key
    );
    fs::write(&migration_path, migration_template)?;
    println!(
        "{}",
        "  ✨ Created 'subscriptions' table migration.".green()
    );

    regenerate_migrations_mod()?;

    // 2. Create Subscription Model
    let models_dir = Path::new("src/models");
    fs::create_dir_all(models_dir)?;
    let model_path = models_dir.join("subscription.rs");
    let model_template = format!(
        r##"use rullst::db::{{Orm, FromRow}};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "subscriptions")]
pub struct Subscription {{
    pub id: i32,
    pub {foreign_key}: i32,
    pub customer_id: String,
    pub subscription_id: String,
    pub plan_id: String,
    pub status: String,
    pub ends_at: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}}
"##,
        foreign_key = foreign_key
    );
    fs::write(&model_path, model_template)?;
    println!("{}", "  ✨ Created 'Subscription' model.".green());

    let customer_model_path = models_dir.join("billing_customer.rs");
    let customer_model_template = format!(
        r##"use rullst::db::{{Orm, FromRow}};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "billing_customers")]
pub struct BillingCustomer {{
    pub id: i32,
    pub {foreign_key}: i32,
    pub email: String,
    pub customer_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}}
"##
    );
    fs::write(&customer_model_path, customer_model_template)?;
    println!("{}", "  ✨ Created 'BillingCustomer' model.".green());

    let mod_models_path = models_dir.join("mod.rs");
    if !mod_models_path.exists() {
        fs::write(&mod_models_path, "")?;
    }
    let mut mod_models_content = fs::read_to_string(&mod_models_path)?;
    if !mod_models_content.contains("pub mod subscription;") {
        mod_models_content.push_str("pub mod subscription;\n");
    }
    if !mod_models_content.contains("pub mod billing_customer;") {
        mod_models_content.push_str("pub mod billing_customer;\n");
    }
    fs::write(&mod_models_path, mod_models_content)?;

    // 3. Create Pricing View Page
    let pages_dir = Path::new("src/pages");
    fs::create_dir_all(pages_dir)?;
    let page_path = pages_dir.join("billing.rs");
    let page_template = include_str!("billing_page.rs.template");
    fs::write(&page_path, page_template)?;
    println!(
        "{}",
        "  ✨ Created HTML views in 'src/pages/billing.rs'.".green()
    );

    let mod_pages_path = pages_dir.join("mod.rs");
    if !mod_pages_path.exists() {
        fs::write(&mod_pages_path, "")?;
    }
    let mut mod_pages_content = fs::read_to_string(&mod_pages_path)?;
    if !mod_pages_content.contains("pub mod billing;") {
        mod_pages_content.push_str("pub mod billing;\n");
        fs::write(&mod_pages_path, mod_pages_content)?;
    }

    // 4. Create Billing Controller
    let controllers_dir = Path::new("src/controllers");
    fs::create_dir_all(controllers_dir)?;
    let controller_path = controllers_dir.join("billing_controller.rs");
    let controller_template = render_billing_controller(&foreign_key);
    fs::write(&controller_path, controller_template)?;
    println!(
        "{}",
        "  ✨ Created 'src/controllers/billing_controller.rs' controller.".green()
    );

    let mod_controllers_path = controllers_dir.join("mod.rs");
    if !mod_controllers_path.exists() {
        fs::write(&mod_controllers_path, "")?;
    }
    let mut mod_controllers_content = fs::read_to_string(&mod_controllers_path)?;
    if !mod_controllers_content.contains("pub mod billing_controller;") {
        mod_controllers_content.push_str("pub mod billing_controller;\n");
        fs::write(&mod_controllers_path, mod_controllers_content)?;
    }

    // 5. Register modules in src/main.rs if needed
    let main_path = Path::new("src/main.rs");
    if main_path.exists() {
        let mut main_content = fs::read_to_string(main_path)?;
        for module in &["controllers", "models", "pages"] {
            let declaration = format!("pub mod {};", module);
            let alt_declaration = format!("mod {};", module);
            if !main_content.contains(&declaration) && !main_content.contains(&alt_declaration) {
                main_content = format!("pub mod {};\n{}", module, main_content);
            }
        }
        fs::write(main_path, main_content)?;
    }

    println!(
        "\n{}",
        "🎉 Rullst Capital Billing Scaffolding Completed Successfully!"
            .green()
            .bold()
    );
    println!(
        "{}",
        "To mount the billing panel and webhooks, register these routes in your main router:"
            .white()
    );
    println!("{}", "  👉 .route(\"/pricing\", rullst::server::get(controllers::billing_controller::pricing_view))".cyan());
    println!("{}", "  👉 .route(\"/billing/checkout\", rullst::server::get(controllers::billing_controller::checkout_redirect))".cyan());
    println!("{}", "  👉 .route(\"/billing/portal\", rullst::server::get(controllers::billing_controller::portal_redirect))".cyan());
    println!("{}", "  👉 .route(\"/billing/webhook\", rullst::server::post(controllers::billing_controller::webhook_handler).route_layer(rullst::server::from_fn(controllers::billing_controller::verify_billing_webhook)))".cyan());
    println!(
        "\n{}",
        "Configure your gateway credentials in environment variables or your .env file:".white()
    );
    println!("{}", "  💰 BILLING_PROVIDER=stripe".yellow());
    println!("{}", "  💰 BILLING_API_KEY=sk_test_...".yellow());
    println!("{}", "  💰 BILLING_WEBHOOK_SECRET=whsec_...".yellow());
    println!(
        "{}",
        "  🔐 Protect checkout/portal with authentication middleware that inserts BillingIdentity (owner_id + email)."
            .yellow()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_billing_binds_signed_events_to_authenticated_owners() {
        let source = render_billing_controller("workspace_id");
        syn::parse_file(&source).expect("billing controller must parse");
        let canonical_environment = source
            .find("std::env::var(\"RULLST_ENV\")")
            .expect("canonical environment lookup");
        let legacy_environment = source
            .find("std::env::var(\"APP_ENV\")")
            .expect("legacy environment fallback");
        assert!(canonical_environment < legacy_environment);
        assert!(source.contains("workspace_id: identity.owner_id"));
        assert!(source.contains("Extension(identity): Extension<BillingIdentity>"));
        assert!(source.contains("rullst-capital's mandatory signature/replay middleware"));
        assert!(source.contains("verify_billing_webhook"));
        assert!(source.contains("initialize_billing_provider"));
        assert!(source.contains("strong_webhook_secret"));
        assert!(!source.contains(".bind(1)"));
        assert!(!source.contains("user@example.com"));
        assert!(!source.contains("mock_secret"));
        assert!(!source.contains(".unwrap("));
        assert!(!source.contains(".expect("));
        assert!(!source.contains("panic!("));
    }
}
