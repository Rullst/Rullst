// src/blueprints/lms.rs — LMS Course Platform blueprint templates.

use super::common;

pub fn file_manifest(
    project_name_safe: &str,
    hot_reload: bool,
    orm_pattern: &str,
    frontend_engine: &str,
) -> Vec<(&'static str, String)> {
    let mut manifest = Vec::new();
    let is_repo = common::is_repo_mode(orm_pattern);
    let _ = (project_name_safe, frontend_engine);

    // 1. main.rs
    if hot_reload {
        let repo_decl = common::repo_mod_decl(orm_pattern);
        let lib_rs = format!(
            r##"use rullst::{{routes, Router}};

pub mod migrations;
pub mod models;
{repo_decl}pub mod controllers;
pub mod pages;

pub fn router() -> Router {{
    let nexus = rullst::nexus::Nexus::new()
        .with_auth("admin", "password")
        .with_brand("LMS Admin")
        .register::<models::category::Category>()
        .register::<models::course::Course>()
        .register::<models::lesson::Lesson>()
        .build();

    routes![
        get("/" => controllers::lms_controller::index),
        get("/courses/{{id}}" => controllers::lms_controller::show_course),
        get("/lessons/{{id}}/play" => controllers::lms_controller::play_lesson),
    ].nest_axum("/nexus", nexus)
}}

#[unsafe(no_mangle)]
pub extern "C" fn rullst_router_init() -> *mut Router {{
    Box::into_raw(Box::new(router()))
}}
"##,
            repo_decl = repo_decl
        );
        manifest.push(("src/lib.rs", lib_rs));

        let main_rs = format!(
            r##"pub mod migrations;
pub mod models;
{repo_decl}pub mod controllers;
pub mod pages;

#[rullst::runtime::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {{
    rullst::artisan!(crate::migrations::get_migrations());

    let is_dev = std::env::var("APP_ENV").unwrap_or_else(|_| "development".to_string()) != "production";
    if is_dev {{
        rullst::runtime::spawn(async {{ let _ = rullst::studio::run_studio(5555).await; }});
        println!("📊 Rullst Studio running on http://127.0.0.1:5555");
    }}
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
        let router = {project_name_safe}::router();
        rullst::Server::new(router)
    }};

    server.run(3000).await?;

    Ok(())
}}
"##,
            repo_decl = repo_decl,
            project_name_safe = project_name_safe
        );
        manifest.push(("src/main.rs", main_rs));
    } else {
        let repo_decl = common::repo_mod_decl(orm_pattern);
        let main_rs = format!(
            r##"use rullst::{{routes, Server}};

pub mod migrations;
pub mod models;
{repo_decl}pub mod controllers;
pub mod pages;

#[rullst::runtime::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {{
    // Run migrations on startup
    rullst::artisan!(crate::migrations::get_migrations());

    let nexus = rullst::nexus::Nexus::new()
        .with_auth("admin", "password")
        .with_brand("LMS Admin")
        .register::<models::category::Category>()
        .register::<models::course::Course>()
        .register::<models::lesson::Lesson>()
        .build();

    let router = routes![
        get("/" => controllers::lms_controller::index),
        get("/courses/{{id}}" => controllers::lms_controller::show_course),
        get("/lessons/{{id}}/play" => controllers::lms_controller::play_lesson),
    ].nest_axum("/nexus", nexus);

    let is_dev = std::env::var("APP_ENV").unwrap_or_else(|_| "development".to_string()) != "production";
    if is_dev {{
        rullst::runtime::spawn(async {{ let _ = rullst::studio::run_studio(5555).await; }});
        println!("📊 Rullst Studio running on port 5555");
    }}
    println!("🚀 LMS server starting on port 3000...");
    Server::new(router)
        .run(3000)
        .await?;

    Ok(())
}}
"##,
            repo_decl = repo_decl
        );
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
            FieldMeta { name: "course_id", label: "Course", kind: FieldKind::ForeignKey { table: "courses", label_col: "title" }, hidden: false, readonly: false },
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
    let course = match Course::find(id).await.unwrap_or(None) {
        Some(c) => c,
        None => return Html("<h1>404 Course Not Found</h1>".to_string()).into_response(),
    };
    let all_lessons = Lesson::all().await.unwrap_or_default();
    let course_lessons: Vec<Lesson> = all_lessons.into_iter().filter(|l| l.course_id == id).collect();
    
    Html(lms::course_detail_page(course, course_lessons)).into_response()
}

pub async fn play_lesson(Path(id): Path<i32>) -> impl IntoResponse {
    let lesson = match Lesson::find(id).await.unwrap_or(None) {
        Some(l) => l,
        None => return Html("<h1>404 Lesson Not Found</h1>".to_string()).into_response(),
    };
    Html(lms::video_player_snippet(&lesson.title, &lesson.video_url)).into_response()
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
    let fe_imports = common::frontend_page_imports(frontend_engine);
    let lms_page = format!(
        r##"{fe_imports}use crate::models::category::Category;
use crate::models::course::Course;
use crate::models::lesson::Lesson;"##,
        fe_imports = fe_imports
    ) + r##"

pub fn index_page(categories: Vec<Category>, courses: Vec<Course>) -> String {
    html! {
        <html lang="en" class="dark">
            <head>
            <link rel="icon" type="image/png" href="https://raw.githubusercontent.com/venelouis/Rullst/main/Rullst.png" />
                <meta charset="UTF-8" />
                <title>"Rullst Academy - Courses"</title>
                <link href="https://fonts.googleapis.com/css2?family=Outfit:wght@300;400;500;600;700&display=swap" rel="stylesheet" />
                <script src="https://unpkg.com/htmx.org@1.9.10" integrity="sha384-D1Kt99CQMDuVetoL1lrYwg5t+9QdHe7NLX/SoJYkXDFfX37iInKRy5xLSi8nO7UC" crossorigin="anonymous"></script>
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
                        <div style="display: flex; gap: 1rem; align-items: flex-start;">
                        <div style="display: flex; flex-direction: column; align-items: center; gap: 0.25rem;">
                            <a class="btn" href="/nexus" style="background: #1e293b; border: 1px solid #334155; font-size: 0.9rem;">"⚙️ Nexus CMS"</a>
                            <span style="font-size: 0.7rem; color: #94a3b8;">"(login: admin / password)"</span>
                        </div>
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
            <link rel="icon" type="image/png" href="https://raw.githubusercontent.com/venelouis/Rullst/main/Rullst.png" />
                <meta charset="UTF-8" />
                <title>{&course.title}</title>
                <link href="https://fonts.googleapis.com/css2?family=Outfit:wght@300;400;500;600;700&display=swap" rel="stylesheet" />
                <script src="https://unpkg.com/htmx.org@1.9.10" integrity="sha384-D1Kt99CQMDuVetoL1lrYwg5t+9QdHe7NLX/SoJYkXDFfX37iInKRy5xLSi8nO7UC" crossorigin="anonymous"></script>
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
                            rullst::html::RawHtml(video_player_snippet(&first_lesson.title, &first_lesson.video_url))
                        } else {
                            rullst::html::RawHtml("<div style=\"color: #64748b;\">No lessons available</div>".to_string())
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

    // Repository layer (if applicable)
    if is_repo {
        manifest.push((
            "src/repositories/course_repository.rs",
            common::generate_repository("Course", "courses"),
        ));
        manifest.push((
            "src/repositories/lesson_repository.rs",
            common::generate_repository("Lesson", "lessons"),
        ));
        manifest.push((
            "src/repositories/category_repository.rs",
            common::generate_repository("Category", "categories"),
        ));
        manifest.push((
            "src/repositories/mod.rs",
            common::generate_repositories_mod(&["Course", "Lesson", "Category"]),
        ));
    }

    manifest
}
