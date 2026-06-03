import os
import re

BLUEPRINTS_DIR = r"c:\Users\venelouis\Desktop\REPOS\Rullst\cargo-rullst\src\blueprints"

# 1. Patch LMS
lms_path = os.path.join(BLUEPRINTS_DIR, "lms.rs")
with open(lms_path, "r", encoding="utf-8") as f:
    lms = f.read()

# Router
lms = lms.replace(
    r"""    let router = routes![
        get("/" => controllers::lms_controller::index),
        get("/courses/{id}" => controllers::lms_controller::show_course),
        get("/lessons/{id}/play" => controllers::lms_controller::play_lesson),
    ];
    Box::into_raw(Box::new(router))""",
    r"""    let nexus = rullst::nexus::Nexus::new()
        .with_brand("LMS Admin")
        .register::<models::category::Category>()
        .register::<models::course::Course>()
        .register::<models::lesson::Lesson>()
        .build();

    let router = routes![
        get("/" => controllers::lms_controller::index),
        get("/courses/{id}" => controllers::lms_controller::show_course),
        get("/lessons/{id}/play" => controllers::lms_controller::play_lesson),
    ].nest("/nexus", nexus);

    Box::into_raw(Box::new(router))"""
)

lms = lms.replace(
    r"""    let router = routes![
        get("/" => controllers::lms_controller::index),
        get("/courses/{id}" => controllers::lms_controller::show_course),
        get("/lessons/{id}/play" => controllers::lms_controller::play_lesson),
    ];

    println!("🚀 LMS server starting on port 3000...");""",
    r"""    let nexus = rullst::nexus::Nexus::new()
        .with_brand("LMS Admin")
        .register::<models::category::Category>()
        .register::<models::course::Course>()
        .register::<models::lesson::Lesson>()
        .build();

    let router = routes![
        get("/" => controllers::lms_controller::index),
        get("/courses/{id}" => controllers::lms_controller::show_course),
        get("/lessons/{id}/play" => controllers::lms_controller::play_lesson),
    ].nest("/nexus", nexus);

    rullst::runtime::spawn(async { let _ = rullst::studio::run_studio("").await; });
    println!("📊 Rullst Studio running on http://127.0.0.1:5555");
    println!("🚀 LMS server starting on port 3000...");"""
)

# Insert spawn to hot reload main
lms = lms.replace(
    r"""    println!("🚀 LMS server starting on port 3000...");""",
    r"""    rullst::runtime::spawn(async { let _ = rullst::studio::run_studio("").await; });
    println!("📊 Rullst Studio running on http://127.0.0.1:5555");
    println!("🚀 LMS server starting on port 3000...");"""
)

# Models
lms = lms.replace(
    r"""use rullst::db::{Orm, RullstModel, FromRow, sqlx};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "categories")]
pub struct Category {
    pub id: i32,
    pub name: String,
}
"##;""",
    r"""use rullst::db::{Orm, RullstModel, FromRow, sqlx};
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
"##;"""
)

lms = lms.replace(
    r"""use rullst::db::{Orm, RullstModel, FromRow, sqlx};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "courses")]
pub struct Course {
    pub id: i32,
    pub category_id: i32,
    pub title: String,
    pub description: String,
    pub thumbnail: String,
}
"##;""",
    r"""use rullst::db::{Orm, RullstModel, FromRow, sqlx};
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
"##;"""
)

lms = lms.replace(
    r"""use rullst::db::{Orm, RullstModel, FromRow, sqlx};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "lessons")]
pub struct Lesson {
    pub id: i32,
    pub course_id: i32,
    pub title: String,
    pub video_url: String,
    pub duration: i32,
}
"##;""",
    r"""use rullst::db::{Orm, RullstModel, FromRow, sqlx};
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
"##;"""
)

# UI
lms = lms.replace(
    r"""                    <header>
                        <h1>"Rullst LMS Academy"</h1>
                        <p class="sub">"Explore high-fidelity systems engineering with Rust"</p>
                    </header>""",
    r"""                    <header style="display: flex; justify-content: space-between; align-items: center;">
                        <div>
                            <h1 style="text-align: left;">"Rullst LMS Academy"</h1>
                            <p class="sub" style="text-align: left;">"Explore high-fidelity systems engineering with Rust"</p>
                        </div>
                        <div style="display: flex; gap: 1rem;">
                            <a class="btn" href="/nexus" style="background: #1e293b;">"⚙️ Nexus CMS"</a>
                            <a class="btn" href="http://localhost:5555" target="_blank" style="background: #1e293b;">"📊 Rullst Studio"</a>
                        </div>
                    </header>"""
)

with open(lms_path, "w", encoding="utf-8") as f:
    f.write(lms)
print("Updated lms.rs")

