// Gamification and automation model templates for the LMS starter.

pub fn get_files() -> Vec<(&'static str, String)> {
    vec![
        ("src/models/achievement.rs", ACHIEVEMENT.to_string()),
        (
            "src/models/leaderboard_entry.rs",
            LEADERBOARD_ENTRY.to_string(),
        ),
        ("src/models/automation_rule.rs", AUTOMATION_RULE.to_string()),
    ]
}

const ACHIEVEMENT: &str = r##"use rullst::db::{FromRow, Orm};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "achievements")]
pub struct Achievement {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub description: String,
    pub xp_reward: i32,
    pub enabled: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl NexusModel for Achievement {
    fn nexus_table() -> &'static str { "achievements" }
    fn nexus_label() -> &'static str { "Achievements" }
    fn nexus_icon() -> &'static str { "🏆" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "code", label: "Stable Code", kind: FieldKind::Text, hidden: false, readonly: false },
            FieldMeta { name: "name", label: "Name", kind: FieldKind::Text, hidden: false, readonly: false },
            FieldMeta { name: "description", label: "Description", kind: FieldKind::Textarea, hidden: false, readonly: false },
            FieldMeta { name: "xp_reward", label: "XP Reward", kind: FieldKind::Number, hidden: false, readonly: false },
            FieldMeta { name: "enabled", label: "Enabled", kind: FieldKind::Boolean, hidden: false, readonly: false },
            FieldMeta { name: "created_at", label: "Created At", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "updated_at", label: "Updated At", kind: FieldKind::Text, hidden: false, readonly: true },
        ]
    }
}
"##;

const LEADERBOARD_ENTRY: &str = r##"use rullst::db::{FromRow, Orm};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "leaderboard_entries")]
pub struct LeaderboardEntry {
    pub id: i32,
    pub user_id: i32,
    pub course_id: i32,
    pub season_key: String,
    pub score: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl NexusModel for LeaderboardEntry {
    fn nexus_table() -> &'static str { "leaderboard_entries" }
    fn nexus_label() -> &'static str { "Leaderboard" }
    fn nexus_icon() -> &'static str { "🥇" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "user_id", label: "Learner", kind: FieldKind::ForeignKey { table: "users", label_col: "email" }, hidden: false, readonly: true },
            FieldMeta { name: "course_id", label: "Course", kind: FieldKind::ForeignKey { table: "courses", label_col: "title" }, hidden: false, readonly: true },
            FieldMeta { name: "season_key", label: "Season", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "score", label: "Authoritative Score", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "created_at", label: "Created At", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "updated_at", label: "Updated At", kind: FieldKind::Text, hidden: false, readonly: true },
        ]
    }
}
"##;

const AUTOMATION_RULE: &str = r##"use rullst::db::{FromRow, Orm};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "automation_rules")]
pub struct AutomationRule {
    pub id: i32,
    pub school_id: i32,
    pub name: String,
    pub trigger_kind: String,
    pub action_kind: String,
    pub config_json: String,
    pub enabled: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl NexusModel for AutomationRule {
    fn nexus_table() -> &'static str { "automation_rules" }
    fn nexus_label() -> &'static str { "Automation Rules" }
    fn nexus_icon() -> &'static str { "⚙️" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "school_id", label: "School", kind: FieldKind::ForeignKey { table: "schools", label_col: "name" }, hidden: false, readonly: true },
            FieldMeta { name: "name", label: "Name", kind: FieldKind::Text, hidden: false, readonly: false },
            FieldMeta { name: "trigger_kind", label: "Trigger", kind: FieldKind::Text, hidden: false, readonly: false },
            FieldMeta { name: "action_kind", label: "Action", kind: FieldKind::Text, hidden: false, readonly: false },
            FieldMeta { name: "config_json", label: "Versioned Configuration", kind: FieldKind::Json, hidden: false, readonly: false },
            FieldMeta { name: "enabled", label: "Enabled", kind: FieldKind::Boolean, hidden: false, readonly: false },
            FieldMeta { name: "created_at", label: "Created At", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "updated_at", label: "Updated At", kind: FieldKind::Text, hidden: false, readonly: true },
        ]
    }
}
"##;

#[cfg(test)]
mod tests {
    use super::get_files;

    #[test]
    fn derived_scores_are_readonly_in_generated_admin_metadata() {
        let files = get_files();
        let leaderboard = files
            .iter()
            .find(|(path, _)| *path == "src/models/leaderboard_entry.rs")
            .map(|(_, source)| source.as_str());

        assert!(leaderboard.is_some_and(|source| source.contains(
            "name: \"score\", label: \"Authoritative Score\", kind: FieldKind::Number, hidden: false, readonly: true"
        )));
    }
}
