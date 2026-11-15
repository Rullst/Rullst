// cargo-rullst/src/generators/auth/sessions.rs — The login-session registry
// feature of `cargo rullst auth`.

use std::io::{Error as IoError, ErrorKind};

const MIGRATION_TEMPLATE: &str = include_str!("sessions_migration.rs.template");

/// File name suffix of the `rullst_auth_sessions` migration.
pub(crate) const MIGRATION_SUFFIX: &str = "_create_rullst_auth_sessions_table.rs";

/// The reversible `rullst_auth_sessions` migration named `migration_name`:
/// `up` creates the registry's table and indexes, `down` drops them.
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
