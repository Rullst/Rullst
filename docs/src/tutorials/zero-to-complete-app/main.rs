use rullst::{routes, Server};

pub mod migrations;
pub mod models;
pub mod controllers;
pub mod middlewares;
pub mod pages;

#[rullst::runtime::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
        get("/register" => controllers::auth_controller::register_view),
        post("/logout" => controllers::auth_controller::logout),
    ];

    // Each credential submission runs Argon2id, so it is budgeted per client.
    let router = router.route("/login", rullst::routing::post(controllers::auth_controller::login_submit)
        .layer(rullst::server::from_fn(controllers::auth_controller::credential_rate_limit)))
    .route("/register", rullst::routing::post(controllers::auth_controller::register_submit)
        .layer(rullst::server::from_fn(controllers::auth_controller::credential_rate_limit)))
    .route("/dashboard", rullst::routing::get(controllers::auth_controller::dashboard)
        .layer(rullst::server::from_fn(middlewares::auth_middleware::auth_middleware)))
    .route("/reports/billing", rullst::routing::get(controllers::billing_controller::billing_report)
        .layer(rullst::server::from_fn(middlewares::auth_middleware::auth_middleware)))
    .route("/billing/checkout", rullst::routing::post(controllers::billing_controller::checkout_redirect)
        .layer(rullst::server::from_fn(middlewares::auth_middleware::auth_middleware)))
    .route("/billing/portal", rullst::routing::post(controllers::billing_controller::portal_redirect)
        .layer(rullst::server::from_fn(middlewares::auth_middleware::auth_middleware)))
    // Notes: signed-in users only. Each note belongs to the user who wrote it.
    .route("/notes", rullst::routing::get(controllers::notes_controller::index)
        .post(controllers::notes_controller::store)
        .layer(rullst::server::from_fn(middlewares::auth_middleware::auth_middleware)))
    // rullst-access: owner — find_owned() calls RbacGuard::authorize_owner_or_role
    .route("/notes/{id}", rullst::routing::get(controllers::notes_controller::show)
        .post(controllers::notes_controller::update)
        .layer(rullst::server::from_fn(middlewares::auth_middleware::auth_middleware)))
    // rullst-access: owner — find_owned() calls RbacGuard::authorize_owner_or_role
    .route("/notes/{id}/delete", rullst::routing::post(controllers::notes_controller::destroy)
        .layer(rullst::server::from_fn(middlewares::auth_middleware::auth_middleware)))
    .layer(rullst::server::from_fn(rullst::security::csrf_middleware))
    .route("/billing/webhook", rullst::routing::post(controllers::billing_controller::webhook_handler)
        .route_layer(rullst::server::from_fn(controllers::billing_controller::verify_billing_webhook)))
    // `/health` and `/ready` for container, Kubernetes and PaaS probes.
    .merge_axum(rullst::health::health_router())
    .layer(rullst::server::from_fn(rullst::security::headers_middleware))
    .nest_axum("/nexus", nexus);

    #[cfg(debug_assertions)]
    {
        rullst::runtime::spawn(async {
            if let Err(error) = rullst::studio::run_studio(5555).await {
                eprintln!("Rullst Studio could not start: {error}");
            }
        });
        println!("📊 Rullst Studio running on http://127.0.0.1:5555");
    }
    println!("🚀 SaaS server starting on port 3000...");
    Server::new(router)
        .run(3000)
        .await?;

    Ok(())
}
