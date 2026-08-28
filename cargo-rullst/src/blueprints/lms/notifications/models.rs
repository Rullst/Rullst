pub(super) const NOTIFICATION_MODEL: &str = r##"use rullst::db::{FromRow, Orm};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "notifications")]
pub struct Notification {
    pub id: i32,
    pub school_id: i32,
    pub notification_key: String,
    pub user_id: i32,
    pub channel: String,
    pub locale: String,
    pub localization_key: String,
    pub payload_json: String,
    pub status: String,
    pub source_event_key: String,
    pub read_at: String,
    pub created_at: String,
    pub updated_at: String,
}

impl NexusModel for Notification {
    fn nexus_table() -> &'static str { "notifications" }
    fn nexus_label() -> &'static str { "Notifications" }
    fn nexus_icon() -> &'static str { "🔔" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "school_id", label: "School", kind: FieldKind::ForeignKey { table: "schools", label_col: "name" }, hidden: false, readonly: true },
            FieldMeta { name: "notification_key", label: "Notification Key", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "user_id", label: "Learner", kind: FieldKind::ForeignKey { table: "users", label_col: "email" }, hidden: false, readonly: true },
            FieldMeta { name: "channel", label: "Channel", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "locale", label: "Locale", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "localization_key", label: "Localization Key", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "payload_json", label: "Payload", kind: FieldKind::Json, hidden: false, readonly: true },
            FieldMeta { name: "status", label: "Status", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "source_event_key", label: "Source Event", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "read_at", label: "Read At", kind: FieldKind::Text, hidden: false, readonly: true },
        ]
    }
}
"##;

pub(super) const NOTIFICATION_PREFERENCE_MODEL: &str = r##"use rullst::db::{FromRow, Orm};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "notification_preferences")]
pub struct NotificationPreference {
    pub id: i32,
    pub school_id: i32,
    pub user_id: i32,
    pub channel: String,
    pub enabled: i32,
    pub locale: String,
    pub created_at: String,
    pub updated_at: String,
}

impl NexusModel for NotificationPreference {
    fn nexus_table() -> &'static str { "notification_preferences" }
    fn nexus_label() -> &'static str { "Notification Preferences" }
    fn nexus_icon() -> &'static str { "⚙️" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "school_id", label: "School", kind: FieldKind::ForeignKey { table: "schools", label_col: "name" }, hidden: false, readonly: true },
            FieldMeta { name: "user_id", label: "Learner", kind: FieldKind::ForeignKey { table: "users", label_col: "email" }, hidden: false, readonly: true },
            FieldMeta { name: "channel", label: "Channel", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "enabled", label: "Enabled", kind: FieldKind::Boolean, hidden: false, readonly: true },
            FieldMeta { name: "locale", label: "Locale", kind: FieldKind::Text, hidden: false, readonly: true },
        ]
    }
}
"##;
