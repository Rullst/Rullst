// src/blueprints/blank.rs — Blank Starter blueprint: an HTMX page or JSON API with
// optional SQLx or Turso persistence and hot reload.
// The templates under `blank/src/` mirror the generated project's layout; the
// snippets under `blank/fragments/` are spliced into them.
use super::common;

mod client;
mod turso;

const BLANK_STYLES: &str = include_str!("blank/styles.css");
pub(super) const BLANK_FAVICON: &[u8] = include_bytes!("blank/rullst.png");

/// Files the SQLx database profile adds, in manifest order.
const MIGRATION_FILES: [(&str, &str); 2] = [
    (
        "src/migrations/m20260601000000_create_users_table.rs",
        include_str!("blank/src/migrations/m20260601000000_create_users_table.rs.template"),
    ),
    (
        "src/migrations/mod.rs",
        include_str!("blank/src/migrations/mod.rs.template"),
    ),
];

pub fn file_manifest(
    project_name: &str,
    project_name_safe: &str,
    api: bool,
    hot_reload: bool,
    db_needed: bool,
    orm_pattern: &str,
    frontend_engine: &str,
) -> Vec<(&'static str, String)> {
    let _ = project_name;
    let is_repo = common::is_repo_mode(orm_pattern);
    let turso_primary = orm_pattern == "Turso Active Record";

    let db_model_code = if turso_primary {
        include_str!("blank/fragments/db_model.turso.rs.template")
    } else if db_needed {
        include_str!("blank/fragments/db_model.rs.template")
    } else {
        ""
    };
    // Every status fragment ends with a line break, so the template line holding
    // `__DB_STATUS_CODE__` is not followed by its own blank line.
    let db_status_code = if hot_reload && db_needed {
        include_str!("blank/fragments/db_status.hot.rs.template")
    } else if turso_primary {
        include_str!("blank/fragments/db_status.turso.rs.template")
    } else if db_needed {
        include_str!("blank/fragments/db_status.rs.template")
    } else {
        include_str!("blank/fragments/db_status.disabled.rs.template")
    };
    let artisan_call = if turso_primary {
        include_str!("blank/fragments/artisan.turso.rs.template")
    } else if db_needed {
        include_str!("blank/fragments/artisan.rs.template")
    } else {
        ""
    };
    let migrations_mod_declaration = if db_needed {
        "pub mod migrations;\n"
    } else {
        ""
    };
    let client_modules = if !api {
        "pub mod islands;\npub mod rpc;\n"
    } else {
        ""
    };
    let rpc_module = if !api { "mod rpc;\n" } else { "" };
    let fill = |template: &str| {
        template
            .replace("__MIGRATIONS_MOD_DECLARATION__", migrations_mod_declaration)
            .replace("__CLIENT_MODULES__", client_modules)
            .replace("__RPC_MODULE__", rpc_module)
            .replace("__DB_MODEL_CODE__", db_model_code)
            .replace("__DB_STATUS_CODE__", db_status_code)
            .replace("__ARTISAN_CALL__", artisan_call)
    };

    let mut manifest = Vec::new();
    // Caller-supplied values are substituted last so they are never re-expanded.
    if hot_reload {
        let lib_rs = if api {
            fill(include_str!("blank/src/lib.api.rs.template"))
        } else {
            fill(include_str!("blank/src/lib.rs.template")).replace(
                "__FE_IMPORTS__",
                &common::frontend_page_imports(frontend_engine),
            )
        };
        manifest.push(("src/lib.rs", lib_rs));

        if !api {
            let island_counter = include_str!("../generators/island.rs.template")
                .replace("__MODULE_NAME__", "counter")
                .replace("__TYPE_NAME__", "Counter");
            manifest.push((
                "src/islands/mod.rs",
                include_str!("blank/src/islands/mod.rs.template").to_string(),
            ));
            manifest.push(("src/islands/counter.rs", island_counter));
        }

        manifest.push((
            "src/main.rs",
            fill(include_str!("blank/src/main.hot.rs.template"))
                .replace("__PROJECT_NAME_SAFE__", project_name_safe),
        ));
    } else {
        let main_rs = if api {
            fill(include_str!("blank/src/main.api.rs.template"))
        } else {
            fill(include_str!("blank/src/main.rs.template"))
        };
        manifest.push(("src/main.rs", main_rs));
    }

    if !api {
        manifest.push(("static/rullst.css", BLANK_STYLES.to_string()));
        manifest.push(("src/rpc.rs", client::rpc_source()));
    }

    if turso_primary {
        manifest.extend(turso::migration_files());
    } else if db_needed {
        manifest.extend(
            MIGRATION_FILES
                .iter()
                .map(|(path, template)| (*path, (*template).to_string())),
        );

        if is_repo {
            manifest.push((
                "src/repositories/user_repository.rs",
                common::generate_repository("User", "users"),
            ));
            manifest.push((
                "src/repositories/mod.rs",
                common::generate_repositories_mod(&["User"]),
            ));
        }
    }

    manifest
}
