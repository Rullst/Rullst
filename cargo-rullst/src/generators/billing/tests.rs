#![allow(clippy::expect_used)]

use super::*;

#[test]
// TM-DEPLOY-06: generated billing binds verified events to server-owned identity.
fn generated_billing_binds_signed_events_to_authenticated_owners() {
    for backend in [ProjectOrmBackend::Sqlx, ProjectOrmBackend::Turso] {
        let source = render_billing_controller("workspace_id", backend);
        syn::parse_file(&source).expect("billing controller must parse");
        // Production follows the environment `Server` enforces and fails closed.
        assert!(source.contains("match rullst::RullstConfig::global().environment() {"));
        assert!(source.contains("Err(_) => true,"));
        assert!(source.contains("rullst::config::project_setting(name)"));
        assert!(source.contains("workspace_id: identity.owner_id"));
        assert!(source.contains("Extension(identity): Extension<BillingIdentity>"));
        assert!(source.contains("rullst-capital's mandatory signature/replay middleware"));
        assert!(source.contains("verify_billing_webhook"));
        assert!(source.contains("initialize_billing_provider"));
        assert!(source.contains("strong_webhook_secret"));
        assert!(source.contains("BILLING_ALLOWED_PLAN_IDS"));
        assert!(source.contains("config.allowed_plan_ids.contains(&form.plan)"));
        assert!(source.contains("Form(form): Form<CheckoutForm>"));
        assert!(source.contains("CsrfToken"));
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

#[test]
fn billing_identity_failures_do_not_return_database_error_text() {
    for backend in [ProjectOrmBackend::Sqlx, ProjectOrmBackend::Turso] {
        let source = render_billing_controller("workspace_id", backend);
        assert!(!source.contains("Failed to persist billing identity: {error}"));
        assert!(!source.contains("Failed to query billing identity: {error}"));
        assert!(source.contains("eprintln!(\"Billing identity persistence failed: {error}\")"));
        assert!(source.contains("eprintln!(\"Billing identity lookup failed: {error}\")"));
        assert_eq!(
            source
                .matches("\"Billing identity is unavailable\".to_string()")
                .count(),
            2
        );
    }
}

#[test]
fn generated_billing_reads_settings_from_the_process_then_dotenv() {
    for backend in [ProjectOrmBackend::Sqlx, ProjectOrmBackend::Turso] {
        let controller = render_billing_controller("workspace_id", backend);
        let mut sources = vec![("src/controllers/billing_controller.rs", controller)];
        sources.extend(live_billing_files("workspace_id", backend));
        for (path, source) in sources.iter().filter(|(path, _)| path.ends_with(".rs")) {
            assert!(!source.contains("std::env::var"), "{path}");
            assert!(!source.contains("dotenvy"), "{path}");
            // Every billing key literal goes through the `.env`-aware helper.
            for (index, _) in source.match_indices("\"BILLING_") {
                let key = &source[index + 1..];
                let length = key
                    .find(|character: char| !(character.is_ascii_uppercase() || character == '_'))
                    .unwrap_or(key.len());
                if key[length..].starts_with('"') {
                    assert!(
                        source[..index].ends_with("setting("),
                        "{path}: {}",
                        &key[..length]
                    );
                }
            }
        }
        let readme = &sources
            .iter()
            .find(|(path, _)| *path == "BILLING.md")
            .expect("billing guide")
            .1;
        assert!(readme.contains("rullst::config::project_setting"));
    }
}
