// src/blueprints/mod.rs — Root of the blueprints module.
#![cfg_attr(mutants, mutants::skip)]

use std::fs;
use std::path::Path;

pub mod blank;
pub mod common;
pub mod deploy;
pub mod k8s;
mod moved;
pub mod saas;
mod security_tests;

pub use moved::MovedBlueprint;

/// Public blueprint IDs are an on-disk/CLI compatibility contract. Never reorder
/// them. IDs 1 (LMS), 3 (Blog), 4 (Portfolio) and 5 (ERP) belonged to the
/// blueprints that moved to the repository's `examples/` in 13.0.0-alpha.4
/// ([`MovedBlueprint`]); they are never reused.
pub const BLANK_BLUEPRINT_ID: usize = 0;
pub const SAAS_BLUEPRINT_ID: usize = 2;

#[allow(clippy::too_many_arguments)]
pub fn apply(
    id: usize,
    path: &Path,
    project_name: &str,
    project_name_safe: &str,
    api: bool,
    hot_reload: bool,
    db_needed: bool,
    orm_pattern: &str,
    frontend_engine: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let manifest = match id {
        BLANK_BLUEPRINT_ID => blank::file_manifest(
            project_name,
            project_name_safe,
            api,
            hot_reload,
            db_needed,
            orm_pattern,
            frontend_engine,
        ),
        SAAS_BLUEPRINT_ID => {
            saas::file_manifest(project_name_safe, hot_reload, orm_pattern, frontend_engine)
        }
        _ => {
            if let Some(moved) = MovedBlueprint::from_id(id) {
                return Err(moved.into());
            }
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("unknown blueprint ID {id}"),
            )
            .into());
        }
    };

    for (rel_path, content) in manifest {
        let full_path = path.join(rel_path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&full_path, content)?;
    }

    if !api {
        let static_dir = path.join("static");
        fs::create_dir_all(&static_dir)?;
        fs::write(static_dir.join("rullst.png"), blank::BLANK_FAVICON)?;
        fs::write(
            static_dir.join("htmx-1.9.12.min.js"),
            include_bytes!("assets/htmx-1.9.12.min.js"),
        )?;
        fs::write(
            static_dir.join("HTMX-LICENSE"),
            include_bytes!("assets/HTMX-LICENSE"),
        )?;
    }

    Ok(())
}

#[cfg(test)]
mod dirty_tracking_tests;
#[cfg(test)]
mod monitor_tests;
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sst_blueprint_ids_remain_stable() {
        assert_eq!(BLANK_BLUEPRINT_ID, 0);
        assert_eq!(SAAS_BLUEPRINT_ID, 2);
    }

    #[test]
    fn stable_ids_select_the_expected_manifest() {
        let saas = saas::file_manifest("demo", false, "Active Record", "Zero-Bundle HTMX");
        assert!(
            saas.iter()
                .any(|(path, _)| *path == "src/models/subscription.rs")
        );
    }

    #[test]
    fn retired_ids_point_to_their_examples_without_writing() {
        for (id, name) in [(1, "lms"), (3, "blog"), (4, "portfolio"), (5, "erp")] {
            let root = tempfile::tempdir().expect("temporary blueprint directory");
            let target = root.path().join("app");
            let error = apply(
                id,
                &target,
                "demo",
                "demo",
                false,
                false,
                true,
                "Active Record",
                "Zero-Bundle HTMX",
            )
            .expect_err("a retired blueprint ID");
            let moved = error
                .downcast_ref::<MovedBlueprint>()
                .unwrap_or_else(|| panic!("{id}: {error}"));
            assert_eq!(moved.name(), name);
            assert!(!target.exists(), "{id} wrote files");
        }
    }

    /// Every SQLx-backed blueprint manifest, in both hot-reload layouts.
    fn sqlx_blueprint_manifests() -> Vec<(&'static str, Vec<(&'static str, String)>)> {
        let (orm, frontend) = ("Active Record", "Zero-Bundle HTMX");
        let mut manifests = Vec::new();
        for hot in [false, true] {
            manifests.extend([
                (
                    "blank",
                    blank::file_manifest("demo", "demo", false, hot, true, orm, frontend),
                ),
                ("saas", saas::file_manifest("demo", hot, orm, frontend)),
            ]);
        }
        manifests
    }

    #[test]
    fn embedded_templates_keep_lf_line_endings() {
        // `include_str!` embeds the checked-out bytes. A CRLF checkout (Git
        // for Windows' `core.autocrlf=true`) would emit CRLF projects and break
        // every multi-line source assertion; `.gitattributes` forces LF.
        for (blueprint, manifest) in sqlx_blueprint_manifests() {
            for (path, source) in manifest {
                assert!(
                    !source.contains('\r'),
                    "{blueprint}:{path} contains CR; check out templates with LF (.gitattributes)"
                );
            }
        }
    }

    #[test]
    fn sqlx_starters_register_their_migrations_without_enabling_them() {
        // `RULLST_AUTO_MIGRATE=true` needs the registry; the default stays off.
        for (blueprint, manifest) in sqlx_blueprint_manifests() {
            let main = manifest
                .iter()
                .find(|(path, _)| *path == "src/main.rs")
                .map(|(_, source)| source.as_str())
                .expect("a main.rs");
            assert_eq!(
                main.matches("        .with_migrations(crate::migrations::get_migrations())\n")
                    .count(),
                1,
                "{blueprint}"
            );
            assert!(!main.contains("auto_migrate"), "{blueprint}");
        }
    }

    #[test]
    fn sqlx_blueprints_avoid_sqlite_only_seed_functions() {
        // `datetime('now')` exists only in SQLite and fails `db:migrate` on
        // PostgreSQL/MySQL/MariaDB; `timestamps()` columns already default to
        // CURRENT_TIMESTAMP on every driver.
        for (blueprint, manifest) in sqlx_blueprint_manifests() {
            for (path, source) in manifest {
                assert!(
                    !source.contains("datetime("),
                    "{blueprint}:{path} uses SQLite-only datetime()"
                );
            }
        }
    }

    #[test]
    fn indexed_string_columns_are_bounded_for_mysql() {
        // MySQL/MariaDB reject an index on a TEXT column without a prefix
        // length, and `string()`/`timestamps()` columns are TEXT.
        let mut indexed_strings = 0;
        for (blueprint, manifest) in sqlx_blueprint_manifests() {
            for (path, source) in manifest {
                for statement in source.split("CREATE ").skip(1) {
                    let Some((_, definition)) = statement
                        .split_once("INDEX ")
                        .filter(|(kind, _)| kind.is_empty() || *kind == "UNIQUE ")
                    else {
                        continue;
                    };
                    let Some(columns) = definition
                        .split_once('(')
                        .and_then(|(_, columns)| columns.split_once(')'))
                        .map(|(columns, _)| columns)
                    else {
                        continue;
                    };
                    for column in columns.split(',').map(str::trim) {
                        assert!(
                            !matches!(column, "created_at" | "updated_at"),
                            "{blueprint}:{path} indexes TEXT timestamp {column}"
                        );
                        let declaration = format!("table.string(\"{column}\")");
                        for line in source.lines().filter(|line| line.contains(&declaration)) {
                            indexed_strings += 1;
                            assert!(
                                line.contains(".col_type = \"VARCHAR("),
                                "{blueprint}:{path} indexes TEXT column {column}"
                            );
                        }
                    }
                }
            }
        }
        assert!(
            indexed_strings > 0,
            "no indexed string column was inspected"
        );
    }

    #[test]
    fn credential_submissions_are_rate_limited_in_auth_starters() {
        for (blueprint, manifest) in sqlx_blueprint_manifests() {
            if blueprint != "saas" {
                continue;
            }
            let routers = manifest
                .iter()
                .filter(|(path, _)| matches!(*path, "src/main.rs" | "src/lib.rs"))
                .map(|(_, source)| source.as_str())
                .collect::<String>();
            for (path, handler) in [("/login", "login_submit"), ("/register", "register_submit")] {
                assert!(
                    routers.contains(&format!(
                        ".route(\"{path}\", rullst::routing::post(controllers::auth_controller::{handler})\n        .layer(rullst::server::from_fn(controllers::auth_controller::credential_rate_limit)))"
                    )),
                    "{blueprint}: {path} submissions must be rate limited"
                );
                assert!(
                    !routers.contains(&format!("post(\"{path}\" =>")),
                    "{blueprint}: unthrottled {path} route"
                );
            }
        }
    }

    #[test]
    fn auth_starters_keep_the_password_hash_out_of_model_json() {
        // `to_json()` feeds audit rows and the post-commit Redis model events,
        // and it omits only `#[orm(hidden)]` fields.
        for (blueprint, manifest) in sqlx_blueprint_manifests() {
            if blueprint != "saas" {
                continue;
            }
            let user = manifest
                .iter()
                .find_map(|(path, source)| (*path == "src/models/user.rs").then_some(source))
                .unwrap_or_else(|| panic!("{blueprint} emits no User model"));
            assert!(
                user.contains("#[orm(hidden)]\n    pub password_hash: Option<String>,"),
                "{blueprint}: password_hash must be #[orm(hidden)]"
            );
        }
    }

    #[test]
    fn auth_starters_revoke_login_cookies_by_session_epoch() {
        // A stateless login cookie stayed valid for 30 days after logout or
        // a password change; the epoch lets the account end every copy.
        let file = |manifest: &[(&str, String)], wanted: &str| {
            manifest
                .iter()
                .find_map(|(path, source)| (*path == wanted).then(|| source.clone()))
                .unwrap_or_else(|| panic!("missing {wanted}"))
        };
        for (blueprint, manifest) in sqlx_blueprint_manifests() {
            if blueprint != "saas" {
                continue;
            }
            let user = file(&manifest, "src/models/user.rs");
            syn::parse_file(&user).unwrap_or_else(|error| panic!("{blueprint}: {error}"));
            assert!(!user.contains("__RULLST"), "{blueprint}: unresolved marker");
            assert!(user.contains("    #[orm(db_default)]\n    pub session_epoch: i64,\n"));
            assert!(
                !user.contains("#[orm(skip)]"),
                "{blueprint}: skip is not a column"
            );
            assert!(user.contains("#[derive(Debug, Clone, Default, FromRow, Orm)]"));
            assert!(user.contains(
                "pub fn accepts_session(&self, claims: &rullst::auth::SessionClaims) -> bool {"
            ));
            assert!(user.contains(
                "pub async fn revoke_sessions(id: i32) -> Result<(), rullst::orm::Error> {"
            ));
            assert!(
                user.contains(
                    "\"UPDATE users SET session_epoch = session_epoch + 1 WHERE id = $1\""
                )
            );
            assert!(
                user.contains(
                    "\"UPDATE users SET session_epoch = session_epoch + 1 WHERE id = ?\""
                )
            );

            let middleware = file(&manifest, "src/middlewares/auth_middleware.rs");
            assert!(middleware.contains("rullst::auth::decrypt_session_claims(&cookie, &app_key)"));
            assert!(middleware.contains("Ok(Some(user)) if user.accepts_session(&claims) => {"));
            assert!(!middleware.contains("decrypt_session(&"), "{blueprint}");

            let migration = manifest
                .iter()
                .filter(|(path, source)| {
                    path.starts_with("src/migrations/")
                        && source.contains("Schema::create(\"users\"")
                })
                .map(|(_, source)| source.as_str())
                .collect::<Vec<_>>();
            assert_eq!(migration.len(), 1, "{blueprint}: one users migration");
            assert!(migration[0].contains(
                "table.big_integer(\"session_epoch\").not_null().default(ColumnDefault::Integer(0));"
            ));

            let routers = manifest
                .iter()
                .filter(|(path, _)| matches!(*path, "src/main.rs" | "src/lib.rs"))
                .map(|(_, source)| source.as_str())
                .collect::<String>();
            let route = ".route(\"/logout/everywhere\", rullst::routing::post(controllers::auth_controller::logout_everywhere)\n        .layer(rullst::server::from_fn(middlewares::auth_middleware::auth_middleware)))";
            assert!(
                routers.contains(route),
                "{blueprint}: unmounted sign-out everywhere"
            );
            assert!(
                file(&manifest, "src/pages/auth.rs").contains("action=\"/logout/everywhere\""),
                "{blueprint}: no sign-out everywhere form"
            );
        }
    }

    #[test]
    fn unknown_blueprint_id_is_not_silently_scaffolded_as_blank() {
        let root = std::env::temp_dir().join(format!(
            "rullst-unknown-blueprint-{}",
            rand::random::<u64>()
        ));
        let result = apply(
            usize::MAX,
            &root,
            "demo",
            "demo",
            false,
            false,
            false,
            "Active Record",
            "Zero-Bundle HTMX",
        );
        assert!(result.is_err());
        assert!(!root.exists());
    }

    #[test]
    fn html_blueprints_write_the_official_favicon() {
        for blueprint in [BLANK_BLUEPRINT_ID, SAAS_BLUEPRINT_ID] {
            let root = tempfile::tempdir().expect("temporary blueprint directory");
            apply(
                blueprint,
                root.path(),
                "demo",
                "demo",
                false,
                false,
                blueprint == BLANK_BLUEPRINT_ID,
                "Active Record",
                "Zero-Bundle HTMX",
            )
            .expect("HTML blueprint");

            let favicon =
                fs::read(root.path().join("static/rullst.png")).expect("blueprint favicon");
            assert_eq!(favicon, blank::BLANK_FAVICON);
            assert!(favicon.starts_with(&[0x89, b'P', b'N', b'G']));
            let script = fs::read(root.path().join("static/htmx-1.9.12.min.js"))
                .expect("offline HTMX client must be materialized");
            assert_eq!(script, include_bytes!("assets/htmx-1.9.12.min.js"));
            assert!(root.path().join("static/HTMX-LICENSE").is_file());
        }
    }
}
