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
        LMS_BLUEPRINT_ID => {
            lms::file_manifest(project_name_safe, hot_reload, orm_pattern, frontend_engine)
        }
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
}
