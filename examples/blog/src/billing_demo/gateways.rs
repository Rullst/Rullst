//! Payment Gateways Catalog and Configuration Metadata for Rullst Capital.
//! Defines payment-adapter metadata with environment credential detection.

use std::sync::OnceLock;

/// The credential variable whose presence marks each gateway as configured.
const CREDENTIAL_VARIABLES: [(&str, &str); 2] = [
    ("stripe", "STRIPE_SECRET_KEY"),
    ("infinitepay", "INFINITEPAY_API_KEY"),
];

/// Ids of the gateways whose credential variable `is_set` reports, asking
/// once per variable.
fn resolve_configured(mut is_set: impl FnMut(&str) -> bool) -> Vec<&'static str> {
    CREDENTIAL_VARIABLES
        .iter()
        .filter(|(_, variable)| is_set(variable))
        .map(|(id, _)| *id)
        .collect()
}

/// Ids of the gateways with a credential in the process environment or
/// `./.env` (the `Server` precedence), resolved once per process.
///
/// `project_setting` reads and parses `./.env` synchronously for each variable
/// the process environment lacks, so the pricing pages must not call it per
/// request. The router resolves this at start-up; credentials added later
/// appear after a restart.
pub fn configured_gateway_ids() -> &'static [&'static str] {
    static CONFIGURED: OnceLock<Vec<&'static str>> = OnceLock::new();
    CONFIGURED.get_or_init(|| {
        resolve_configured(|name| matches!(rullst::config::project_setting(name), Ok(Some(_))))
    })
}

/// Metadata model for a supported payment gateway.
#[derive(Debug, Clone)]
pub struct GatewayInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub archetype: &'static str,
    pub archetype_badge_class: &'static str,
    pub flag: &'static str,
    pub current_boundary: &'static str,
    pub env_example: &'static str,
    pub rust_init_code: &'static str,
}

impl GatewayInfo {
    /// Returns true when an expected credential variable existed at start-up.
    ///
    /// Presence does not validate the credential or prove that every adapter capability is live.
    pub fn is_configured(&self) -> bool {
        configured_gateway_ids().contains(&self.id)
    }

    /// Returns a credential-presence or offline-demo status label and CSS badge.
    pub fn status_badge(&self) -> (&'static str, &'static str) {
        if self.is_configured() {
            ("🔐 Credentials Detected (Unverified)", "status-live")
        } else {
            ("🟡 Offline Demo", "status-mock")
        }
    }
}

/// Returns the two built-in payment adapters represented by offline fixtures.
pub fn all_gateways() -> Vec<GatewayInfo> {
    vec![
        GatewayInfo {
            id: "infinitepay",
            name: "InfinitePay",
            archetype: "Experimental billing adapter",
            archetype_badge_class: "badge-emerald",
            flag: "🇧🇷",
            current_boundary: "Experimental: deterministic offline checkout fixture. Live plan-only checkout and live callbacks fail closed until validated against a live account.",
            env_example: "INFINITEPAY_API_KEY=\"inf_live_sec_...\"\nINFINITEPAY_WEBHOOK_SECRET=\"whsec_inf_...\"",
            rust_init_code: "use rullst_capital::{init_provider, InfinitePayProvider};\n\ninit_provider(Box::new(InfinitePayProvider::new(\n    std::env::var(\"INFINITEPAY_API_KEY\")?,\n    std::env::var(\"INFINITEPAY_WEBHOOK_SECRET\")?,\n)));",
        },
        GatewayInfo {
            id: "stripe",
            name: "Stripe",
            archetype: "Billing adapter",
            archetype_badge_class: "badge-blue",
            flag: "🌐",
            current_boundary: "Reviewed plan-based checkout adapter plus bounded immediate Payment Intent charge and documented webhook foundations; validate your live account and products.",
            env_example: "STRIPE_SECRET_KEY=\"sk_live_51...\"\nSTRIPE_WEBHOOK_SECRET=\"whsec_...\"",
            rust_init_code: "use rullst_capital::{init_provider, StripeProvider};\n\ninit_provider(Box::new(StripeProvider::new(\n    std::env::var(\"STRIPE_SECRET_KEY\")?,\n    std::env::var(\"STRIPE_WEBHOOK_SECRET\")?,\n)));",
        },
    ]
}

/// Generates deterministic offline adapter output for the showcase.
pub async fn simulate_provider_checkout(
    provider_id: &str,
    customer_email: &str,
    plan_id: &str,
    redirect_url: &str,
) -> Result<String, rullst_capital::CapitalError> {
    use rullst_capital::providers::*;

    match provider_id {
        "stripe" => {
            let p = StripeProvider::new("mock_stripe_key".to_string(), String::new());
            p.create_checkout_session(customer_email, plan_id, redirect_url)
                .await
        }
        "infinitepay" => {
            let p = InfinitePayProvider::new("mock_inf".to_string(), String::new());
            p.create_checkout_session(customer_email, plan_id, redirect_url)
                .await
        }
        _ => Err(rullst_capital::CapitalError::ConfigurationError(format!(
            "Unknown provider: {}",
            provider_id
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CREDENTIAL_VARIABLES, all_gateways, configured_gateway_ids, resolve_configured,
        simulate_provider_checkout,
    };
    use std::collections::HashSet;

    #[test]
    fn credential_presence_is_resolved_once_per_variable_and_cached() {
        let mut lookups = Vec::new();
        let configured = resolve_configured(|name| {
            lookups.push(name.to_owned());
            matches!(name, "STRIPE_SECRET_KEY")
        });
        assert_eq!(configured, ["stripe"]);
        assert_eq!(lookups.len(), all_gateways().len());
        assert_eq!(lookups.iter().collect::<HashSet<_>>().len(), lookups.len());
        for gateway in all_gateways() {
            assert!(
                CREDENTIAL_VARIABLES.iter().any(|(id, _)| *id == gateway.id),
                "{} has no credential variable",
                gateway.id
            );
        }

        // Every page render shares the start-up snapshot instead of reading
        // `./.env` again.
        assert!(std::ptr::eq(
            configured_gateway_ids(),
            configured_gateway_ids()
        ));
    }

    #[tokio::test]
    async fn every_catalogue_action_returns_bounded_offline_output() {
        let gateways = all_gateways();
        assert_eq!(gateways.len(), 2);

        let unique_ids = gateways
            .iter()
            .map(|gateway| gateway.id)
            .collect::<HashSet<_>>();
        assert_eq!(unique_ids.len(), gateways.len());

        for gateway in gateways {
            assert!(!gateway.current_boundary.trim().is_empty());
            let output = simulate_provider_checkout(
                gateway.id,
                "fixture@example.invalid",
                "pro_plan",
                "https://showcase.example.invalid/pricing?status=success",
            )
            .await
            .unwrap_or_else(|error| panic!("{} offline fixture failed: {error}", gateway.id));
            assert!(
                !output.trim().is_empty(),
                "{} fixture was empty",
                gateway.id
            );
            assert!(
                output.len() <= 2_048,
                "{} fixture was unbounded",
                gateway.id
            );
        }
    }

    #[tokio::test]
    async fn unknown_catalogue_action_fails_explicitly() {
        let error = simulate_provider_checkout(
            "unknown",
            "fixture@example.invalid",
            "pro_plan",
            "https://showcase.example.invalid/pricing",
        )
        .await
        .expect_err("unknown provider must fail");
        assert!(error.to_string().contains("Unknown provider"));
    }
}
