//! ERP example: products, stock movements and orders, with the dashboard,
//! forms and Nexus behind one administrator policy.
//!
//! An example to copy from, not a supported starter; see `README.md`.

use rullst::{Router, routes};

pub mod controllers;
pub mod migrations;
pub mod models;
pub mod pages;

pub fn router() -> Result<Router, Box<dyn std::error::Error>> {
    let admin_access = match rullst::nexus::NexusAuthPolicy::local_development_or_basic_from_env() {
        Ok(policy) => policy,
        // The back office fails closed, but the process still starts: a
        // release build without the credentials serves only `/health` and `/ready`.
        Err(rullst::nexus::NexusBuildError::MissingCredential { variable }) => {
            eprintln!(
                "The back office and Nexus are not mounted: {variable} is not set; set NEXUS_ADMIN_USERNAME and NEXUS_ADMIN_PASSWORD (at least 16 characters) to enable them"
            );
            return Ok(Router::new()
                .merge_axum(rullst::health::health_router())
                .layer(rullst::server::from_fn(
                    rullst::security::headers_middleware,
                )));
        }
        Err(error) => return Err(error.into()),
    };
    let nexus = rullst::nexus::Nexus::new()
        .with_auth_policy(admin_access.clone())
        .with_brand("ERP Admin")
        .register::<models::product::Product>()
        .register::<models::order::Order>()
        .try_build()?;

    // The whole back office shares the Nexus administrator policy, including the
    // dashboard that lists every order's customer and the revenue totals:
    // loopback-only in debug builds, Basic Auth over verified TLS in release.
    let admin_routes = routes![
        get("/" => controllers::erp_controller::index),
        post("/products" => controllers::erp_controller::store_product),
        // rullst-access: admin — protected by admin_access.protect_router below.
        post("/products/{id}/add-stock" => controllers::erp_controller::add_stock),
        post("/orders" => controllers::erp_controller::store_order),
    ];
    let admin_routes = admin_access.protect_router(admin_routes.into_axum())?;

    Ok(Router::new()
        .merge_axum(admin_routes)
        .layer(rullst::server::from_fn(rullst::security::csrf_middleware))
        // `/health` and `/ready` for container, Kubernetes and PaaS probes,
        // outside the administrator policy.
        .merge_axum(rullst::health::health_router())
        .layer(rullst::server::from_fn(
            rullst::security::headers_middleware,
        ))
        .nest_axum("/nexus", nexus))
}

// Security checks against `router()`; run them with `cargo test`.
#[cfg(all(test, debug_assertions))]
mod security_tests;
