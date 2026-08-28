// Persistence models for derived completion and certificate audit state.

pub fn get_files() -> Vec<(&'static str, String)> {
    vec![
        (
            "src/models/course_completion.rs",
            COURSE_COMPLETION_MODEL.to_string(),
        ),
        ("src/models/certificate.rs", CERTIFICATE_MODEL.to_string()),
    ]
}

const COURSE_COMPLETION_MODEL: &str = r##"use rullst::db::{FromRow, Orm};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "course_completions")]
pub struct CourseCompletion {
    pub id: i32,
    pub completion_key: String,
    pub subject_user_id: i32,
    pub course_id: i32,
    pub course_version_id: i32,
    pub ruleset_version: String,
    pub completed_at_epoch: i64,
    pub evidence_json: String,
    pub created_at: String,
    pub updated_at: String,
}

impl NexusModel for CourseCompletion {
    fn nexus_table() -> &'static str { "course_completions" }
    fn nexus_label() -> &'static str { "Course Completions" }
    fn nexus_icon() -> &'static str { "🏁" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "completion_key", label: "Completion Key", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "subject_user_id", label: "Learner", kind: FieldKind::ForeignKey { table: "users", label_col: "email" }, hidden: false, readonly: true },
            FieldMeta { name: "course_id", label: "Course", kind: FieldKind::ForeignKey { table: "courses", label_col: "title" }, hidden: false, readonly: true },
            FieldMeta { name: "course_version_id", label: "Course Version", kind: FieldKind::ForeignKey { table: "course_versions", label_col: "version_key" }, hidden: false, readonly: true },
            FieldMeta { name: "ruleset_version", label: "Ruleset", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "completed_at_epoch", label: "Completed Epoch", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "evidence_json", label: "Evidence", kind: FieldKind::Json, hidden: false, readonly: true },
        ]
    }
}
"##;

const CERTIFICATE_MODEL: &str = r##"use rullst::db::{FromRow, Orm};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "certificates")]
pub struct Certificate {
    pub id: i32,
    pub certificate_key: String,
    pub completion_id: i32,
    pub status: String,
    pub issued_at_epoch: i64,
    pub revocation_key: Option<String>,
    pub revoked_by: i32,
    pub revoked_at_epoch: i64,
    pub revocation_reason: String,
    pub created_at: String,
    pub updated_at: String,
}

impl NexusModel for Certificate {
    fn nexus_table() -> &'static str { "certificates" }
    fn nexus_label() -> &'static str { "Certificates" }
    fn nexus_icon() -> &'static str { "📜" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "certificate_key", label: "Public Key", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "completion_id", label: "Completion", kind: FieldKind::ForeignKey { table: "course_completions", label_col: "completion_key" }, hidden: false, readonly: true },
            FieldMeta { name: "status", label: "Status", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "issued_at_epoch", label: "Issued Epoch", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "revocation_key", label: "Revocation Key", kind: FieldKind::Text, hidden: true, readonly: true },
            FieldMeta { name: "revoked_by", label: "Revoked By", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "revoked_at_epoch", label: "Revoked Epoch", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "revocation_reason", label: "Revocation Reason", kind: FieldKind::Textarea, hidden: false, readonly: true },
        ]
    }
}
"##;

#[cfg(test)]
mod tests {
    use super::{CERTIFICATE_MODEL, COURSE_COMPLETION_MODEL};

    #[test]
    fn completion_and_certificate_models_are_read_only_in_nexus() {
        assert!(COURSE_COMPLETION_MODEL.contains("Evidence"));
        assert!(CERTIFICATE_MODEL.contains("Public Key"));
        assert!(!COURSE_COMPLETION_MODEL.contains("readonly: false"));
        assert!(!CERTIFICATE_MODEL.contains("readonly: false"));
    }
}
