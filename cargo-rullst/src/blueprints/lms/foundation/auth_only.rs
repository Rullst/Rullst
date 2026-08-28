//! Compiling detached authentication profile for LMS applications.

const RETAINED_FILES: &[&str] = &[
    "src/controllers/auth_controller.rs",
    "src/models/user.rs",
    "src/pages/auth.rs",
];

pub(super) fn select(
    mut full_manifest: Vec<(&'static str, String)>,
) -> Vec<(&'static str, String)> {
    full_manifest.retain(|(path, _)| RETAINED_FILES.contains(path));
    full_manifest.extend([
        ("src/main.rs", MAIN_SOURCE.to_string()),
        (
            "src/controllers/mod.rs",
            "pub mod auth_controller;\n".to_string(),
        ),
        (
            "src/middlewares/auth_middleware.rs",
            super::FOUNDATION_AUTH_MIDDLEWARE.to_string(),
        ),
        (
            "src/middlewares/mod.rs",
            "pub mod auth_middleware;\n".to_string(),
        ),
        (
            "src/migrations/m20260827000000_add_auth_identity.rs",
            AUTH_MIGRATION.to_string(),
        ),
        ("src/migrations/mod.rs", AUTH_MIGRATIONS_MODULE.to_string()),
        ("src/models/mod.rs", "pub mod user;\n".to_string()),
        ("src/pages/mod.rs", "pub mod auth;\n".to_string()),
        (
            "rullst-lms-modules.json",
            "{\n  \"schema_version\": 1,\n  \"modules\": [\"auth\"],\n  \"profile\": \"auth\"\n}\n"
                .to_string(),
        ),
    ]);
    full_manifest.sort_unstable_by_key(|(path, _)| *path);
    full_manifest
}

const AUTH_MIGRATIONS_MODULE: &str = r##"pub mod m20260827000000_add_auth_identity;

pub fn get_migrations() -> Vec<Box<dyn rullst::db::schema::Migration>> {
    vec![Box::new(m20260827000000_add_auth_identity::MigrationImpl)]
}
"##;

const AUTH_MIGRATION: &str = r##"use rullst::db::{Orm, sqlx};
use rullst::db::async_trait;
use rullst::db::schema::{Migration, Schema};

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {
    fn name(&self) -> &'static str {
        "m20260827000000_add_auth_identity"
    }

    async fn up(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::create("users", |table| {
            table.id();
            table.string("name").not_null();
            table.string("email").not_null();
            table.string("password_hash").nullable();
            table.string("oauth_provider").nullable();
            table.string("oauth_id").nullable();
            table.timestamps();
        }).await?;
        sqlx::query(sqlx::AssertSqlSafe(
            "CREATE UNIQUE INDEX users_email_unique ON users(email)",
        )).execute(Orm::pool()?).await?;
        Ok(())
    }

    async fn down(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::drop_if_exists("users").await
    }
}
"##;

const MAIN_SOURCE: &str = r##"use rullst::{routes, Router, Server};

pub mod controllers;
pub mod middlewares;
pub mod migrations;
pub mod models;
pub mod pages;

pub fn router() -> Result<Router, Box<dyn std::error::Error>> {
    let nexus_auth = rullst::nexus::NexusAuthPolicy::local_development_or_basic_from_env()?;
    let nexus = rullst::nexus::Nexus::new()
        .with_auth_policy(nexus_auth)
        .with_brand("LMS Identity Admin")
        .register::<models::user::User>()
        .try_build()?;

    let public = routes![
        get("/" => controllers::auth_controller::login_view),
        get("/login" => controllers::auth_controller::login_view),
        post("/login" => controllers::auth_controller::login_submit),
        get("/register" => controllers::auth_controller::register_view),
        post("/register" => controllers::auth_controller::register_submit),
        post("/logout" => controllers::auth_controller::logout),
    ];
    let authenticated = routes![
        get("/dashboard" => controllers::auth_controller::dashboard),
    ].layer(rullst::server::from_fn(middlewares::auth_middleware::auth_middleware));

    Ok(public
        .merge_axum(authenticated.into_axum())
        .layer(rullst::server::from_fn(rullst::security::csrf_middleware))
        .layer(rullst::server::from_fn(rullst::security::headers_middleware))
        .nest_axum("/nexus", nexus))
}

#[rullst::runtime::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    rullst::artisan!(crate::migrations::get_migrations());
    #[cfg(debug_assertions)]
    rullst::runtime::spawn(async {
        if let Err(error) = rullst::studio::run_studio(5555).await {
            eprintln!("Rullst Studio could not start: {error}");
        }
    });
    Server::new(router()?).run(3000).await?;
    Ok(())
}
"##;
