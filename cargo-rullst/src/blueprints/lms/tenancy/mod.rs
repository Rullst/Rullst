// Authenticated school/tenant boundaries for the generated Academy starter.

mod migration;
mod models;
mod service;

#[cfg(test)]
mod tests;

pub fn get_files() -> Vec<(&'static str, String)> {
    let mut files = models::get_files();
    files.push((
        "src/services/school_service.rs",
        service::SCHOOL_SERVICE.to_string(),
    ));
    files.push((
        "src/migrations/m20260901500000_add_school_tenancy.rs",
        migration::SCHOOL_TENANCY_MIGRATION.to_string(),
    ));
    files
}
