#![allow(clippy::needless_update)]
#![allow(unexpected_cfgs)]
#![cfg_attr(mutants, mutants::skip)]

pub mod ai_demo;
#[cfg(not(target_arch = "wasm32"))]
mod assets;
pub mod billing_demo;
#[cfg(not(target_arch = "wasm32"))]
pub mod database;
pub mod omni_demo;
pub mod pico_demo;
#[cfg(not(target_arch = "wasm32"))]
mod public_origin;
pub mod repository_demo;
pub mod security_demo;
pub mod showcase_nav;
pub mod templates_demo;

#[cfg(not(target_arch = "wasm32"))]
pub mod live_counter;

#[cfg(not(target_arch = "wasm32"))]
pub mod app;

#[cfg(not(target_arch = "wasm32"))]
pub fn router() -> Result<rullst::Router, Box<dyn std::error::Error>> {
    let nexus_auth = rullst_nexus::NexusAuthPolicy::local_development_or_basic_from_env()?;
    router_with_nexus_auth(nexus_auth)
}

#[cfg(not(target_arch = "wasm32"))]
fn router_with_nexus_auth(
    nexus_auth: rullst_nexus::NexusAuthPolicy,
) -> Result<rullst::Router, Box<dyn std::error::Error>> {
    use app::*;
    use rullst::routes;

    let config =
        rullst::TenantConfig::new(rullst::TenantStrategy::Header).with_header_name("X-Tenant-ID");
    // Local showcase fixture standing in for authenticated membership claims. Production
    // applications must derive this extension from a verified session or token.
    let demo_membership = rullst::security::TenantMembership::try_new([
        "community",
        "tenant-enterprise",
        "tenant-startup",
    ])?
    .with_default("community")?;

    let nexus_router = rullst_nexus::Nexus::new()
        .with_auth_policy(nexus_auth)
        .with_brand("Rullst Sovereign Publisher")
        .register::<Post>()
        .try_build()?;
    rullst_security::register_deception_trap("/wp-admin");

    Ok(routes![
        get("/" => index),
        post("/posts" => store),
        get("/posts/repository" => crate::repository_demo::repository_page),
        get("/live-feed" => live_demo),
        get("/live-counter" => live_demo),
        get("/_live" => live_ws),
        get("/pico-demo" => crate::pico_demo::render_pico_demo_page),
        get("/templates-demo" => crate::templates_demo::render_templates_demo_page),
        get("/pricing" => crate::billing_demo::pricing_page),
        get("/billing" => crate::billing_demo::pricing_page),
        get("/checkout" => crate::billing_demo::checkout_handler_get),
        post("/checkout" => crate::billing_demo::checkout_handler_post),
        get("/security-demo" => crate::security_demo::security_page),
        get("/ai-assistant" => crate::ai_demo::ai_page),
        get("/omni" => crate::omni_demo::omni_page),
        get("/wp-admin" => honeypot_trap),
        get("/assets/showcase.css" => crate::assets::showcase_css),
        get("/assets/pages.css" => crate::assets::pages_css),
        get("/assets/showcase.js" => crate::assets::showcase_js),
        get("/assets/vendor/htmx-1.9.12.min.js" => crate::assets::htmx_js),
        get("/assets/vendor/htmx-ext-ws-1.9.12.js" => crate::assets::htmx_ws_js),
        get("/assets/vendor/pico-2.1.1.slate.min.css" => crate::assets::pico_css),
        get("/assets/rullst-logo.png" => crate::assets::rullst_logo),
        get("/favicon.ico" => crate::assets::rullst_logo),
        get("/robots.txt" => robots_txt),
        get("/sitemap.xml" => sitemap_xml),
    ]
    .nest_axum("/nexus", nexus_router)
    .layer(axum::extract::DefaultBodyLimit::max(app::MAX_FORM_BYTES))
    .layer(rullst::tenant_layer(config))
    .layer(axum::Extension(demo_membership))
    .layer(axum::middleware::from_fn(
        rullst_security::deception_trap_middleware,
    ))
    .layer(axum::middleware::from_fn(rullst::security::csrf_middleware))
    // The production header baseline, including the nonce-based CSP, is also
    // mounted here so development serves the policy that `Server` enforces in
    // staging and production. Repeating it there reuses the same nonce.
    .layer(axum::middleware::from_fn(
        rullst::security::headers_middleware,
    )))
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn rullst_router_init() -> *mut rullst::Router {
    match router() {
        Ok(router) => Box::into_raw(Box::new(router)),
        Err(error) => {
            eprintln!("Nexus startup configuration error: {error}");
            std::ptr::null_mut()
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;
