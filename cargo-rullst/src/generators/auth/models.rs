// cargo-rullst/src/generators/auth/models.rs — Migration and User model generator.

use super::users_table::{UsersTable, render_account_columns_migration};
use crate::generators::{
    migration::regenerate_migrations_mod, output_guard::write_new, register_mod_ast,
};
use colored::*;
use std::fs;
use std::path::Path;

/// The `User` model written by `cargo rullst auth`.
const USER_MODEL_SOURCE: &str = r##"use rullst::db::{Orm, RowSnapshot};
use rullst::orm::chrono::{DateTime, Utc};

#[derive(Debug, Clone, Default, Orm)]
#[orm(table = "users")]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub password_hash: Option<String>,
    pub oauth_provider: Option<String>,
    pub oauth_id: Option<String>,
    // `timestamps()` columns: TIMESTAMPTZ on PostgreSQL, DATETIME(6) on
    // MySQL/MariaDB and RFC 3339 text on SQLite.
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // Lets this account into Nexus (`/nexus`) under the application's
    // sign-in. The database manages this column: ORM inserts and saves never
    // write it, so neither a form nor `save()` grants it, while queries can
    // still filter on it (`where_eq("is_admin", true)`). Set it to true with
    // your database client for the first administrator, who can then grant
    // others in Nexus.
    #[orm(db_default)]
    pub is_admin: bool,
    // Login cookies carry the epoch they were issued with. ORM inserts and
    // saves never write this column, so `save()` never undoes
    // `revoke_sessions`.
    #[orm(db_default)]
    pub session_epoch: i64,
    // Remembers the loaded values, so `save()` writes only the columns this
    // record changed. Build new records with `snapshot: RowSnapshot::default()`.
    #[sqlx(skip)]
    pub snapshot: RowSnapshot,
}

impl User {
    pub async fn find_by_email(email: &str) -> Result<Option<Self>, rullst::orm::Error> {
        Self::query()
            .where_eq("email", email.to_owned())
            .first()
            .await
    }
__RULLST_SESSION_EPOCH_METHODS__}
"##;

/// The session-epoch methods shared by the `make:auth`, SaaS and LMS `User`
/// models, which the shared auth controller and middlewares call.
pub(crate) const SESSION_EPOCH_METHODS: &str = r##"
    /// The epoch to put in this account's login cookies, or `None` when the
    /// column holds a negative value, which no generated code writes.
    pub fn current_session_epoch(&self) -> Option<u64> {
        u64::try_from(self.session_epoch).ok()
    }

    /// Whether a decoded login cookie was issued for this account at its
    /// current session epoch.
    pub fn accepts_session(&self, claims: &rullst::auth::SessionClaims) -> bool {
        claims.user_id == self.id
            && self
                .current_session_epoch()
                .is_some_and(|epoch| claims.verify_epoch(epoch).is_ok())
    }

    /// Signs out every device of this account: one UPDATE increments
    /// `session_epoch`, so each login cookie issued before stops matching.
    /// "Sign out everywhere" calls it; call it too when you deactivate or
    /// delete the account. Password changes and resets bump the epoch in the
    /// same UPDATE that stores the new hash.
    pub async fn revoke_sessions(id: i32) -> Result<(), rullst::orm::Error> {
        let increment = if rullst::db::Orm::driver()? == "postgres" {
            "UPDATE users SET session_epoch = session_epoch + 1 WHERE id = $1"
        } else {
            "UPDATE users SET session_epoch = session_epoch + 1 WHERE id = ?"
        };
        rullst::db::sqlx::query(increment)
            .bind(id)
            .execute(rullst::db::Orm::pool()?)
            .await?;
        Ok(())
    }

    /// Stores a new password hash and signs out every device in one UPDATE,
    /// but only while the account is still at `epoch`, the epoch of the
    /// cookie that asked. Returns the new epoch for that browser's next
    /// cookie, or `None` when its sessions were revoked meanwhile and nothing
    /// changed.
    pub async fn change_password(
        id: i32,
        epoch: u64,
        password_hash: &str,
        updated_at: rullst::orm::chrono::DateTime<rullst::orm::chrono::Utc>,
    ) -> Result<Option<u64>, rullst::orm::Error> {
        let (Ok(current), Some(next)) = (i64::try_from(epoch), epoch.checked_add(1)) else {
            return Ok(None);
        };
        let update = if rullst::db::Orm::driver()? == "postgres" {
            "UPDATE users SET password_hash = $1, updated_at = $2, session_epoch = session_epoch + 1 WHERE id = $3 AND session_epoch = $4"
        } else {
            "UPDATE users SET password_hash = ?, updated_at = ?, session_epoch = session_epoch + 1 WHERE id = ? AND session_epoch = ?"
        };
        let changed = rullst::db::sqlx::query(update)
            .bind(password_hash)
            .bind(updated_at)
            .bind(id)
            .bind(current)
            .execute(rullst::db::Orm::pool()?)
            .await?
            .rows_affected();
        Ok((changed == 1).then_some(next))
    }
"##;

/// Marker in the starter `User` model templates replaced by
/// [`SESSION_EPOCH_METHODS`].
pub(crate) const SESSION_EPOCH_MARKER: &str = "__RULLST_SESSION_EPOCH_METHODS__";

/// A `User` model template with the session-epoch methods in place.
pub(crate) fn with_session_epoch_methods(template: &str) -> String {
    template.replace(SESSION_EPOCH_MARKER, SESSION_EPOCH_METHODS)
}

/// Writes a complete `users` table migration and the `User` model.
pub fn generate_user_model_and_migration() -> Result<(), Box<dyn std::error::Error>> {
    let timestamp = chrono::Local::now().format("%Y%m%d%H%M%S").to_string();
    write_user_model_and_migration(&UsersTable::Create, &timestamp)
}

/// Writes the `users` migration `users` plans, named after `timestamp`, and
/// the `User` model.
pub(crate) fn write_user_model_and_migration(
    users: &UsersTable,
    timestamp: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create or extend the users table
    let migrations_dir = Path::new("src/migrations");
    fs::create_dir_all(migrations_dir)?;
    match users {
        UsersTable::Create => {
            let file_stem = format!("m{timestamp}_create_users_table");
            write_new(
                &migrations_dir.join(format!("{file_stem}.rs")),
                user_migration_source(&file_stem).as_bytes(),
            )?;
            println!("{}", "  ✨ Created 'users' table migration.".green());
        }
        UsersTable::ExtendStarter(starter) => {
            let file_stem = format!("m{timestamp}_add_account_columns_to_users_table");
            write_new(
                &migrations_dir.join(format!("{file_stem}.rs")),
                render_account_columns_migration(&file_stem).as_bytes(),
            )?;
            println!(
                "{}",
                format!(
                    "  ✨ Kept the starter's {} and added a migration with the account columns of 'users'.",
                    crate::generators::slash_path(starter)
                )
                .green()
            );
        }
    }

    regenerate_migrations_mod()?;

    // 2. Create User Model
    let models_dir = Path::new("src/models");
    fs::create_dir_all(models_dir)?;
    let model_path = models_dir.join("user.rs");
    write_new(
        &model_path,
        with_session_epoch_methods(USER_MODEL_SOURCE).as_bytes(),
    )?;
    println!("{}", "  ✨ Created 'User' model.".green());
    register_mod_ast(&models_dir.join("mod.rs"), "user")?;

    Ok(())
}

/// The `users` migration written by `make:auth`.
pub(super) fn user_migration_source(file_stem: &str) -> String {
    format!(
        r##"use rullst::db::{{Orm, sqlx}};
use rullst::db::schema::{{ColumnDefault, Schema, Migration}};
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
            // Grants Nexus under the application's sign-in; see the User model.
            table.boolean("is_admin").not_null().default(ColumnDefault::Integer(0));
            // Bumped to revoke every login cookie of the account.
            table.big_integer("session_epoch").not_null().default(ColumnDefault::Integer(0));
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
    fn users_store_a_session_epoch_that_orm_writes_never_touch() {
        let migration = user_migration_source("m20260101000000_create_users_table");
        assert!(migration.contains("use rullst::db::schema::{ColumnDefault, Schema, Migration};"));
        assert!(migration.contains(
            "table.big_integer(\"session_epoch\").not_null().default(ColumnDefault::Integer(0));"
        ));
        let model = with_session_epoch_methods(USER_MODEL_SOURCE);
        assert!(!model.contains(SESSION_EPOCH_MARKER));
        // A database-managed column: read and filterable, never written.
        assert!(model.contains("    #[orm(db_default)]\n    pub session_epoch: i64,\n"));
        // Dirty tracking: `save()` writes only the columns a request changed.
        assert!(model.starts_with("use rullst::db::{Orm, RowSnapshot};\n"));
        assert!(model.contains("    #[sqlx(skip)]\n    pub snapshot: RowSnapshot,\n}"));
        assert!(model.contains("    #[orm(db_default)]\n    pub is_admin: bool,\n"));
        assert!(!model.contains("#[orm(skip)]"));
        assert!(model.contains("#[derive(Debug, Clone, Default, Orm)]"));
        assert!(model.contains("pub fn current_session_epoch(&self) -> Option<u64> {"));
        assert!(
            model.contains(
                "pub async fn revoke_sessions(id: i32) -> Result<(), rullst::orm::Error> {"
            )
        );
        // revoke_sessions and change_password, each in both dialects.
        assert_eq!(
            model
                .matches("session_epoch = session_epoch + 1 WHERE id = ")
                .count(),
            4
        );
        assert!(!USER_MODEL_SOURCE.contains("UPDATE users"));
        assert_eq!(
            with_session_epoch_methods("a__RULLST_SESSION_EPOCH_METHODS__b"),
            format!("a{SESSION_EPOCH_METHODS}b")
        );
    }

    #[test]
    fn password_changes_bump_the_epoch_only_from_the_asking_session() {
        let model = with_session_epoch_methods(USER_MODEL_SOURCE);
        assert!(model.contains(
            "\"UPDATE users SET password_hash = ?, updated_at = ?, session_epoch = session_epoch + 1 WHERE id = ? AND session_epoch = ?\""
        ));
        assert!(model.contains(
            "\"UPDATE users SET password_hash = $1, updated_at = $2, session_epoch = session_epoch + 1 WHERE id = $3 AND session_epoch = $4\""
        ));
        assert!(model.contains(
            "let (Ok(current), Some(next)) = (i64::try_from(epoch), epoch.checked_add(1)) else {\n            return Ok(None);"
        ));
        assert!(model.contains("            .bind(current)\n"));
        assert!(model.contains("Ok((changed == 1).then_some(next))"));
    }

    #[test]
    fn account_sources_name_the_orm_through_the_rullst_facade() {
        // `cargo rullst auth` enables the `orm` feature but adds no direct
        // `rullst-orm` dependency.
        let migration = user_migration_source("m20260101000000_create_users_table");
        let controller = crate::generators::auth::controllers::render_auth_controller(None);
        let model = with_session_epoch_methods(USER_MODEL_SOURCE);
        for source in [model.as_str(), migration.as_str(), controller.as_str()] {
            assert!(!source.contains("rullst_orm"), "{source}");
            syn::parse_file(source).expect("account source must parse");
        }
        assert!(USER_MODEL_SOURCE.contains("Result<Option<Self>, rullst::orm::Error>"));
        assert!(migration.contains("Result<(), rullst::orm::Error>"));
    }
}
