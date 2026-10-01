// cargo-rullst/src/blueprints/saas/billing.rs — Stripe/Capital pricing & checkout views for SaaS blueprint.
// The page templates live under `src/` beside this module.

const SAAS_STYLES: &str = include_str!("styles.css");

pub fn get_billing_pages() -> Vec<(&'static str, String)> {
    vec![
        (
            "src/pages/billing.rs",
            include_str!("src/pages/billing.rs.template").to_string(),
        ),
        ("static/rullst.css", SAAS_STYLES.to_string()),
        (
            "src/pages/mod.rs",
            include_str!("src/pages/mod.rs.template").to_string(),
        ),
    ]
}
