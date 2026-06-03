use rullst::{routes, Server};

pub mod migrations;
pub mod models;
pub mod controllers;
pub mod middlewares;
pub mod pages;

#[rullst::runtime::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Run migrations on startup
    rullst::artisan!(crate::migrations::get_migrations());

    let router = routes![
        // Public routes
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
    .layer(rullst::server::from_fn(rullst::security::headers_middleware));

    println!("🚀 SaaS server starting on port 3000...");
    Server::new(router)
        .run(3000)
        .await?;

    Ok(())
}
