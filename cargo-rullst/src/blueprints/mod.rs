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
        0 => blank::file_manifest(
            project_name,
            project_name_safe,
            api,
            hot_reload,
            db_needed,
            orm_pattern,
            frontend_engine,
        ),
        1 => portfolio::file_manifest(project_name_safe, hot_reload, orm_pattern, frontend_engine),
        2 => lms::file_manifest(project_name_safe, hot_reload, orm_pattern, frontend_engine),
        3 => saas::file_manifest(project_name_safe, hot_reload, orm_pattern, frontend_engine),
        4 => blog::file_manifest(project_name_safe, hot_reload, orm_pattern, frontend_engine),
        5 => erp::file_manifest(project_name_safe, hot_reload, orm_pattern, frontend_engine),
        _ => blank::file_manifest(
            project_name,
            project_name_safe,
            api,
            hot_reload,
            db_needed,
            orm_pattern,
            frontend_engine,
        ),
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
