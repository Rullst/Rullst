// cargo-rullst/src/generators/auth/models.rs — Migration and User model generator.

use crate::generators::migration::regenerate_migrations_mod;
use colored::*;
use std::fs;
use std::path::Path;

pub fn generate_user_model_and_migration() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create User Migration
    let migrations_dir = Path::new("src/migrations");
    fs::create_dir_all(migrations_dir)?;
    let now = chrono::Local::now();
    let timestamp = now.format("%Y%m%d%H%M%S").to_string();
    let file_stem = format!("m{}_create_users_table", timestamp);
    let migration_path = migrations_dir.join(format!("{}.rs", file_stem));

    let migration_template = format!(
        r##"use rullst::db::{{Orm, sqlx}};
use rullst::db::schema::{{Schema, Migration}};
use rullst::db::async_trait;

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {{
    fn name(&self) -> &'static str {{
        "{file_stem}"
    }}

    async fn up(&self) -> Result<(), rullst_orm::error::RullstError> {{
        Schema::create("users", |table| {{
            table.id();
            table.string("name").not_null();
            table.string("email").not_null();
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

    async fn down(&self) -> Result<(), rullst_orm::error::RullstError> {{
        Schema::drop_if_exists("users").await
    }}
}}
"##,
        file_stem = file_stem
    );
    fs::write(&migration_path, migration_template)?;
    println!("{}", "  ✨ Created 'users' table migration.".green());

    regenerate_migrations_mod()?;

    // 2. Create User Model
    let models_dir = Path::new("src/models");
    fs::create_dir_all(models_dir)?;
    let model_path = models_dir.join("user.rs");
    let model_template = r##"use rullst::db::{Orm, FromRow};

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
    pub async fn find_by_email(email: &str) -> Result<Option<Self>, rullst_orm::Error> {
        Self::query()
            .where_eq("email", email.to_owned())
            .first()
            .await
    }
}
"##;
    fs::write(&model_path, model_template)?;
    println!("{}", "  ✨ Created 'User' model.".green());

    let mod_models_path = models_dir.join("mod.rs");
    if !mod_models_path.exists() {
        fs::write(&mod_models_path, "")?;
    }
    let mut mod_models_content = fs::read_to_string(&mod_models_path)?;
    let mut modified = false;
    if !mod_models_content.contains("pub mod user;") {
        mod_models_content.push_str("pub mod user;\n");
        modified = true;
    }
    if modified {
        fs::write(&mod_models_path, mod_models_content)?;
    }

    Ok(())
}
