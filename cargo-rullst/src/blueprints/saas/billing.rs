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

#[cfg(test)]
mod tests {
    #[test]
    fn pages_keep_their_layout_at_phone_width() {
        // render_page* links this stylesheet: several elements in <body>
        // stack instead of sitting side by side.
        let styles = super::SAAS_STYLES;
        let body = styles
            .split("\n\nbody {")
            .nth(1)
            .and_then(|rule| rule.split('}').next())
            .expect("the body rule");
        assert!(body.contains("flex-direction: column;"), "{body}");
        // The dashboard's actions wrap instead of overlapping on a phone.
        let dashboard = include_str!("src/pages/auth.rs.template");
        for rule in [
            ".topbar { display: flex; flex-wrap: wrap;",
            ".actions { display: flex; flex-wrap: wrap;",
            ".btn-nexus { display: inline-block;",
            "@media (max-width: 600px)",
            "<div class=\"actions\">",
        ] {
            assert!(dashboard.contains(rule), "{rule}");
        }
        assert!(!dashboard.contains("margin-right: 0.75rem"));
    }
}
