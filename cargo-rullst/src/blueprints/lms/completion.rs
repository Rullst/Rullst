// Version-pinned course completion and publicly verifiable certificate templates.

#[path = "completion_controller.rs"]
mod completion_controller;
#[path = "completion_migration.rs"]
mod completion_migration;
#[path = "completion_models.rs"]
mod completion_models;
#[path = "completion_service.rs"]
mod completion_service;

pub fn get_files() -> Vec<(&'static str, String)> {
    let mut files = completion_models::get_files();
    files.extend(completion_service::get_files());
    files.extend(completion_migration::get_files());
    files.extend(completion_controller::get_files());
    files
}
