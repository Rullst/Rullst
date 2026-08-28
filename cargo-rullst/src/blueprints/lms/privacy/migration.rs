pub(super) const PRIVACY_MIGRATION: &str = r##"use rullst::db::{Orm, sqlx};
use rullst::db::async_trait;
use rullst::db::schema::{Migration, Schema};

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {
    fn name(&self) -> &'static str {
        "m20260906000000_add_privacy_lifecycle"
    }

    async fn up(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::create("privacy_subject_policies", |table| {
            table.id();
            table.string("policy_key").not_null();
            table.integer("school_id").not_null();
            table.integer("subject_user_id").not_null();
            table.string("age_band").not_null();
            table.string("policy_version").not_null();
            table.big_integer("retention_until_epoch").not_null();
            table.string("status").not_null();
            table.timestamps();
        }).await?;
        Schema::create("guardian_consents", |table| {
            table.id();
            table.string("consent_key").not_null();
            table.integer("school_id").not_null();
            table.integer("subject_user_id").not_null();
            table.integer("guardian_user_id").not_null();
            table.string("purpose").not_null();
            table.string("policy_version").not_null();
            table.string("status").not_null();
            table.big_integer("granted_at_epoch").not_null();
            table.big_integer("revoked_at_epoch").not_null();
            table.timestamps();
        }).await?;
        Schema::create("privacy_requests", |table| {
            table.id();
            table.string("request_key").not_null();
            table.integer("school_id").not_null();
            table.integer("subject_user_id").not_null();
            table.integer("requested_by_user_id").not_null();
            table.string("request_kind").not_null();
            table.string("status").not_null();
            table.integer("attempts").not_null();
            table.string("claim_key").not_null();
            table.big_integer("claim_expires_at_epoch").not_null();
            table.big_integer("available_at_epoch").not_null();
            table.integer("processed_by_user_id").not_null();
            table.string("last_error_code").not_null();
            table.big_integer("requested_at_epoch").not_null();
            table.big_integer("completed_at_epoch").not_null();
            table.string("result_digest").not_null();
            table.timestamps();
        }).await?;

        let pool = Orm::pool()?;
        for statement in [
            "CREATE UNIQUE INDEX privacy_subject_policy_key_unique ON privacy_subject_policies(policy_key)",
            "CREATE INDEX privacy_subject_school_status_idx ON privacy_subject_policies(school_id, subject_user_id, status)",
            "CREATE UNIQUE INDEX guardian_consent_key_unique ON guardian_consents(consent_key)",
            "CREATE INDEX guardian_consent_school_subject_idx ON guardian_consents(school_id, subject_user_id, purpose, policy_version, status)",
            "CREATE UNIQUE INDEX privacy_request_key_unique ON privacy_requests(request_key)",
            "CREATE INDEX privacy_request_school_subject_idx ON privacy_requests(school_id, subject_user_id, status)",
            "CREATE INDEX privacy_request_school_delivery_idx ON privacy_requests(school_id, status, available_at_epoch, requested_at_epoch)",
        ] {
            sqlx::query(sqlx::AssertSqlSafe(statement)).execute(pool).await?;
        }
        Ok(())
    }

    async fn down(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::drop_if_exists("privacy_requests").await?;
        Schema::drop_if_exists("guardian_consents").await?;
        Schema::drop_if_exists("privacy_subject_policies").await
    }
}
"##;
