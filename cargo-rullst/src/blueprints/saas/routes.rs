// cargo-rullst/src/blueprints/saas/routes.rs — Main router and application entrypoint for SaaS blueprint.

use crate::blueprints::common;

pub fn get_routes(project_name_safe: &str, hot_reload: bool, orm_pattern: &str) -> Vec<(&'static str, String)> {
    let mut manifest = Vec::new();
    let repo_decl = common::repo_mod_decl(orm_pattern);

    if hot_reload {
        let lib_rs = format!(
            r##"use rullst::{{routes, Router}};

pub mod migrations;
pub mod models;
{repo_decl}pub mod controllers;
pub mod middlewares;
pub mod pages;

#[unsafe(no_mangle)]
pub extern "C" fn rullst_router_init() -> *mut Router {{
    let nexus = rullst::nexus::Nexus::new()
        .with_auth("admin", "password")
        .with_brand("SaaS Admin")
        .register::<models::user::User>()
        .register::<models::subscription::Subscription>()
        .build();

    let router = routes![
        get("/" => controllers::billing_controller::pricing_view),
        get("/pricing" => controllers::billing_controller::pricing_view),
        get("/login" => controllers::auth_controller::login_view),
        post("/login" => controllers::auth_controller::login_submit),
        get("/register" => controllers::auth_controller::register_view),
        post("/register" => controllers::auth_controller::register_submit),
        get("/logout" => controllers::auth_controller::logout),
        get("/billing/checkout" => controllers::billing_controller::checkout_redirect),
        post("/billing/webhook" => controllers::billing_controller::webhook_handler),
    ];

    let router = router.route("/dashboard", rullst::routing::get(controllers::auth_controller::dashboard)
        .layer(rullst::server::from_fn(middlewares::auth_middleware::auth_middleware)))
    .layer(rullst::server::from_fn(rullst::security::csrf_middleware))
    .layer(rullst::server::from_fn(rullst::security::headers_middleware))
    .nest_axum("/nexus", nexus);
    Box::into_raw(Box::new(router))
}}
"##,
            repo_decl = repo_decl
        );
        manifest.push(("src/lib.rs", lib_rs));

        let main_rs = format!(
            r##"pub mod migrations;
pub mod models;
{repo_decl}pub mod controllers;
pub mod middlewares;
pub mod pages;

#[rullst::runtime::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {{
    rullst::artisan!(crate::migrations::get_migrations());

    let is_dev = std::env::var("APP_ENV").unwrap_or_else(|_| "development".to_string()) != "production";
    if is_dev {{
        rullst::runtime::spawn(async {{ let _ = rullst::studio::run_studio(5555).await; }});
        println!("📊 Rullst Studio running on http://127.0.0.1:5555");
    }}
    println!("🚀 SaaS server starting on port 3000...");
    let is_hot = std::env::var("HOT_RELOAD").is_ok();

    let server = if is_hot {{
        let lib_path = if cfg!(target_os = "windows") {{
            format!("target/debug/{{}}", "{project_name_safe}")
        }} else {{
            format!("target/debug/lib{{}}", "{project_name_safe}")
        }};
        rullst::Server::new_hot(&lib_path)
    }} else {{
        let router_ptr = {project_name_safe}::rullst_router_init();
        let router = unsafe {{ *Box::from_raw(router_ptr) }};
        rullst::Server::new(router)
    }};

    server.run(3000).await?;

    Ok(())
}}
"##,
            repo_decl = repo_decl,
            project_name_safe = project_name_safe
        );
        manifest.push(("src/main.rs", main_rs));
    } else {
        let main_rs = format!(
            r##"use rullst::{{routes, Server}};

pub mod migrations;
pub mod models;
{repo_decl}pub mod controllers;
pub mod middlewares;
pub mod pages;

#[rullst::runtime::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {{
    rullst::artisan!(crate::migrations::get_migrations());

    let nexus = rullst::nexus::Nexus::new()
        .with_auth("admin", "password")
        .with_brand("SaaS Admin")
        .register::<models::user::User>()
        .register::<models::subscription::Subscription>()
        .build();

    let router = routes![
        get("/" => controllers::billing_controller::pricing_view),
        get("/pricing" => controllers::billing_controller::pricing_view),
        get("/login" => controllers::auth_controller::login_view),
        post("/login" => controllers::auth_controller::login_submit),
        get("/register" => controllers::auth_controller::register_view),
        post("/register" => controllers::auth_controller::register_submit),
        get("/logout" => controllers::auth_controller::logout),
        get("/billing/checkout" => controllers::billing_controller::checkout_redirect),
        post("/billing/webhook" => controllers::billing_controller::webhook_handler),
    ];

    let router = router.route("/dashboard", rullst::routing::get(controllers::auth_controller::dashboard)
        .layer(rullst::server::from_fn(middlewares::auth_middleware::auth_middleware)))
    .layer(rullst::server::from_fn(rullst::security::csrf_middleware))
    .layer(rullst::server::from_fn(rullst::security::headers_middleware))
    .nest_axum("/nexus", nexus);

    let is_dev = std::env::var("APP_ENV").unwrap_or_else(|_| "development".to_string()) != "production";
    if is_dev {{
        rullst::runtime::spawn(async {{ let _ = rullst::studio::run_studio(5555).await; }});
        println!("📊 Rullst Studio running on port 5555");
    }}
    println!("🚀 SaaS server starting on port 3000...");
    Server::new(router)
        .run(3000)
        .await?;

    Ok(())
}}
"##,
            repo_decl = repo_decl
        );
        manifest.push(("src/main.rs", main_rs));
    }

    manifest
}
