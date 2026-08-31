// src/blueprints/lms.rs — LMS Course Platform blueprint templates.

use super::common;

mod academy_catalog_tests;
mod academy_http_tests;
mod academy_notification_realtime_tests;
mod academy_privacy_tests;
mod academy_schema;
mod academy_schema_tests;
mod academy_score_quiz_tests;
mod academy_tenancy_tests;
mod academy_timed_tests;
mod access;
mod activity_contract;
mod assessment;
#[cfg(test)]
mod assessment_tests;
mod assessment_timing;
mod auth;
mod automation;
mod automation_execution;
mod automation_worker;
mod availability;
mod base_modules;
mod catalog;
mod curriculum;
mod domain_events;
mod foundation;
mod gamification;
mod learning;
mod lms_player;
mod module_selection;
mod notifications;
mod outbox;
mod privacy;
mod progress;
mod repositories;
mod routes;
mod scheduler_lease;
mod score;
mod score_corrections;
mod tenancy;

pub use module_selection::{
    LmsModule, LmsModuleError, file_manifest_for_modules, validate_module_selection,
};

pub fn file_manifest(
    project_name_safe: &str,
    hot_reload: bool,
    orm_pattern: &str,
    frontend_engine: &str,
) -> Vec<(&'static str, String)> {
    let mut manifest = Vec::new();
    let is_repo = common::is_repo_mode(orm_pattern);
    manifest.extend(routes::get_routes(
        project_name_safe,
        hot_reload,
        orm_pattern,
    ));
    manifest.extend(activity_contract::get_files());
    manifest.extend(assessment::get_files());
    manifest.extend(assessment_timing::get_files());
    manifest.extend(automation::get_files());
    manifest.extend(automation_execution::get_files());
    manifest.extend(automation_worker::get_files());
    manifest.extend(availability::get_files());
    manifest.extend(learning::get_files());
    manifest.extend(notifications::get_files());
    manifest.extend(curriculum::get_files());
    manifest.extend(domain_events::get_files());
    manifest.extend(gamification::get_files());
    manifest.extend(outbox::get_files());
    manifest.extend(progress::get_files());
    manifest.extend(privacy::get_files());
    manifest.extend(score::get_files());
    manifest.extend(score_corrections::get_files());
    manifest.extend(tenancy::get_files());
    manifest.extend(scheduler_lease::get_files());
    manifest.extend(academy_schema::get_files());
    manifest.extend(access::get_files());
    manifest.extend(auth::get_files());
    manifest.extend(catalog::get_files(frontend_engine));

    let migration = r##"use rullst::db::schema::{Schema, Migration};
use rullst::db::async_trait;

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {
    fn name(&self) -> &'static str {
        "m20260601000000_create_lms_tables"
    }

    async fn up(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::create("categories", |table| {
            table.id();
            table.string("name").not_null();
            table.timestamps();
        }).await?;
        Schema::create("courses", |table| {
            table.id();
            table.integer("category_id").not_null();
            table.string("title").not_null();
            table.string("description").not_null();
            table.string("thumbnail").not_null();
            table.timestamps();
        }).await?;
        Schema::create("course_modules", |table| {
            table.id();
            table.integer("course_id").not_null();
            table.string("title").not_null();
            table.integer("position").not_null();
            table.string("status").not_null();
            table.timestamps();
        }).await?;
        Schema::create("lessons", |table| {
            table.id();
            table.integer("course_id").not_null();
            table.integer("module_id").not_null();
            table.string("title").not_null();
            table.string("media_kind").not_null();
            table.string("media_url").not_null();
            table.string("captions_url").not_null();
            table.string("transcript").not_null();
            table.string("language_tag").not_null();
            table.integer("duration").not_null(); // in minutes
            table.timestamps();
        }).await?;
        let pool = rullst::db::Orm::pool()?;
        rullst::db::sqlx::query(
            "INSERT INTO categories (id, name) VALUES
             (1, 'Backend & Systems'),
             (2, 'Web Development')"
        ).execute(pool).await?;
        rullst::db::sqlx::query(
            "INSERT INTO courses (id, category_id, title, description, thumbnail) VALUES
             (1, 1, 'Rust Advanced Systems Programming', 'Master threads, concurrency, async, and high-performance design.', 'https://images.unsplash.com/photo-1607799279861-4dd421887fb3?q=80&w=300'),
             (2, 2, 'Zero to Hero: Web Apps with Rullst', 'Build clean, high-performance web applications using Rust.', 'https://images.unsplash.com/photo-1547082299-de196ea013d6?q=80&w=300')"
        ).execute(pool).await?;
        rullst::db::sqlx::query(
            "INSERT INTO course_modules (id, course_id, title, position, status) VALUES
             (1, 1, 'Safe Systems Foundations', 1, 'published'),
             (2, 2, 'Rullst Web Foundations', 1, 'published')"
        ).execute(pool).await?;
        // Seed Lessons
        rullst::db::sqlx::query(
            "INSERT INTO lessons (id, course_id, module_id, title, media_kind, media_url, captions_url, transcript, language_tag, duration) VALUES
             (1, 1, 1, 'Introduction to Memory Safety', 'video', 'https://www.w3schools.com/html/mov_bbb.mp4', '/static/media/memory-safety.en.vtt', 'Rust ownership keeps one clear owner for each value and releases the value when that owner leaves scope.', 'en', 15),
             (2, 1, 1, 'Deep Dive into Smart Pointers', 'audio', 'https://www.w3schools.com/html/horse.ogg', '', 'Smart pointers combine pointer behavior with metadata and ownership rules enforced by their types.', 'en', 25),
             (3, 2, 2, 'Setting up your first Rullst Project', 'video', 'https://www.w3schools.com/html/mov_bbb.mp4', '/static/media/first-project.en.vtt', 'Create a project, inspect the generated files, run migrations and keep the server as the authority.', 'en', 10),
             (4, 2, 2, 'Building Interactive UIs with HTMX', 'audio', 'https://www.w3schools.com/html/horse.ogg', '', 'HTMX can request server-rendered fragments while Rust keeps validation and authorization on the server.', 'en', 20)"
        ).execute(pool).await?;

        Ok(())
    }

    async fn down(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::drop_if_exists("lessons").await?;
        Schema::drop_if_exists("course_modules").await?;
        Schema::drop_if_exists("courses").await?;
        Schema::drop_if_exists("categories").await?;
        Ok(())
    }
}
"##;
    manifest.push((
        "src/migrations/m20260601000000_create_lms_tables.rs",
        migration.to_string(),
    ));
    manifest.push((
        "static/media/memory-safety.en.vtt",
        "WEBVTT\n\n00:00.000 --> 00:05.000\nRust ownership keeps one clear owner for each value.\n"
            .to_string(),
    ));
    manifest.push((
        "static/media/first-project.en.vtt",
        "WEBVTT\n\n00:00.000 --> 00:05.000\nCreate a project, inspect its files, then run the migrations.\n"
            .to_string(),
    ));

    manifest.push(("src/migrations/mod.rs", academy_schema::migrations_module()));

    let category_model = r##"use rullst::db::{Orm, FromRow};
use rullst::nexus::{NexusModel, FieldMeta, FieldKind};
#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "categories")]
pub struct Category {
    pub id: i32,
    pub name: String,
}
impl NexusModel for Category {
    fn nexus_table() -> &'static str { "categories" }
    fn nexus_label() -> &'static str { "Categories" }
    fn nexus_icon() -> &'static str { "📁" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "name", label: "Name", kind: FieldKind::Text, hidden: false, readonly: false },
        ]
    }
}
"##;
    manifest.push(("src/models/category.rs", category_model.to_string()));

    let course_model = r##"use rullst::db::{Orm, FromRow};
use rullst::nexus::{NexusModel, FieldMeta, FieldKind};
#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "courses")]
pub struct Course {
    pub id: i32,
    pub category_id: i32,
    pub title: String,
    pub description: String,
    pub thumbnail: String,
}
impl NexusModel for Course {
    fn nexus_table() -> &'static str { "courses" }
    fn nexus_label() -> &'static str { "Courses" }
    fn nexus_icon() -> &'static str { "🎓" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "category_id", label: "Category", kind: FieldKind::ForeignKey { table: "categories", label_col: "name" }, hidden: false, readonly: false },
            FieldMeta { name: "title", label: "Title", kind: FieldKind::Text, hidden: false, readonly: false },
            FieldMeta { name: "description", label: "Description", kind: FieldKind::Textarea, hidden: false, readonly: false },
            FieldMeta { name: "thumbnail", label: "Thumbnail URL", kind: FieldKind::Url, hidden: false, readonly: false },
        ]
    }
}
"##;
    manifest.push(("src/models/course.rs", course_model.to_string()));

    let lesson_model = r##"use rullst::db::{Orm, FromRow};
use rullst::nexus::{NexusModel, FieldMeta, FieldKind};
#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "lessons")]
pub struct Lesson {
    pub id: i32,
    pub course_id: i32,
    pub module_id: i32,
    pub title: String,
    pub media_kind: String,
    pub media_url: String,
    pub captions_url: String,
    pub transcript: String,
    pub language_tag: String,
    pub duration: i32,
}
impl NexusModel for Lesson {
    fn nexus_table() -> &'static str { "lessons" }
    fn nexus_label() -> &'static str { "Lessons" }
    fn nexus_icon() -> &'static str { "▶️" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "course_id", label: "Course", kind: FieldKind::ForeignKey { table: "courses", label_col: "title" }, hidden: false, readonly: false },
            FieldMeta { name: "module_id", label: "Module", kind: FieldKind::ForeignKey { table: "course_modules", label_col: "title" }, hidden: false, readonly: false },
            FieldMeta { name: "title", label: "Title", kind: FieldKind::Text, hidden: false, readonly: false },
            FieldMeta { name: "media_kind", label: "Media Kind", kind: FieldKind::Text, hidden: false, readonly: false },
            FieldMeta { name: "media_url", label: "Media URL", kind: FieldKind::Url, hidden: false, readonly: false },
            FieldMeta { name: "captions_url", label: "Captions URL", kind: FieldKind::Url, hidden: false, readonly: false },
            FieldMeta { name: "transcript", label: "Transcript", kind: FieldKind::Textarea, hidden: false, readonly: false },
            FieldMeta { name: "language_tag", label: "Language", kind: FieldKind::Text, hidden: false, readonly: false },
            FieldMeta { name: "duration", label: "Duration (mins)", kind: FieldKind::Number, hidden: false, readonly: false },
        ]
    }
}
"##;
    manifest.push(("src/models/lesson.rs", lesson_model.to_string()));

    manifest.push(("src/models/mod.rs", base_modules::MODELS_MODULE.to_string()));

    let controllers_mod = r##"pub mod activity_controller; pub mod activity_matching_controller; pub mod activity_typed_controller; pub mod auth_controller;
pub mod assessment_controller; pub mod assignment_controller; pub mod completion_controller;
pub mod learning_controller;
pub mod lms_controller;
pub mod notification_controller;
pub mod publication_controller; pub mod publication_rollback_controller;
pub mod role_controller;
"##;
    manifest.push(("src/controllers/mod.rs", controllers_mod.to_string()));

    let pages_mod = r##"pub mod auth;
pub mod lms;
"##;
    manifest.push(("src/pages/mod.rs", pages_mod.to_string()));

    repositories::extend_manifest(&mut manifest, is_repo);

    manifest
}
