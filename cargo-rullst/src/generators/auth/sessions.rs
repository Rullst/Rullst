// cargo-rullst/src/generators/auth/sessions.rs — The login-session registry
// feature of `cargo rullst auth`.

use std::io::{Error as IoError, ErrorKind};

const MIGRATION_TEMPLATE: &str = include_str!("sessions_migration.rs.template");

/// File name suffix of the `rullst_auth_sessions` migration.
pub(crate) const MIGRATION_SUFFIX: &str = "_create_rullst_auth_sessions_table.rs";

/// The reversible `rullst_auth_sessions` migration named `migration_name`:
/// `up` creates the registry's table and indexes with `user_id` referencing
/// `users (id)` `ON DELETE CASCADE`, so deleting an account deletes its
/// sessions, and `down` drops them. Its name must sort after the migration
/// that creates `users`.
pub(crate) fn render_sessions_migration(migration_name: &str) -> String {
    MIGRATION_TEMPLATE.replace("__MIGRATION_NAME__", migration_name)
}

/// Writes the `rullst_auth_sessions` migration named after `timestamp` and
/// refreshes the migration registry.
pub(crate) fn generate_sessions_migration(
    timestamp: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let migration_name = format!("m{timestamp}{}", MIGRATION_SUFFIX.trim_end_matches(".rs"));
    crate::generators::output_guard::write_new(
        &std::path::Path::new("src/migrations").join(format!("{migration_name}.rs")),
        render_sessions_migration(&migration_name).as_bytes(),
    )?;
    crate::generators::migration::regenerate_migrations_mod()?;
    println!(
        "{}",
        colored::Colorize::green(
            "  ✨ Created 'rullst_auth_sessions' table migration (login sessions)."
        )
    );
    Ok(())
}

/// The `rullst` feature of the session registry over the project's strict
/// database profile (the one `cargo rullst new --database ...` selects).
pub(crate) fn registry_feature(manifest: &str) -> Result<&'static str, IoError> {
    let invalid = |message: &str| IoError::new(ErrorKind::InvalidInput, message.to_owned());
    let manifest: toml::Table =
        toml::from_str(manifest).map_err(|_| invalid("Cargo.toml is not valid TOML"))?;
    let dependencies = manifest.get("dependencies").and_then(toml::Value::as_table);
    let mut selected = Vec::new();
    for name in ["rullst", "rullst-orm"] {
        let features = dependencies
            .and_then(|dependencies| dependencies.get(name))
            .and_then(toml::Value::as_table)
            .and_then(|dependency| dependency.get("features"))
            .and_then(toml::Value::as_array);
        for feature in features
            .into_iter()
            .flatten()
            .filter_map(toml::Value::as_str)
        {
            let registry = match feature {
                "strict-sqlite" => "auth-session-registry-sqlite",
                "strict-postgres" => "auth-session-registry-postgres",
                "strict-mysql" => "auth-session-registry-mysql",
                _ => continue,
            };
            if !selected.contains(&registry) {
                selected.push(registry);
            }
        }
    }
    match selected.as_slice() {
        [registry] => Ok(registry),
        _ => Err(invalid(
            "cargo rullst auth keeps login sessions in the application database and needs exactly one strict database profile (strict-sqlite, strict-postgres or strict-mysql), as `cargo rullst new --database ...` selects",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_migration_owns_the_sessions_table() {
        let migration =
            render_sessions_migration("m20260601000003_create_rullst_auth_sessions_table");
        syn::parse_file(&migration).expect("sessions migration must parse");
        assert!(!migration.contains("__MIGRATION_NAME__"));
        assert!(migration.contains("\"m20260601000003_create_rullst_auth_sessions_table\""));
        assert!(migration.contains(
            "rullst::auth::session_registry::SessionRegistryFor::<rullst::orm::RullstPool>::create_table(\n            Orm::pool()?,\n        )"
        ));
        assert!(migration.contains("Schema::drop_if_exists(\"rullst_auth_sessions\").await"));
        assert_eq!(MIGRATION_SUFFIX, "_create_rullst_auth_sessions_table.rs");
    }

    /// The SQLite statement of the migration's table.
    fn sqlite_table(migration: &str) -> &str {
        let start = migration
            .find("\"CREATE TABLE IF NOT EXISTS rullst_auth_sessions (")
            .expect("the migration creates the SQLite table");
        let rest = &migration[start + 1..];
        &rest[..rest.find('"').expect("closing quote")]
    }

    /// Deleting an account deletes its sessions (#827): SQLite gets the key
    /// in the table it creates before the registry adds its indexes, and
    /// PostgreSQL and MySQL/MariaDB get it added to the registry's table.
    #[test]
    fn the_sessions_reference_users_on_delete_cascade() {
        let migration =
            render_sessions_migration("m20260601000003_create_rullst_auth_sessions_table");
        assert!(
            sqlite_table(&migration)
                .contains("user_id INTEGER NOT NULL REFERENCES users (id) ON DELETE CASCADE,")
        );
        let sqlite_first = migration
            .find("sqlx::query(SQLITE_TABLE)")
            .expect("SQLite creates its table");
        let registry = migration.find("::create_table(").expect("registry table");
        let key = migration
            .find(".foreign(\"user_id\")\n                    .references(\"users\", \"id\")\n                    .on_delete(ForeignKeyAction::Cascade);")
            .expect("the other databases add the key");
        assert!(sqlite_first < registry && registry < key, "{migration}");
        assert!(migration.contains("if sqlite {\n            sqlx::query(SQLITE_TABLE)"));
        assert!(migration.contains(
            "if !sqlite {\n            // PostgreSQL and MySQL/MariaDB add the key to the table."
        ));
        assert!(migration.contains("Schema::table(\"rullst_auth_sessions\", |table| {"));
    }

    /// The migration's SQLite table is the registry's, with the key added,
    /// so the registry's `CREATE TABLE IF NOT EXISTS` keeps it.
    #[test]
    fn the_sqlite_table_is_the_registrys_with_the_key() {
        let registry = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../rullst-auth/src/auth/session_registry/sqlite.rs");
        // A packaged crate has no sibling rullst-auth source.
        let Ok(source) = std::fs::read_to_string(registry) else {
            return;
        };
        let migration =
            render_sessions_migration("m20260601000003_create_rullst_auth_sessions_table");
        let with_key = sqlite_table(&migration);
        let without_key = with_key.replace(" REFERENCES users (id) ON DELETE CASCADE", "");
        assert_ne!(with_key, without_key);
        assert!(
            source.contains(&format!("\"{without_key}\"")),
            "the SQLite table of rullst-auth changed; update sessions_migration.rs.template:\n{without_key}"
        );
    }

    fn manifest(rullst: &str, orm: &str) -> String {
        format!(
            "[package]\nname = \"demo\"\n\n[dependencies]\nrullst = {{ version = \"13\", features = [{rullst}] }}\nrullst-orm = {{ version = \"13\", features = [{orm}] }}\n"
        )
    }

    #[test]
    fn the_registry_follows_the_strict_profile() {
        for (rullst, orm, feature) in [
            (
                "\"orm\", \"strict-sqlite\"",
                "\"strict-sqlite\"",
                "auth-session-registry-sqlite",
            ),
            (
                "\"orm\"",
                "\"strict-postgres\"",
                "auth-session-registry-postgres",
            ),
            ("\"strict-mysql\"", "", "auth-session-registry-mysql"),
        ] {
            assert_eq!(registry_feature(&manifest(rullst, orm)).unwrap(), feature);
        }
    }

    #[test]
    fn no_or_several_strict_profiles_are_refused() {
        for (rullst, orm) in [
            ("\"orm\", \"drivers-all\"", ""),
            ("\"strict-sqlite\"", "\"strict-postgres\""),
            ("", ""),
        ] {
            let error = registry_feature(&manifest(rullst, orm)).unwrap_err();
            assert_eq!(error.kind(), ErrorKind::InvalidInput);
            assert!(
                error
                    .to_string()
                    .contains("exactly one strict database profile")
            );
        }
        assert!(registry_feature("[package]\nname = \"demo\"\n").is_err());
        assert_eq!(
            registry_feature("not toml [").unwrap_err().to_string(),
            "Cargo.toml is not valid TOML"
        );
    }
}
