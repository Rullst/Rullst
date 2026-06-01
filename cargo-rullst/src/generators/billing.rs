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
        r##"use rullst::db::schema::{{Schema, Blueprint, Migration}};
use rullst::db::async_trait;

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {{
    fn name(&self) -> &'static str {{
        "{file_stem}"
    }}

    async fn up(&self) -> Result<(), rullst::db::sqlx::Error> {{
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

    async fn down(&self) -> Result<(), rullst::db::sqlx::Error> {{
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
    let model_template = r##"use rullst::db::{Orm, RullstModel, FromRow, sqlx};

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
    let page_template = r##"use rullst::html;
use rullst::server::Html;

pub fn pricing_page() -> Html<String> {
    Html(html! {
        <!DOCTYPE html>
        <html lang="en" class="dark">
        <head>
            <meta charset="UTF-8" />
            <meta name="viewport" content="width=device-width, initial-scale=1.0" />
            <title>Select a Plan - Rullst Billing</title>
            <link href="https://fonts.googleapis.com/css2?family=Outfit:wght@300;400;500;600;700&display=swap" rel="stylesheet" />
            <style>
                * { box-sizing: border-box; margin: 0; padding: 0; font-family: 'Outfit', sans-serif; }
                body { background: #0b0f19; color: #f3f4f6; min-height: 100vh; display: flex; flex-direction: column; align-items: center; justify-content: center; overflow-x: hidden; position: relative; }
                .glow-bg { position: absolute; width: 600px; height: 600px; background: radial-gradient(circle, rgba(99, 102, 241, 0.15) 0%, rgba(139, 92, 246, 0.05) 50%, transparent 100%); top: -10%; left: -10%; z-index: -1; }
                .glow-bg-right { position: absolute; width: 600px; height: 600px; background: radial-gradient(circle, rgba(236, 72, 153, 0.1) 0%, rgba(99, 102, 241, 0.05) 50%, transparent 100%); bottom: -10%; right: -10%; z-index: -1; }
                .container { max-width: 1200px; margin: 0 auto; padding: 4rem 2rem; text-align: center; z-index: 1; }
                .header { margin-bottom: 3.5rem; }
                .badge { background: linear-gradient(135deg, #6366f1 0%, #a855f7 100%); color: white; padding: 0.35rem 1rem; border-radius: 9999px; font-size: 0.85rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; display: inline-block; margin-bottom: 1rem; }
                h1 { font-size: 3rem; font-weight: 700; background: linear-gradient(to right, #ffffff, #9ca3af); -webkit-background-clip: text; -webkit-text-fill-color: transparent; margin-bottom: 1rem; }
                .subtitle { color: #9ca3af; font-size: 1.15rem; max-width: 600px; margin: 0 auto; }
                .pricing-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(320px, 1fr)); gap: 2rem; max-width: 1000px; margin: 0 auto; }
                .pricing-card { background: rgba(17, 24, 39, 0.7); backdrop-filter: blur(16px); -webkit-backdrop-filter: blur(16px); border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 24px; padding: 3rem 2rem; text-align: left; display: flex; flex-direction: column; transition: all 0.4s cubic-bezier(0.4, 0, 0.2, 1); position: relative; }
                .pricing-card:hover { transform: translateY(-8px); border-color: rgba(99, 102, 241, 0.4); box-shadow: 0 20px 40px rgba(0, 0, 0, 0.3); }
                .pricing-card.premium { border: 2px solid #6366f1; }
                .pricing-card.premium::after { content: 'Best Value'; position: absolute; top: -14px; right: 24px; background: #6366f1; color: white; font-size: 0.75rem; font-weight: 700; padding: 0.25rem 0.75rem; border-radius: 9999px; text-transform: uppercase; }
                .plan-name { font-size: 1.5rem; font-weight: 600; color: #ffffff; margin-bottom: 0.5rem; }
                .plan-desc { color: #9ca3af; font-size: 0.95rem; margin-bottom: 2rem; min-height: 40px; }
                .price-container { display: flex; align-items: baseline; margin-bottom: 2.5rem; }
                .currency { font-size: 1.75rem; font-weight: 600; color: #ffffff; }
                .price { font-size: 3.5rem; font-weight: 700; color: #ffffff; letter-spacing: -0.02em; }
                .period { color: #9ca3af; font-size: 1rem; margin-left: 0.5rem; }
                .features-list { list-style: none; margin-bottom: 3rem; flex-grow: 1; }
                .features-list li { display: flex; align-items: center; color: #d1d5db; font-size: 0.95rem; margin-bottom: 1rem; }
                .features-list svg { width: 20px; height: 20px; margin-right: 0.75rem; color: #10b981; flex-shrink: 0; }
                .btn-checkout { display: block; width: 100%; text-align: center; padding: 1rem; border-radius: 12px; font-weight: 600; text-decoration: none; font-size: 1rem; transition: all 0.3s; cursor: pointer; border: none; }
                .btn-checkout.primary { background: linear-gradient(135deg, #6366f1 0%, #8b5cf6 100%); color: white; box-shadow: 0 4px 14px rgba(99, 102, 241, 0.4); }
                .btn-checkout.primary:hover { background: linear-gradient(135deg, #4f46e5 0%, #7c3aed 100%); box-shadow: 0 6px 20px rgba(99, 102, 241, 0.6); }
                .btn-checkout.secondary { background: rgba(255, 255, 255, 0.08); color: white; border: 1px solid rgba(255, 255, 255, 0.1); }
                .btn-checkout.secondary:hover { background: rgba(255, 255, 255, 0.15); border-color: rgba(255, 255, 255, 0.25); }
            </style>
        </head>
        <body>
            <div class="glow-bg"></div>
            <div class="glow-bg-right"></div>
            <div class="container">
                <div class="header">
                    <span class="badge">Rullst Capital</span>
                    <h1>Simple, Transparent Pricing</h1>
                    <p class="subtitle">Choose the perfect plan to boost your application with next-gen fullstack performance.</p>
                </div>
                <div class="pricing-grid">
                    <!-- Starter Plan -->
                    <div class="pricing-card">
                        <h2 class="plan-name">Starter</h2>
                        <p class="plan-desc">For hobbyists and early-stage startup prototypes.</p>
                        <div class="price-container">
                            <span class="currency">$</span>
                            <span class="price">9</span>
                            <span class="period">/mo</span>
                        </div>
                        <ul class="features-list">
                            <li>
                                <svg fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7"></path></svg>
                                Up to 5 Projects
                            </li>
                            <li>
                                <svg fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7"></path></svg>
                                Standard SQLite Database
                            </li>
                            <li>
                                <svg fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7"></path></svg>
                                Email Support
                            </li>
                        </ul>
                        <a href="/billing/checkout?plan=price_starter" class="btn-checkout secondary">Get Started</a>
                    </div>
                    
                    <!-- Pro Plan -->
                    <div class="pricing-card premium">
                        <h2 class="plan-name">Pro</h2>
                        <p class="plan-desc">For growing apps needing production scaling and support.</p>
                        <div class="price-container">
                            <span class="currency">$</span>
                            <span class="price">29</span>
                            <span class="period">/mo</span>
                        </div>
                        <ul class="features-list">
                            <li>
                                <svg fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7"></path></svg>
                                Unlimited Projects
                            </li>
                            <li>
                                <svg fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7"></path></svg>
                                PostgreSQL & SQLite Support
                            </li>
                            <li>
                                <svg fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7"></path></svg>
                                Adaptive WAF & Bot Management
                            </li>
                            <li>
                                <svg fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7"></path></svg>
                                Priority Support (Sub-1 hour)
                            </li>
                        </ul>
                        <a href="/billing/checkout?plan=price_pro" class="btn-checkout primary">Go Pro</a>
                    </div>
                </div>
            </div>
        </body>
        </html>
    })
}
"##;
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
