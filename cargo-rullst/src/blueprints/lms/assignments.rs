// Human-graded assignment and rubric templates for the LMS starter.

#[path = "assignments_controller.rs"]
mod assignments_controller;
#[path = "assignments_correction.rs"]
mod assignments_correction;
#[path = "assignments_grade.rs"]
mod assignments_grade;
#[path = "assignments_migration.rs"]
mod assignments_migration;
#[path = "assignments_models.rs"]
mod assignments_models;
#[path = "assignments_submit.rs"]
mod assignments_submit;

pub fn get_files() -> Vec<(&'static str, String)> {
    let mut files = Vec::new();
    files.extend(assignments_models::get_files());
    files.extend(assignments_migration::get_files());
    files.extend(assignments_submit::get_files());
    files.extend(assignments_grade::get_files());
    files.extend(assignments_correction::get_files());
    files.extend(assignments_controller::get_files());
    files
}
