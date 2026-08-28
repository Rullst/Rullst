// src/blueprints/mod.rs — Root of the blueprints module.

use std::fs;
use std::path::Path;

pub mod blank;
pub mod blog;
pub mod erp;
pub mod lms;
pub mod portfolio;
pub mod saas;
pub mod uptime;

pub fn apply(
    id: usize,
    path: &Path,
    project_name: &str,
    project_name_safe: &str,
    api: bool,
    hot_reload: bool,
    db_needed: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let manifest = match id {
        0 => blank::file_manifest(project_name, project_name_safe, api, hot_reload, db_needed),
        1 => portfolio::file_manifest(project_name_safe, hot_reload),
        2 => lms::file_manifest(project_name_safe, hot_reload),
        3 => saas::file_manifest(project_name_safe, hot_reload),
        4 => blog::file_manifest(project_name_safe, hot_reload),
        5 => erp::file_manifest(project_name_safe, hot_reload),
        6 => uptime::file_manifest(project_name_safe, hot_reload),
        _ => blank::file_manifest(project_name, project_name_safe, api, hot_reload, db_needed),
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
