#![allow(clippy::expect_used)]

use super::*;

#[test]
// TM-DEPLOY-06: generated billing binds verified events to server-owned identity.
fn generated_billing_binds_signed_events_to_authenticated_owners() {
    let source = render_billing_controller("workspace_id");
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

    let (subscription, customer) = render_billing_models("workspace_id");
    let migration =
        render_billing_migration("m20260830000000_create_subscriptions_table", "workspace_id");
    syn::parse_file(&subscription).expect("subscription model must parse");
    syn::parse_file(&customer).expect("billing customer model must parse");
    syn::parse_file(&migration).expect("billing migration must parse");
    assert!(!subscription.contains("backend = \"turso\""));
    assert!(migration.contains("subscriptions_subscription_id_unique"));
    assert!(
        migration.contains("DROP TABLE subscriptions")
            || migration.contains("drop_if_exists(\"subscriptions\")")
    );
}

#[test]
fn billing_identity_failures_do_not_return_database_error_text() {
    let source = render_billing_controller("workspace_id");
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

#[test]
fn generated_billing_reads_settings_from_the_process_then_dotenv() {
    let controller = render_billing_controller("workspace_id");
    let mut sources = vec![("src/controllers/billing_controller.rs", controller)];
    sources.extend(live_billing_files("workspace_id"));
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

#[test]
fn generated_billing_rows_get_real_timestamps() {
    let source = render_billing_controller("workspace_id");
    // The ORM inserts every field, so '' replaced the CURRENT_TIMESTAMP
    // default of new billing customers and subscriptions.
    assert!(!source.contains("created_at: String::new()"));
    assert!(!source.contains("updated_at: String::new()"));
    assert_eq!(
        source.matches("let created_at = utc_timestamp();").count(),
        2
    );
    assert!(!source.contains("__UTC_TIMESTAMP_FN__") && !source.contains("__UPDATED_AT__"));
    // `timestamps()` columns hold `chrono` instants on SQLx pools.
    assert_eq!(source.matches("updated_at: created_at,").count(), 2);
    // New rows start an empty dirty-tracking snapshot.
    assert_eq!(
        source
            .matches(
                "updated_at: created_at,\n                snapshot: rullst::db::RowSnapshot::default(),\n                created_at,\n"
            )
            .count(),
        2
    );
    assert!(source.contains(
        "fn utc_timestamp() -> rullst::orm::chrono::DateTime<rullst::orm::chrono::Utc> {\n    rullst::orm::chrono::Utc::now()\n}"
    ));
    assert!(!source.contains("{year:04}"));
}

#[test]
fn every_placeholder_is_filled_with_the_sqlx_profile() {
    let mut sources = vec![
        (
            "src/controllers/billing_controller.rs",
            render_billing_controller("workspace_id"),
        ),
        (
            "src/migrations/m1.rs",
            render_billing_migration("m1", "workspace_id"),
        ),
    ];
    let (subscription, customer) = render_billing_models("workspace_id");
    sources.push(("src/models/subscription.rs", subscription));
    sources.push(("src/models/billing_customer.rs", customer));
    sources.extend(live_billing_files("workspace_id"));
    for (path, source) in &sources {
        assert!(!source.contains("__"), "{path} keeps a placeholder");
        assert!(!source.contains("TursoModel"), "{path}");
        assert!(!source.contains("TursoOrm"), "{path}");
    }
    let store = &sources
        .iter()
        .find(|(path, _)| *path == "src/controllers/billing_store.rs")
        .expect("billing store")
        .1;
    assert!(store.contains("i32::try_from(next.owner)"));
}

#[test]
fn turso_primary_projects_are_refused_with_the_supported_alternative() {
    assert!(require_sqlx(ProjectOrmBackend::Sqlx).is_ok());
    let error = require_sqlx(ProjectOrmBackend::Turso).expect_err("Turso is refused");
    assert_eq!(error.kind(), ErrorKind::Unsupported);
    let message = error.to_string();
    assert_eq!(message, TURSO_REFUSAL);
    for part in [
        "make:billing needs the SQLx database layer",
        "SQLite, PostgreSQL, MySQL or MariaDB",
        "Turso is supported only as the primary database of the Blank and API starters",
        "nothing was written",
    ] {
        assert!(message.contains(part), "{part}");
    }
}
