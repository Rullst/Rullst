// cargo-rullst/src/generators/auth/models.rs — Migration and User model generator.

use crate::generators::{
    migration::regenerate_migrations_mod, output_guard::write_new, register_mod_ast,
};
use colored::*;
use std::fs;
use std::path::Path;

/// The `User` model written by `cargo rullst auth`.
const USER_MODEL_SOURCE: &str = r##"use rullst::db::{Orm, FromRow};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "users")]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub password_hash: Option<String>,
    pub oauth_provider: Option<String>,
    pub oauth_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl User {
    pub async fn find_by_email(email: &str) -> Result<Option<Self>, rullst::orm::Error> {
        Self::query()
            .where_eq("email", email.to_owned())
            .first()
            .await
    }
}
"##;

pub fn generate_user_model_and_migration() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create User Migration
    let migrations_dir = Path::new("src/migrations");
    fs::create_dir_all(migrations_dir)?;
    let now = chrono::Local::now();
    let timestamp = now.format("%Y%m%d%H%M%S").to_string();
    let file_stem = format!("m{}_create_users_table", timestamp);
    let migration_path = migrations_dir.join(format!("{}.rs", file_stem));
    write_new(
        &migration_path,
        user_migration_source(&file_stem).as_bytes(),
    )?;
    println!("{}", "  ✨ Created 'users' table migration.".green());

    regenerate_migrations_mod()?;

    // 2. Create User Model
    let models_dir = Path::new("src/models");
    fs::create_dir_all(models_dir)?;
    let model_path = models_dir.join("user.rs");
    write_new(&model_path, USER_MODEL_SOURCE.as_bytes())?;
    println!("{}", "  ✨ Created 'User' model.".green());
    register_mod_ast(&models_dir.join("mod.rs"), "user")?;

    Ok(())
}

/// The `users` migration written by `make:auth`.
fn user_migration_source(file_stem: &str) -> String {
    format!(
        r##"use rullst::db::{{Orm, sqlx}};
use rullst::db::schema::{{Schema, Migration}};
use rullst::db::async_trait;

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {{
    fn name(&self) -> &'static str {{
        "{file_stem}"
    }}

    async fn up(&self) -> Result<(), rullst::orm::Error> {{
        Schema::create("users", |table| {{
            table.id();
            table.string("name").not_null();
            // MySQL/MariaDB cannot index a TEXT column without a prefix length, so
            // indexed strings are bounded VARCHAR columns on every driver.
            table.string("email").not_null().col_type = "VARCHAR(255)".to_string();
            table.string("password_hash").nullable();
            table.string("oauth_provider").nullable();
            table.string("oauth_id").nullable();
            table.timestamps();
        }}).await?;
        sqlx::query("CREATE UNIQUE INDEX users_email_unique ON users(email)")
            .execute(Orm::pool()?)
            .await?;
        Ok(())
    }}

    async fn down(&self) -> Result<(), rullst::orm::Error> {{
        Schema::drop_if_exists("users").await
    }}
}}
"##,
        file_stem = file_stem
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexed_user_email_is_a_bounded_varchar_for_mysql() {
        let source = user_migration_source("m20260101000000_create_users_table");
        syn::parse_file(&source).expect("users migration must parse");
        assert!(source.contains(
            "table.string(\"email\").not_null().col_type = \"VARCHAR(255)\".to_string();"
        ));
        assert!(source.contains("CREATE UNIQUE INDEX users_email_unique ON users(email)"));
        assert!(source.contains("\"m20260101000000_create_users_table\""));
    }

    #[test]
    fn account_sources_name_the_orm_through_the_rullst_facade() {
        // `cargo rullst auth` enables the `orm` feature but adds no direct
        // `rullst-orm` dependency.
        let migration = user_migration_source("m20260101000000_create_users_table");
        let controller = crate::generators::auth::controllers::render_auth_controller(None);
        for source in [USER_MODEL_SOURCE, migration.as_str(), controller.as_str()] {
            assert!(!source.contains("rullst_orm"), "{source}");
            syn::parse_file(source).expect("account source must parse");
        }
        assert!(USER_MODEL_SOURCE.contains("Result<Option<Self>, rullst::orm::Error>"));
        assert!(migration.contains("Result<(), rullst::orm::Error>"));
    }
}
