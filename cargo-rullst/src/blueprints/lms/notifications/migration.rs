pub(super) const NOTIFICATION_MIGRATION: &str = r##"use rullst::db::{Orm, sqlx};
use rullst::db::async_trait;
use rullst::db::schema::{Migration, Schema};

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {
    fn name(&self) -> &'static str { "m20260830000000_add_notifications" }

    async fn up(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::create("notification_preferences", |table| {
            table.id();
            table.integer("school_id").not_null();
            table.integer("user_id").not_null();
            table.string("channel").not_null();
            table.boolean("enabled").not_null();
            table.string("locale").not_null();
            table.timestamps();
        }).await?;
        Schema::create("notifications", |table| {
            table.id();
            table.integer("school_id").not_null();
            table.string("notification_key").not_null();
            table.integer("user_id").not_null();
            table.string("channel").not_null();
            table.string("locale").not_null();
            table.string("localization_key").not_null();
            table.string("payload_json").not_null();
            table.string("status").not_null();
            table.string("source_event_key").not_null();
            table.string("read_at").not_null();
            table.timestamps();
        }).await?;
        let pool = Orm::pool()?;
        for statement in [
            "CREATE UNIQUE INDEX notification_preferences_school_user_channel_unique ON notification_preferences(school_id, user_id, channel)",
            "CREATE UNIQUE INDEX notifications_key_unique ON notifications(notification_key)",
            "CREATE INDEX notifications_school_user_status_idx ON notifications(school_id, user_id, status, created_at)",
            "CREATE INDEX notifications_source_idx ON notifications(source_event_key)",
        ] {
            sqlx::query(sqlx::AssertSqlSafe(statement)).execute(pool).await?;
        }
        Ok(())
    }

    async fn down(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::drop_if_exists("notifications").await?;
        Schema::drop_if_exists("notification_preferences").await
    }
}
"##;
