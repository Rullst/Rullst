// cargo-rullst/src/blueprints/saas/routes.rs — Main router and application entrypoint for SaaS blueprint.
// The entrypoint templates live under `src/` beside this module.

use crate::blueprints::common;

pub fn get_routes(
    project_name_safe: &str,
    hot_reload: bool,
    orm_pattern: &str,
) -> Vec<(&'static str, String)> {
    let repo_mod_decl = common::repo_mod_decl(orm_pattern);

    if hot_reload {
        vec![
            (
                "src/lib.rs",
                include_str!("src/lib.rs.template").replace("__REPO_MOD_DECL__", repo_mod_decl),
            ),
            // The caller-supplied name is substituted last so it is never re-expanded.
            (
                "src/main.rs",
                include_str!("src/main.hot.rs.template")
                    .replace("__REPO_MOD_DECL__", repo_mod_decl)
                    .replace("__PROJECT_NAME_SAFE__", project_name_safe),
            ),
        ]
    } else {
        vec![(
            "src/main.rs",
            include_str!("src/main.rs.template").replace("__REPO_MOD_DECL__", repo_mod_decl),
        )]
    }
}
