// src/blueprints/lms.rs — LMS Course Platform blueprint templates.

pub fn file_manifest(project_name_safe: &str, hot_reload: bool) -> Vec<(&'static str, String)> {
    let mut manifest = Vec::new();

    // 1. main.rs
    if hot_reload {
        let lib_rs = r##"use rullst::{routes, Router};

pub mod migrations;
pub mod models;
pub mod controllers;
pub mod pages;

#[unsafe(no_mangle)]
pub extern "C" fn rullst_router_init() -> *mut Router {
    let nexus = rullst::nexus::Nexus::new()
        .with_brand("LMS Admin")
        .register::<models::category::Category>()
        .register::<models::course::Course>()
        .register::<models::lesson::Lesson>()
        .build();

    let router = routes![
        get("/" => controllers::lms_controller::index),
        get("/courses/{id}" => controllers::lms_controller::show_course),
        get("/lessons/{id}/play" => controllers::lms_controller::play_lesson),
    ].nest_axum("/nexus", nexus);
    Box::into_raw(Box::new(router))
}
"##
        .to_string();
        manifest.push(("src/lib.rs", lib_rs));

        let main_rs = format!(
            r##"pub mod migrations;
pub mod models;
pub mod controllers;
pub mod pages;

#[rullst::runtime::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {{
    rullst::artisan!(crate::migrations::get_migrations());

    rullst::runtime::spawn(async {{ let _ = rullst::studio::run_studio("").await; }});
    println!("📊 Rullst Studio running on http://127.0.0.1:5555");
    println!("🚀 LMS server starting on port 3000...");
    let is_hot = std::env::var("HOT_RELOAD").is_ok();

    let server = if is_hot {{
        let lib_path = if cfg!(target_os = "windows") {{
            format!("target/debug/{{}}", "{project_name_safe}")
        }} else {{
            format!("target/debug/lib{{}}", "{project_name_safe}")
        }};
        rullst::Server::new_hot(&lib_path)
    }} else {{
        let router_ptr = {project_name_safe}::rullst_router_init();
        let router = unsafe {{ *Box::from_raw(router_ptr) }};
        rullst::Server::new(router)
    }};

    server.run(3000).await?;

    Ok(())
}}
"##,
            project_name_safe = project_name_safe
        );
        manifest.push(("src/main.rs", main_rs));
    } else {
        let main_rs = r##"use rullst::{routes, Server};

pub mod migrations;
pub mod models;
pub mod controllers;
pub mod pages;

#[rullst::runtime::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Run migrations on startup
    rullst::artisan!(crate::migrations::get_migrations());

    let nexus = rullst::nexus::Nexus::new()
        .with_brand("LMS Admin")
        .register::<models::category::Category>()
        .register::<models::course::Course>()
        .register::<models::lesson::Lesson>()
        .build();

    let router = routes![
        get("/" => controllers::lms_controller::index),
        get("/courses/{id}" => controllers::lms_controller::show_course),
        get("/lessons/{id}/play" => controllers::lms_controller::play_lesson),
    ].nest_axum("/nexus", nexus);

    rullst::runtime::spawn(async { let _ = rullst::studio::run_studio("").await; });
    println!("📊 Rullst Studio running on http://127.0.0.1:5555");
    println!("🚀 LMS server starting on port 3000...");
    Server::new(router)
        .run(3000)
        .await?;

    Ok(())
}
"##
        .to_string();
        manifest.push(("src/main.rs", main_rs));
    }

    // 2. Migration
    let migration = r##"use rullst::db::schema::{Schema, Migration};
use rullst::db::async_trait;

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {
    fn name(&self) -> &'static str {
        "m20260601000000_create_lms_tables"
    }

    async fn up(&self) -> Result<(), rullst_orm::error::RullstError> {
        // Create categories table
        Schema::create("categories", |table| {
            table.id();
            table.string("name").not_null();
            table.timestamps();
        }).await?;

        // Create courses table
        Schema::create("courses", |table| {
            table.id();
            table.integer("category_id").not_null();
            table.string("title").not_null();
            table.string("description").not_null();
            table.string("thumbnail").not_null();
            table.timestamps();
        }).await?;

        // Create lessons table
        Schema::create("lessons", |table| {
            table.id();
            table.integer("course_id").not_null();
            table.string("title").not_null();
            table.string("video_url").not_null();
            table.integer("duration").not_null(); // in minutes
            table.timestamps();
        }).await?;

        // Seed initial data
        let pool = rullst::db::Orm::pool();

        // Seed Categories
        rullst::db::sqlx::query(
            "INSERT INTO categories (id, name, created_at, updated_at) VALUES 
             (1, 'Backend & Systems', datetime('now'), datetime('now')),
             (2, 'Web Development', datetime('now'), datetime('now'))"
        ).execute(pool).await?;

        // Seed Courses
        rullst::db::sqlx::query(
            "INSERT INTO courses (id, category_id, title, description, thumbnail, created_at, updated_at) VALUES 
             (1, 1, 'Rust Advanced Systems Programming', 'Master threads, concurrency, async, and high-performance design.', 'https://images.unsplash.com/photo-1607799279861-4dd421887fb3?q=80&w=300', datetime('now'), datetime('now')),
             (2, 2, 'Zero to Hero: Web Apps with Rullst', 'Build clean, high-performance web applications using Rust.', 'https://images.unsplash.com/photo-1547082299-de196ea013d6?q=80&w=300', datetime('now'), datetime('now'))"
        ).execute(pool).await?;

        // Seed Lessons
        rullst::db::sqlx::query(
            "INSERT INTO lessons (id, course_id, title, video_url, duration, created_at, updated_at) VALUES 
             (1, 1, 'Introduction to Memory Safety', 'https://www.w3schools.com/html/mov_bbb.mp4', 15, datetime('now'), datetime('now')),
             (2, 1, 'Deep Dive into Smart Pointers', 'https://media.w3.org/2010/05/sintel/trailer.mp4', 25, datetime('now'), datetime('now')),
             (3, 2, 'Setting up your first Rullst Project', 'https://www.w3schools.com/html/mov_bbb.mp4', 10, datetime('now'), datetime('now')),
             (4, 2, 'Building Interactive UIs with HTMX', 'https://media.w3.org/2010/05/sintel/trailer.mp4', 20, datetime('now'), datetime('now'))"
        ).execute(pool).await?;

        Ok(())
    }

    async fn down(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::drop_if_exists("lessons").await?;
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

    let migrations_mod = r##"// Generated by Rullst.
pub mod m20260601000000_create_lms_tables;

pub fn get_migrations() -> Vec<Box<dyn rullst::db::schema::Migration>> {
    vec![
        Box::new(m20260601000000_create_lms_tables::MigrationImpl),
    ]
}
"##;
    manifest.push(("src/migrations/mod.rs", migrations_mod.to_string()));

    // 3. Models
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
            FieldMeta { name: "category_id", label: "Category ID", kind: FieldKind::Number, hidden: false, readonly: false },
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
    pub title: String,
    pub video_url: String,
    pub duration: i32,
}

impl NexusModel for Lesson {
    fn nexus_table() -> &'static str { "lessons" }
    fn nexus_label() -> &'static str { "Lessons" }
    fn nexus_icon() -> &'static str { "▶️" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "course_id", label: "Course ID", kind: FieldKind::Number, hidden: false, readonly: false },
            FieldMeta { name: "title", label: "Title", kind: FieldKind::Text, hidden: false, readonly: false },
            FieldMeta { name: "video_url", label: "Video URL", kind: FieldKind::Url, hidden: false, readonly: false },
            FieldMeta { name: "duration", label: "Duration (mins)", kind: FieldKind::Number, hidden: false, readonly: false },
        ]
    }
}
"##;
    manifest.push(("src/models/lesson.rs", lesson_model.to_string()));

    let models_mod = r##"pub mod category;
pub mod course;
pub mod lesson;
"##;
    manifest.push(("src/models/mod.rs", models_mod.to_string()));

    // 4. Controller
    let lms_controller = r##"use rullst::server::{Path, IntoResponse};
use rullst::response::Html;
use crate::models::category::Category;
use crate::models::course::Course;
use crate::models::lesson::Lesson;
use crate::pages::lms;

pub async fn index() -> impl IntoResponse {
    let categories = Category::all().await.unwrap_or_default();
    let courses = Course::all().await.unwrap_or_default();
    Html(lms::index_page(categories, courses))
}

pub async fn show_course(Path(id): Path<i32>) -> impl IntoResponse {
    let course = Course::find(id).await.unwrap().unwrap();
    let all_lessons = Lesson::all().await.unwrap_or_default();
    let course_lessons: Vec<Lesson> = all_lessons.into_iter().filter(|l| l.course_id == id).collect();
    
    Html(lms::course_detail_page(course, course_lessons))
}

pub async fn play_lesson(Path(id): Path<i32>) -> impl IntoResponse {
    let lesson = Lesson::find(id).await.unwrap().unwrap();
    Html(lms::video_player_snippet(&lesson.title, &lesson.video_url))
}
"##;
    manifest.push((
        "src/controllers/lms_controller.rs",
        lms_controller.to_string(),
    ));

    let controllers_mod = r##"pub mod lms_controller;
"##;
    manifest.push(("src/controllers/mod.rs", controllers_mod.to_string()));

    // 5. Pages
    let lms_page = r##"use rullst::html;
use crate::models::category::Category;
use crate::models::course::Course;
use crate::models::lesson::Lesson;

pub fn index_page(categories: Vec<Category>, courses: Vec<Course>) -> String {
    html! {
        <html lang="en" class="dark">
            <head>
            <link rel="icon" type="image/png" href="data:image/png;base64,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" />
                <meta charset="UTF-8" />
                <title>"Rullst Academy - Courses"</title>
                <link href="https://fonts.googleapis.com/css2?family=Outfit:wght@300;400;500;600;700&display=swap" rel="stylesheet" />
                <script src="https://unpkg.com/htmx.org@1.9.10"></script>
                <style>
                    "
                    * { box-sizing: border-box; margin: 0; padding: 0; font-family: 'Outfit', sans-serif; }
                    body { background: #080b11; color: #f1f5f9; min-height: 100vh; padding: 3rem 1.5rem; }
                    .container { max-width: 1000px; margin: 0 auto; }
                    header { text-align: center; margin-bottom: 4rem; }
                    h1 { font-size: 3rem; background: linear-gradient(135deg, #10b981, #f97316); -webkit-background-clip: text; -webkit-text-fill-color: transparent; font-weight: 800; }
                    p.sub { color: #64748b; font-size: 1.15rem; margin-top: 0.5rem; }
                    .courses-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 2.5rem; }
                    .card { background: #111827; border: 1px solid #1f2937; border-radius: 1.25rem; overflow: hidden; transition: transform 0.3s, border-color 0.3s; display: flex; flex-direction: column; }
                    .card:hover { transform: translateY(-5px); border-color: #10b981; }
                    .card-img { height: 180px; width: 100%; object-fit: cover; }
                    .card-body { padding: 2rem; flex: 1; display: flex; flex-direction: column; }
                    .card h3 { font-size: 1.4rem; margin-bottom: 0.75rem; color: #ffffff; }
                    .card p { color: #94a3b8; font-size: 0.95rem; line-height: 1.6; margin-bottom: 1.5rem; flex: 1; }
                    .btn { display: inline-block; text-align: center; background: linear-gradient(135deg, #10b981, #059669); color: #ffffff; text-decoration: none; padding: 0.8rem; border-radius: 0.75rem; font-weight: 600; transition: opacity 0.2s; }
                    .btn:hover { opacity: 0.9; }
                    "
                </style>
            </head>
            <body>
                <div class="container">
                    <header style="display: flex; justify-content: space-between; align-items: center;">
                        <div style="text-align: left;">
                            <h1>"Rullst LMS Academy"</h1>
                            <p class="sub">"Explore high-fidelity systems engineering with Rust"</p>
                        </div>
                        <div style="display: flex; gap: 1rem;">
                            <a class="btn" href="/nexus" style="background: #1e293b; border: 1px solid #334155; font-size: 0.9rem;">"⚙️ Nexus CMS"</a>
                            <a class="btn" href="http://localhost:5555" target="_blank" style="background: #1e293b; border: 1px solid #334155; font-size: 0.9rem;">"📊 Rullst Studio"</a>
                        </div>
                    </header>
                    <div class="categories-container">
                        { rullst::html::RawHtml(categories.into_iter().map(|cat| html! {
                            <div style="margin-bottom: 4rem;">
                                <h2 style="font-size: 2rem; color: #ffffff; margin-bottom: 1.5rem; padding-bottom: 0.5rem; border-bottom: 1px solid #1e293b;">{&cat.name}</h2>
                                <div class="courses-grid">
                                    { rullst::html::RawHtml(courses.iter().filter(|c| c.category_id == cat.id).map(|c| html! {
                                        <div class="card">
                                            <img class="card-img" src={&c.thumbnail} alt={&c.title} />
                                            <div class="card-body">
                                                <h3>{&c.title}</h3>
                                                <p>{&c.description}</p>
                                                <a class="btn" href={format!("/courses/{}", c.id)}>"Start Learning"</a>
                                            </div>
                                        </div>
                                    }).collect::<Vec<_>>().join("")) }
                                </div>
                            </div>
                        }).collect::<Vec<_>>().join("")) }
                    </div>
                </div>
            </body>
        </html>
    }
}

pub fn course_detail_page(course: Course, lessons: Vec<Lesson>) -> String {
    html! {
        <html lang="en" class="dark">
            <head>
            <link rel="icon" type="image/png" href="data:image/png;base64,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" />
                <meta charset="UTF-8" />
                <title>{&course.title}</title>
                <link href="https://fonts.googleapis.com/css2?family=Outfit:wght@300;400;500;600;700&display=swap" rel="stylesheet" />
                <script src="https://unpkg.com/htmx.org@1.9.10"></script>
                <style>
                    "
                    * { box-sizing: border-box; margin: 0; padding: 0; font-family: 'Outfit', sans-serif; }
                    body { background: #080b11; color: #f1f5f9; min-height: 100vh; display: flex; }
                    .sidebar { width: 350px; background: #0f172a; border-right: 1px solid #1e293b; display: flex; flex-direction: column; }
                    .sidebar-header { padding: 2rem; border-bottom: 1px solid #1e293b; }
                    .sidebar-header h2 { font-size: 1.25rem; font-weight: 700; color: #ffffff; }
                    .lessons-list { list-style: none; overflow-y: auto; flex: 1; }
                    .lesson-item { padding: 1.5rem 2rem; border-bottom: 1px solid #1e293b; cursor: pointer; transition: background-color 0.2s; }
                    .lesson-item:hover { background-color: #1e293b; }
                    .lesson-item.active { background-color: #064e3b; }
                    .lesson-item h4 { font-size: 0.95rem; font-weight: 600; color: #ffffff; margin-bottom: 0.25rem; }
                    .lesson-item span { font-size: 0.8rem; color: #94a3b8; }
                    .main-content { flex: 1; display: flex; flex-direction: column; background: #090d16; }
                    .video-wrapper { flex: 1; display: flex; align-items: center; justify-content: center; padding: 3rem; }
                    .video-container { width: 100%; max-width: 800px; background: #111827; border: 1px solid #1f2937; border-radius: 1.5rem; overflow: hidden; box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.5); }
                    .player { width: 100%; aspect-ratio: 16/9; background: #000; display: block; }
                    .info-bar { padding: 2rem; background: #0f172a; border-top: 1px solid #1e293b; }
                    .info-bar h3 { font-size: 1.5rem; color: #ffffff; margin-bottom: 0.5rem; }
                    .back-btn { margin-bottom: 1rem; display: inline-block; color: #f97316; text-decoration: none; font-size: 0.9rem; font-weight: 600; }
                    "
                </style>
            </head>
            <body>
                <div class="sidebar">
                    <div class="sidebar-header">
                        <a class="back-btn" href="/">"&larr; Back to Academy"</a>
                        <h2>{&course.title}</h2>
                    </div>
                    <ul class="lessons-list">
                        { rullst::html::RawHtml(lessons.iter().map(|l| html! {
                            <li class="lesson-item" hx-get={format!("/lessons/{}/play", l.id)} hx-target="#video-panel" hx-swap="innerHTML">
                                <h4>{&l.title}</h4>
                                <span>{{l.duration.to_string()}}" mins"</span>
                            </li>
                        }).collect::<Vec<_>>().join("")) }
                    </ul>
                </div>
                <div class="main-content">
                    <div class="video-wrapper" id="video-panel">
                        { if let Some(first_lesson) = lessons.first() {
                            video_player_snippet(&first_lesson.title, &first_lesson.video_url)
                        } else {
                            html! { <div style="color: #64748b;">"No lessons available"</div> }
                        } }
                    </div>
                </div>
            </body>
        </html>
    }
}

pub fn video_player_snippet(title: &str, video_url: &str) -> String {
    html! {
        <div class="video-container">
            <video class="player" controls="controls" autoplay="autoplay" src={video_url}></video>
            <div class="info-bar">
                <h3>{title}</h3>
                <p style="color: #94a3b8; font-size: 0.9rem;">"Now playing from Rullst Cloud CDN."</p>
            </div>
        </div>
    }
}
"##;
    manifest.push(("src/pages/lms.rs", lms_page.to_string()));

    let pages_mod = r##"pub mod lms;
"##;
    manifest.push(("src/pages/mod.rs", pages_mod.to_string()));

    manifest
}
