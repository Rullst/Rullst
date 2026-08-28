// src/generators/billing.rs — Billing generator.

use crate::generators::is_rullst_project;
use crate::generators::migration::regenerate_migrations_mod;
use colored::*;
use std::fs;
use std::path::Path;

pub fn scaffold_billing_system() -> Result<(), Box<dyn std::error::Error>> {
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
        "💳 Starting scaffolding of Rullst billing system (Stripe & LemonSqueezy)..."
            .cyan()
            .bold()
    );

    // 1. Create Subscriptions Migration
    let migrations_dir = Path::new("src/migrations");
    fs::create_dir_all(migrations_dir)?;
    let now = chrono::Local::now();
    let timestamp = now.format("%Y%m%d%H%M%S").to_string();
    let file_stem = format!("m{}_create_subscriptions_table", timestamp);
    let migration_path = migrations_dir.join(format!("{}.rs", file_stem));

    let migration_template = format!(
        r##"use rullst::db::schema::{{Schema, Migration}};
use rullst::db::async_trait;

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {{
    fn name(&self) -> &'static str {{
        "{file_stem}"
    }}

    async fn up(&self) -> Result<(), rullst_orm::error::RullstError> {{
        Schema::create("subscriptions", |table| {{
            table.id();
            table.integer("user_id").not_null();
            table.string("customer_id").not_null();
            table.string("subscription_id").unique().not_null();
            table.string("plan_id").not_null();
            table.string("status").not_null();
            table.integer("ends_at").nullable();
            table.timestamps();
        }}).await
    }}

    async fn down(&self) -> Result<(), rullst_orm::error::RullstError> {{
        Schema::drop_if_exists("subscriptions").await
    }}
}}
"##,
        file_stem = file_stem
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
    let model_template = r##"use rullst::db::{Orm, FromRow};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "subscriptions")]
pub struct Subscription {
    pub id: i32,
    pub user_id: i32,
    pub customer_id: String,
    pub subscription_id: String,
    pub plan_id: String,
    pub status: String,
    pub ends_at: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}
"##;
    fs::write(&model_path, model_template)?;
    println!("{}", "  ✨ Created 'Subscription' model.".green());

    let mod_models_path = models_dir.join("mod.rs");
    if !mod_models_path.exists() {
        fs::write(&mod_models_path, "")?;
    }
    let mut mod_models_content = fs::read_to_string(&mod_models_path)?;
    if !mod_models_content.contains("pub mod subscription;") {
        mod_models_content.push_str("pub mod subscription;\n");
        fs::write(&mod_models_path, mod_models_content)?;
    }

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
    let controller_template = r##"use rullst::server::{
    Query, State,
    Html, IntoResponse, Redirect, Response,
    HeaderMap, StatusCode,
    body::Bytes,
};
use serde::Deserialize;
use std::collections::HashMap;
use rullst::capital::{BillingProvider, StripeProvider, LemonSqueezyProvider};
use rullst::db::sqlx::{self, Row};
use crate::pages::billing;

#[derive(Deserialize)]
pub struct CheckoutQuery {
    pub plan: String,
}

/// Serves the premium pricing page.
pub async fn pricing_view() -> impl IntoResponse {
    billing::pricing_page()
}

/// Initiates a checkout redirect.
pub async fn checkout_redirect(Query(query): Query<CheckoutQuery>) -> impl IntoResponse {
    // Resolve Billing Provider using environment keys
    let provider_name = std::env::var("BILLING_PROVIDER").unwrap_or_else(|_| "stripe".to_string());
    let api_key = std::env::var("BILLING_API_KEY").unwrap_or_else(|_| "mock_key".to_string());
    let webhook_secret = std::env::var("BILLING_WEBHOOK_SECRET").unwrap_or_else(|_| "mock_secret".to_string());

    let redirect_url = std::env::var("BILLING_REDIRECT_URL").unwrap_or_else(|_| "http://localhost:3000/dashboard".to_string());

    let url_result = match provider_name.to_lowercase().as_str() {
        "lemonsqueezy" => {
            let provider = LemonSqueezyProvider::new(api_key, webhook_secret);
            provider.create_checkout_session("user@example.com", &query.plan, &redirect_url).await
        }
        _ => {
            let provider = StripeProvider::new(api_key, webhook_secret);
            provider.create_checkout_session("user@example.com", &query.plan, &redirect_url).await
        }
    };

    match url_result {
        Ok(url) => Redirect::temporary(&url).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create checkout session: {}", e)).into_response(),
    }
}

/// Handles incoming webhook events from the selected provider.
pub async fn webhook_handler(headers: HeaderMap, body: rullst::server::Bytes) -> impl IntoResponse {
    let provider_name = std::env::var("BILLING_PROVIDER").unwrap_or_else(|_| "stripe".to_string());
    let api_key = std::env::var("BILLING_API_KEY").unwrap_or_else(|_| "mock_key".to_string());
    let webhook_secret = std::env::var("BILLING_WEBHOOK_SECRET").unwrap_or_else(|_| "mock_secret".to_string());

    let mut headers_map = HashMap::new();
    for (k, v) in headers.iter() {
        if let Ok(val_str) = v.to_str() {
            headers_map.insert(k.as_str().to_string(), val_str.to_string());
        }
    }

    let event_result = match provider_name.to_lowercase().as_str() {
        "lemonsqueezy" => {
            let provider = LemonSqueezyProvider::new(api_key, webhook_secret);
            provider.handle_webhook(&body, &headers_map)
        }
        _ => {
            let provider = StripeProvider::new(api_key, webhook_secret);
            provider.handle_webhook(&body, &headers_map)
        }
    };

    let event = match event_result {
        Ok(evt) => evt,
        Err(e) => {
            eprintln!("❌ Webhook verification/parsing error: {}", e);
            return (StatusCode::BAD_REQUEST, "Invalid webhook signature or payload").into_response();
        }
    };

    println!("🔔 Received Webhook for Subscription {} [{}] -> Status: {:?}", event.subscription_id, event.plan_id, event.status);

    let pool = rullst::db::Orm::pool();
    
    let existing = rullst::db::sqlx::query("SELECT id FROM subscriptions WHERE subscription_id = ?1")
        .bind(&event.subscription_id)
        .fetch_optional(pool)
        .await;

    match existing {
        Ok(Some(row)) => {
            let id: i32 = row.get("id");
            let update_res = rullst::db::sqlx::query("UPDATE subscriptions SET status = ?1, plan_id = ?2, ends_at = ?3, updated_at = datetime('now') WHERE id = ?4")
                .bind(event.status.as_str())
                .bind(&event.plan_id)
                .bind(event.ends_at)
                .bind(id)
                .execute(pool)
                .await;
            if let Err(err) = update_res {
                eprintln!("❌ Failed to update subscription: {}", err);
                return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
            }
        }
        Ok(None) => {
            let insert_res = rullst::db::sqlx::query("INSERT INTO subscriptions (user_id, customer_id, subscription_id, plan_id, status, ends_at, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'), datetime('now'))")
                .bind(1)
                .bind(&event.customer_id)
                .bind(&event.subscription_id)
                .bind(&event.plan_id)
                .bind(event.status.as_str())
                .bind(event.ends_at)
                .execute(pool)
                .await;
            if let Err(err) = insert_res {
                eprintln!("❌ Failed to insert subscription: {}", err);
                return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
            }
        }
        Err(err) => {
            eprintln!("❌ Database query failed: {}", err);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
        }
    }

    (StatusCode::OK, "Webhook processed successfully").into_response()
}
"##;
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
    println!("{}", "  👉 .route(\"/billing/webhook\", rullst::server::post(controllers::billing_controller::webhook_handler))".cyan());
    println!(
        "\n{}",
        "Configure your gateway credentials in environment variables or your .env file:".white()
    );
    println!("{}", "  💰 BILLING_PROVIDER=stripe".yellow());
    println!("{}", "  💰 BILLING_API_KEY=sk_test_...".yellow());
    println!("{}", "  💰 BILLING_WEBHOOK_SECRET=whsec_...".yellow());

    Ok(())
}
