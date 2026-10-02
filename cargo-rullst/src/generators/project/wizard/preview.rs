//! The file list a plan generates, taken from the real project writers: the
//! plan is rendered quietly into a private temporary directory, listed and
//! removed. Nothing is written to the destination, nothing is built and no
//! network is used.

use super::plan::ProjectPlan;
use crate::generators::project::{ProjectIdentity, create};
use std::path::Path;

/// The relative `/`-separated files `plan` would create, sorted; `None` when
/// the preview could not be rendered (for example an unwritable temp dir).
pub(crate) fn rendered_files(plan: &ProjectPlan) -> Option<Vec<String>> {
    let identity = ProjectIdentity::from_destination(&plan.name).ok()?;
    let staging = tempfile::Builder::new()
        .prefix("rullst-new-preview-")
        .tempdir()
        .ok()?;
    let root = staging.path().join(identity.package_name());
    std::fs::create_dir(&root).ok()?;
    let current_dir = std::env::current_dir().ok()?;
    let options = plan.wizard_options();
    create::write_project_files(
        &root,
        plan,
        &options,
        &identity,
        &current_dir,
        create::Report::Quiet,
    )
    .ok()?;
    create::write_late_files(&root, plan, &identity, create::Report::Quiet).ok()?;
    list_files(&root)
}

/// Regular files below `root` as sorted relative `/` paths. Links are not
/// followed.
pub(crate) fn list_files(root: &Path) -> Option<Vec<String>> {
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(root).follow_links(false) {
        let entry = entry.ok()?;
        if !entry.file_type().is_file() {
            continue;
        }
        let relative = entry.path().strip_prefix(root).ok()?;
        let parts: Vec<String> = relative
            .components()
            .map(|part| part.as_os_str().to_string_lossy().into_owned())
            .collect();
        files.push(parts.join("/"));
    }
    files.sort();
    Some(files)
}

#[cfg(test)]
#[path = "preview_tests.rs"]
mod tests;
