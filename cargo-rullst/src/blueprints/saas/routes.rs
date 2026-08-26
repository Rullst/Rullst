// cargo-rullst/src/blueprints/saas/routes.rs — Main router and application entrypoint for SaaS blueprint.

use crate::blueprints::common;

pub fn get_routes(
    project_name_safe: &str,
    hot_reload: bool,
    orm_pattern: &str,
) -> Vec<(&'static str, String)> {
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

pub fn router() -> Result<Router, Box<dyn std::error::Error>> {{
    controllers::billing_controller::initialize_billing_provider()?;
    let nexus_auth = rullst::nexus::NexusAuthPolicy::local_development_or_basic_from_env()?;
    let nexus = rullst::nexus::Nexus::new()
        .with_auth_policy(nexus_auth)
        .with_brand("SaaS Admin")
        .register::<models::user::User>()
        .register::<models::subscription::Subscription>()
        .try_build()?;

    let router = routes![
        get("/" => controllers::billing_controller::pricing_view),
        get("/pricing" => controllers::billing_controller::pricing_view),
        get("/login" => controllers::auth_controller::login_view),
        post("/login" => controllers::auth_controller::login_submit),
        get("/register" => controllers::auth_controller::register_view),
        post("/register" => controllers::auth_controller::register_submit),
        get("/logout" => controllers::auth_controller::logout),
    ];

    Ok(router.route("/dashboard", rullst::routing::get(controllers::auth_controller::dashboard)
        .layer(rullst::server::from_fn(middlewares::auth_middleware::auth_middleware)))
    .route("/billing/checkout", rullst::routing::get(controllers::billing_controller::checkout_redirect)
        .layer(rullst::server::from_fn(middlewares::auth_middleware::auth_middleware)))
    .route("/billing/portal", rullst::routing::get(controllers::billing_controller::portal_redirect)
        .layer(rullst::server::from_fn(middlewares::auth_middleware::auth_middleware)))
    .layer(rullst::server::from_fn(rullst::security::csrf_middleware))
    .route("/billing/webhook", rullst::routing::post(controllers::billing_controller::webhook_handler)
        .route_layer(rullst::server::from_fn(controllers::billing_controller::verify_billing_webhook)))
    .layer(rullst::server::from_fn(rullst::security::headers_middleware))
    .nest_axum("/nexus", nexus))
}}

#[unsafe(no_mangle)]
pub extern "C" fn rullst_router_init() -> *mut Router {{
    let router = match router() {{
        Ok(router) => router,
        Err(error) => {{
            eprintln!("Nexus startup configuration error: {{error}}");
            Router::new()
        }}
    }};
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

    #[cfg(debug_assertions)]
    {{
        rullst::runtime::spawn(async {{
            if let Err(error) = rullst::studio::run_studio(5555).await {{
                eprintln!("Rullst Studio could not start: {{error}}");
            }}
        }});
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
        let router = {project_name_safe}::router()?;
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
    controllers::billing_controller::initialize_billing_provider()?;

    let nexus_auth = rullst::nexus::NexusAuthPolicy::local_development_or_basic_from_env()?;
    let nexus = rullst::nexus::Nexus::new()
        .with_auth_policy(nexus_auth)
        .with_brand("SaaS Admin")
        .register::<models::user::User>()
        .register::<models::subscription::Subscription>()
        .try_build()?;

    let router = routes![
        get("/" => controllers::billing_controller::pricing_view),
        get("/pricing" => controllers::billing_controller::pricing_view),
        get("/login" => controllers::auth_controller::login_view),
        post("/login" => controllers::auth_controller::login_submit),
        get("/register" => controllers::auth_controller::register_view),
        post("/register" => controllers::auth_controller::register_submit),
        get("/logout" => controllers::auth_controller::logout),
    ];

    let router = router.route("/dashboard", rullst::routing::get(controllers::auth_controller::dashboard)
        .layer(rullst::server::from_fn(middlewares::auth_middleware::auth_middleware)))
    .route("/billing/checkout", rullst::routing::get(controllers::billing_controller::checkout_redirect)
        .layer(rullst::server::from_fn(middlewares::auth_middleware::auth_middleware)))
    .route("/billing/portal", rullst::routing::get(controllers::billing_controller::portal_redirect)
        .layer(rullst::server::from_fn(middlewares::auth_middleware::auth_middleware)))
    .layer(rullst::server::from_fn(rullst::security::csrf_middleware))
    .route("/billing/webhook", rullst::routing::post(controllers::billing_controller::webhook_handler)
        .route_layer(rullst::server::from_fn(controllers::billing_controller::verify_billing_webhook)))
    .layer(rullst::server::from_fn(rullst::security::headers_middleware))
    .nest_axum("/nexus", nexus);

    #[cfg(debug_assertions)]
    {{
        rullst::runtime::spawn(async {{
            if let Err(error) = rullst::studio::run_studio(5555).await {{
                eprintln!("Rullst Studio could not start: {{error}}");
            }}
        }});
        println!("📊 Rullst Studio running on http://127.0.0.1:5555");
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
