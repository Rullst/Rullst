//! Portfolio example: a developer profile, projects, experience and skills
//! rendered from the database and edited in Nexus.
//!
//! An example to copy from, not a supported starter; see `README.md`.

use rullst::{Router, routes};

pub mod controllers;
pub mod migrations;
pub mod models;
pub mod pages;

pub fn router() -> Result<Router, Box<dyn std::error::Error>> {
    // Nexus fails closed, but the site still starts: a release build without
    // NEXUS_ADMIN_USERNAME and NEXUS_ADMIN_PASSWORD serves it without `/nexus`.
    let nexus = match rullst::nexus::NexusAuthPolicy::local_development_or_basic_from_env() {
        Ok(nexus_auth) => Some(
            rullst::nexus::Nexus::new()
                .with_auth_policy(nexus_auth)
                .with_brand("Portfolio CMS Admin")
                .register::<models::profile::Profile>()
                .register::<models::project::Project>()
                .register::<models::experience::Experience>()
                .register::<models::skill::Skill>()
                .try_build()?,
        ),
        Err(rullst::nexus::NexusBuildError::MissingCredential { variable }) => {
            eprintln!(
                "Nexus is not mounted at /nexus: {variable} is not set; set NEXUS_ADMIN_USERNAME and NEXUS_ADMIN_PASSWORD (at least 16 characters) to enable it"
            );
            None
        }
        Err(error) => return Err(error.into()),
    };

    let router = routes![
        get("/" => controllers::portfolio_controller::index),
    ]
    // `/health` and `/ready` for container, Kubernetes and PaaS probes.
    .merge_axum(rullst::health::health_router());
    Ok(match nexus {
        Some(nexus) => router.nest_axum("/nexus", nexus),
        None => router,
    })
}

// Security checks against `router()`; run them with `cargo test`.
#[cfg(all(test, debug_assertions))]
mod security_tests;
