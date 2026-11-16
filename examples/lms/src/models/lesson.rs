use rullst::db::{Orm, RowSnapshot};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};
#[derive(Debug, Clone, Orm)]
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
    // Remembers the loaded values, so `save()` writes only the columns this
    // record changed. Build new records with `snapshot: RowSnapshot::default()`.
    #[sqlx(skip)]
    pub snapshot: RowSnapshot,
}
impl NexusModel for Lesson {
    fn nexus_table() -> &'static str {
        "lessons"
    }
    fn nexus_label() -> &'static str {
        "Lessons"
    }
    fn nexus_icon() -> &'static str {
        "▶️"
    }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta::new("id", "ID", FieldKind::Number)
                .hidden()
                .readonly(),
            FieldMeta::new(
                "course_id",
                "Course",
                FieldKind::ForeignKey {
                    table: "courses",
                    label_col: "title",
                },
            ),
            FieldMeta::new(
                "module_id",
                "Module",
                FieldKind::ForeignKey {
                    table: "course_modules",
                    label_col: "title",
                },
            ),
            FieldMeta::new("title", "Title", FieldKind::Text),
            // The player plays exactly these kinds.
            FieldMeta::new(
                "media_kind",
                "Media Kind",
                FieldKind::Enum {
                    options: vec!["video", "audio"],
                },
            ),
            // Text, not Url: the player accepts HTTPS or same-origin paths such
            // as the seeded `/static/media/*.vtt` captions, which the CSP and
            // `<track>` need, while Nexus `Url` fields require an absolute URL.
            FieldMeta::new(
                "media_url",
                "Media URL (https:// or /path)",
                FieldKind::Text,
            ),
            FieldMeta::new(
                "captions_url",
                "Captions URL (https:// or /path)",
                FieldKind::Text,
            ),
            FieldMeta::new("transcript", "Transcript", FieldKind::Textarea),
            FieldMeta::new("language_tag", "Language", FieldKind::Text),
            FieldMeta::new("duration", "Duration (mins)", FieldKind::Number),
        ]
    }
}
