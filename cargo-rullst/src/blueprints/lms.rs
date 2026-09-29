// src/blueprints/lms.rs — LMS starter: catalog, courses, modules, lessons, an
// accessible player, enrollment, progress, login and a Nexus admin.

mod auth;
mod catalog;
mod curriculum;
mod foundation;
mod learning;
mod lms_player;

/// Generates the LMS starter. Its Active Record models do not vary with the ORM
/// pattern, and hot reload exports the router from `src/lib.rs`.
pub fn file_manifest(
    project_name_safe: &str,
    hot_reload: bool,
    _orm_pattern: &str,
    frontend_engine: &str,
) -> Vec<(&'static str, String)> {
    let mut manifest = Vec::new();
    manifest.extend(auth::get_files());
    manifest.extend(catalog::get_files(frontend_engine));
    manifest.extend(learning::get_files());
    manifest.extend(curriculum::get_files());

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

    foundation::starter(manifest, project_name_safe, hot_reload)
}
