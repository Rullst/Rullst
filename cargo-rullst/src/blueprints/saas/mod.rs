// cargo-rullst/src/blueprints/saas/mod.rs — Root of SaaS blueprint module (< 50 lines).

pub mod billing;
pub mod models;
pub mod routes;

use super::common;

pub fn file_manifest(
    project_name_safe: &str,
    hot_reload: bool,
    orm_pattern: &str,
    frontend_engine: &str,
) -> Vec<(&'static str, String)> {
    let mut manifest = Vec::new();
    let is_repo = common::is_repo_mode(orm_pattern);
    let _ = frontend_engine;

    manifest.extend(routes::get_routes(project_name_safe, hot_reload));
    manifest.extend(models::get_models_and_migrations());
    manifest.extend(billing::get_billing_pages());

    if is_repo {
        manifest.push((
            "src/repositories/user_repository.rs",
            common::generate_repository("User", "users"),
        ));
        manifest.push((
            "src/repositories/subscription_repository.rs",
            common::generate_repository("Subscription", "subscriptions"),
        ));
        manifest.push((
            "src/repositories/mod.rs",
            common::generate_repositories_mod(&["User", "Subscription"]),
        ));
    }

    manifest
}
