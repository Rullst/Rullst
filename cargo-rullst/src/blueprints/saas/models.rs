// cargo-rullst/src/blueprints/saas/models.rs — Database models and migrations for SaaS blueprint.
// The model and migration templates live under `src/` beside this module.

use crate::generators::{ProjectOrmBackend, billing::render_billing_models};

pub fn get_models_and_migrations() -> Vec<(&'static str, String)> {
    // Share persistence methods with make:billing so placeholder dialects and
    // ownership-model fixes cannot drift between the two generators.
    let (mut subscription_model, billing_customer_model) =
        render_billing_models("user_id", ProjectOrmBackend::Sqlx);
    subscription_model.push_str(include_str!("fragments/subscription_nexus.rs.template"));

    vec![
        (
            "src/models/user.rs",
            include_str!("src/models/user.rs.template").to_string(),
        ),
        ("src/models/subscription.rs", subscription_model),
        ("src/models/billing_customer.rs", billing_customer_model),
        (
            "src/models/mod.rs",
            include_str!("src/models/mod.rs.template").to_string(),
        ),
        (
            "src/migrations/m20260601000000_create_users_table.rs",
            include_str!("src/migrations/m20260601000000_create_users_table.rs.template")
                .to_string(),
        ),
        (
            "src/migrations/m20260601000002_create_subscriptions_table.rs",
            crate::generators::billing::render_billing_migration(
                "m20260601000002_create_subscriptions_table",
                "user_id",
                ProjectOrmBackend::Sqlx,
            ),
        ),
        (
            "src/migrations/mod.rs",
            include_str!("src/migrations/mod.rs.template").to_string(),
        ),
    ]
}
