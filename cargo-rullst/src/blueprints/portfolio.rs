// src/blueprints/portfolio.rs — Portfolio blueprint: profile, projects, experience
// and skills with a Nexus CMS, in Active Record, Repository or Hybrid mode.
// The templates under `portfolio/src/` mirror the generated project's layout.
use super::common;

/// Files every portfolio variant emits unchanged.
const COMMON_FILES: [(&str, &str); 9] = [
    (
        "src/migrations/mod.rs",
        include_str!("portfolio/src/migrations/mod.rs.template"),
    ),
    (
        "src/migrations/m20260701000000_create_portfolio_tables.rs",
        include_str!(
            "portfolio/src/migrations/m20260701000000_create_portfolio_tables.rs.template"
        ),
    ),
    (
        "src/models/mod.rs",
        include_str!("portfolio/src/models/mod.rs.template"),
    ),
    (
        "src/models/profile.rs",
        include_str!("portfolio/src/models/profile.rs.template"),
    ),
    (
        "src/models/project.rs",
        include_str!("portfolio/src/models/project.rs.template"),
    ),
    (
        "src/models/experience.rs",
        include_str!("portfolio/src/models/experience.rs.template"),
    ),
    (
        "src/models/skill.rs",
        include_str!("portfolio/src/models/skill.rs.template"),
    ),
    (
        "src/controllers/mod.rs",
        include_str!("portfolio/src/controllers/mod.rs.template"),
    ),
    (
        "src/pages/mod.rs",
        include_str!("portfolio/src/pages/mod.rs.template"),
    ),
];

/// Files the Repository and Hybrid ORM modes add.
const REPOSITORY_FILES: [(&str, &str); 5] = [
    (
        "src/repositories/mod.rs",
        include_str!("portfolio/src/repositories/mod.rs.template"),
    ),
    (
        "src/repositories/profile_repository.rs",
        include_str!("portfolio/src/repositories/profile_repository.rs.template"),
    ),
    (
        "src/repositories/project_repository.rs",
        include_str!("portfolio/src/repositories/project_repository.rs.template"),
    ),
    (
        "src/repositories/experience_repository.rs",
        include_str!("portfolio/src/repositories/experience_repository.rs.template"),
    ),
    (
        "src/repositories/skill_repository.rs",
        include_str!("portfolio/src/repositories/skill_repository.rs.template"),
    ),
];

pub fn file_manifest(
    project_name_safe: &str,
    hot_reload: bool,
    orm_pattern: &str,
    frontend_engine: &str,
) -> Vec<(&'static str, String)> {
    let is_repo_mode = common::is_repo_mode(orm_pattern);
    let fill = |template: &str| {
        template
            .replace("__REPO_MOD_DECL__", common::repo_mod_decl(orm_pattern))
            .replace("__PROJECT_NAME_SAFE__", project_name_safe)
            .replace("__FRONTEND_ENGINE__", frontend_engine)
            .replace("__ORM_PATTERN__", orm_pattern)
            .replace(
                "__ENGINE_BADGE__",
                common::frontend_engine_badge(frontend_engine),
            )
    };

    let mut manifest = Vec::new();
    if hot_reload {
        manifest.push((
            "src/lib.rs",
            fill(include_str!("portfolio/src/lib.rs.template")),
        ));
        manifest.push((
            "src/main.rs",
            fill(include_str!("portfolio/src/main.hot.rs.template")),
        ));
    } else {
        manifest.push((
            "src/main.rs",
            fill(include_str!("portfolio/src/main.rs.template")),
        ));
    }
    manifest.extend(
        COMMON_FILES
            .iter()
            .map(|(path, template)| (*path, (*template).to_string())),
    );
    let controller = if is_repo_mode {
        manifest.extend(
            REPOSITORY_FILES
                .iter()
                .map(|(path, template)| (*path, (*template).to_string())),
        );
        include_str!("portfolio/src/controllers/portfolio_controller.repository.rs.template")
    } else {
        include_str!("portfolio/src/controllers/portfolio_controller.rs.template")
    };
    manifest.push((
        "src/controllers/portfolio_controller.rs",
        controller.to_string(),
    ));
    manifest.push((
        "src/pages/home.rs",
        fill(include_str!("portfolio/src/pages/home.rs.template")),
    ));
    manifest
}

#[cfg(test)]
#[path = "portfolio_tests.rs"]
mod tests;
