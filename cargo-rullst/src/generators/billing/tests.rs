#![allow(clippy::expect_used)]

use super::*;

#[test]
// TM-DEPLOY-06: generated billing binds verified events to server-owned identity.
fn generated_billing_binds_signed_events_to_authenticated_owners() {
    for backend in [ProjectOrmBackend::Sqlx, ProjectOrmBackend::Turso] {
        let source = render_billing_controller("workspace_id", backend);
        syn::parse_file(&source).expect("billing controller must parse");
        let canonical_environment = source
            .find("std::env::var(\"RULLST_ENV\")")
            .expect("canonical environment lookup");
        let legacy_environment = source
            .find("std::env::var(\"APP_ENV\")")
            .expect("legacy environment fallback");
        assert!(canonical_environment < legacy_environment);
        assert!(source.contains("workspace_id: identity.owner_id"));
        assert!(source.contains("Extension(identity): Extension<BillingIdentity>"));
        assert!(source.contains("rullst-capital's mandatory signature/replay middleware"));
        assert!(source.contains("verify_billing_webhook"));
        assert!(source.contains("initialize_billing_provider"));
        assert!(source.contains("strong_webhook_secret"));
        assert!(source.contains("BILLING_ALLOWED_PLAN_IDS"));
        assert!(source.contains("config.allowed_plan_ids.contains(&query.plan)"));
        assert!(source.contains("find_by_subscription_id"));
        assert!(!source.contains("#[derive(Debug)]\nstruct BillingConfig"));
        assert!(!source.contains(".bind(1)"));
        assert!(!source.contains("user@example.com"));
        assert!(!source.contains("mock_secret"));
        assert!(!source.contains(".unwrap("));
        assert!(!source.contains(".expect("));
        assert!(!source.contains("panic!("));

        let (subscription, customer) = render_billing_models("workspace_id", backend);
        let migration = render_billing_migration(
            "m20260830000000_create_subscriptions_table",
            "workspace_id",
            backend,
        );
        syn::parse_file(&subscription).expect("subscription model must parse");
        syn::parse_file(&customer).expect("billing customer model must parse");
        syn::parse_file(&migration).expect("billing migration must parse");
        assert_eq!(
            subscription.contains("backend = \"turso\""),
            backend == ProjectOrmBackend::Turso
        );
        assert!(migration.contains("subscriptions_subscription_id_unique"));
        assert!(
            migration.contains("DROP TABLE subscriptions")
                || migration.contains("drop_if_exists(\"subscriptions\")")
        );
    }
}
