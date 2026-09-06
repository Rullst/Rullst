// src/blueprints/mod.rs — Root of the blueprints module.
#![cfg_attr(mutants, mutants::skip)]

use std::fs;
use std::path::Path;

pub mod blank;
pub mod blog;
pub mod common;
pub mod deploy;
pub mod erp;
pub mod k8s;
pub mod lms;
pub mod portfolio;
pub mod saas;

/// Public blueprint IDs are an on-disk/CLI compatibility contract. Never reorder them.
pub const BLANK_BLUEPRINT_ID: usize = 0;
pub const LMS_BLUEPRINT_ID: usize = 1;
pub const SAAS_BLUEPRINT_ID: usize = 2;
pub const BLOG_BLUEPRINT_ID: usize = 3;
pub const PORTFOLIO_BLUEPRINT_ID: usize = 4;
pub const ERP_BLUEPRINT_ID: usize = 5;

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
    apply_with_lms_modules(
        id,
        path,
        project_name,
        project_name_safe,
        api,
        hot_reload,
        db_needed,
        orm_pattern,
        frontend_engine,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn apply_with_lms_modules(
    id: usize,
    path: &Path,
    project_name: &str,
    project_name_safe: &str,
    api: bool,
    hot_reload: bool,
    db_needed: bool,
    orm_pattern: &str,
    frontend_engine: &str,
    lms_modules: Option<&[lms::LmsModule]>,
) -> Result<(), Box<dyn std::error::Error>> {
    if id != LMS_BLUEPRINT_ID && lms_modules.is_some() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "LMS modules may only be selected with the LMS blueprint",
        )
        .into());
    }
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
        LMS_BLUEPRINT_ID => match lms_modules {
            Some(modules) => lms::file_manifest_for_modules(
                project_name_safe,
                hot_reload,
                orm_pattern,
                frontend_engine,
                modules,
            )?,
            None => lms::file_manifest(project_name_safe, hot_reload, orm_pattern, frontend_engine),
        },
        SAAS_BLUEPRINT_ID => {
            saas::file_manifest(project_name_safe, hot_reload, orm_pattern, frontend_engine)
        }
        BLOG_BLUEPRINT_ID => {
            blog::file_manifest(project_name_safe, hot_reload, orm_pattern, frontend_engine)
        }
        PORTFOLIO_BLUEPRINT_ID => {
            portfolio::file_manifest(project_name_safe, hot_reload, orm_pattern, frontend_engine)
        }
        ERP_BLUEPRINT_ID => {
            erp::file_manifest(project_name_safe, hot_reload, orm_pattern, frontend_engine)
        }
        _ => {
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
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sst_blueprint_ids_remain_stable() {
        assert_eq!(BLANK_BLUEPRINT_ID, 0);
        assert_eq!(LMS_BLUEPRINT_ID, 1);
        assert_eq!(SAAS_BLUEPRINT_ID, 2);
        assert_eq!(BLOG_BLUEPRINT_ID, 3);
        assert_eq!(PORTFOLIO_BLUEPRINT_ID, 4);
        assert_eq!(ERP_BLUEPRINT_ID, 5);
    }

    #[test]
    fn stable_ids_select_the_expected_manifest() {
        let lms = lms::file_manifest("demo", false, "Active Record", "Zero-Bundle HTMX");
        let saas = saas::file_manifest("demo", false, "Active Record", "Zero-Bundle HTMX");
        let blog = blog::file_manifest("demo", false, "Active Record", "Zero-Bundle HTMX");

        assert!(lms.iter().any(|(path, _)| *path == "src/models/course.rs"));
        for academy_path in [
            "src/models/course_module.rs",
            "src/models/quiz.rs",
            "src/models/activity.rs",
            "src/models/achievement.rs",
            "src/models/leaderboard_entry.rs",
            "src/models/automation_rule.rs",
            "src/models/score_event.rs",
            "src/models/score_correction.rs",
            "src/models/domain_event.rs",
        ] {
            assert!(lms.iter().any(|(path, _)| *path == academy_path));
        }
        assert!(lms.iter().all(|(_, source)| !source.contains("datetime(")));
        assert!(
            saas.iter()
                .any(|(path, _)| *path == "src/models/subscription.rs")
        );
        assert!(blog.iter().any(|(path, _)| *path == "src/models/post.rs"));
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
        for blueprint in [BLANK_BLUEPRINT_ID, SAAS_BLUEPRINT_ID, ERP_BLUEPRINT_ID] {
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
        }
    }
}
