// cargo-rullst/src/blueprints/saas/mod.rs — Root of SaaS blueprint module.
// The templates under `src/` mirror the generated project's layout; the snippet
// under `fragments/` extends a shared billing model.

pub mod billing;
pub mod models;
pub mod routes;

use super::common;
use crate::generators::{
    ProjectOrmBackend, auth::controllers::render_auth_controller,
    billing::render_billing_controller,
};

pub fn file_manifest(
    project_name_safe: &str,
    hot_reload: bool,
    orm_pattern: &str,
    frontend_engine: &str,
) -> Vec<(&'static str, String)> {
    let mut manifest = Vec::new();
    let is_repo = common::is_repo_mode(orm_pattern);
    let _ = frontend_engine;

    manifest.extend(routes::get_routes(
        project_name_safe,
        hot_reload,
        orm_pattern,
    ));
    manifest.extend(models::get_models_and_migrations());
    manifest.extend(billing::get_billing_pages());
    manifest.extend(crate::generators::billing::live_billing_files(
        "user_id",
        ProjectOrmBackend::Sqlx,
    ));

    // 1. Controllers
    manifest.push((
        "src/controllers/auth_controller.rs",
        render_auth_controller(None),
    ));
    manifest.push((
        "src/controllers/billing_controller.rs",
        render_billing_controller("user_id", ProjectOrmBackend::Sqlx),
    ));
    manifest.push((
        "src/controllers/mod.rs",
        include_str!("src/controllers/mod.rs.template").to_string(),
    ));

    // 2. Middlewares
    manifest.push((
        "src/middlewares/auth_middleware.rs",
        include_str!("src/middlewares/auth_middleware.rs.template").to_string(),
    ));
    manifest.push((
        "src/middlewares/mod.rs",
        include_str!("src/middlewares/mod.rs.template").to_string(),
    ));

    // 3. Pages Auth
    manifest.push((
        "src/pages/auth.rs",
        include_str!("src/pages/auth.rs.template").to_string(),
    ));

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
