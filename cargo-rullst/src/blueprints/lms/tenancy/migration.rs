pub(super) const SCHOOL_TENANCY_MIGRATION: &str = r##"use rullst::db::{Orm, sqlx};
use rullst::db::async_trait;
use rullst::db::schema::{Migration, Schema};

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {
    fn name(&self) -> &'static str { "m20260901500000_add_school_tenancy" }

    async fn up(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::create("schools", |table| {
            table.id(); table.string("tenant_key").not_null(); table.string("name").not_null();
            table.string("status").not_null(); table.timestamps();
        }).await?;
        Schema::create("school_memberships", |table| {
            table.id(); table.string("membership_key").not_null(); table.integer("school_id").not_null();
            table.integer("user_id").not_null(); table.string("status").not_null();
            table.integer("is_default").not_null(); table.big_integer("valid_from_epoch").not_null();
            table.big_integer("expires_at_epoch").not_null(); table.timestamps();
        }).await?;
        Schema::create("course_school_scopes", |table| {
            table.id(); table.integer("school_id").not_null(); table.integer("course_id").not_null();
            table.string("enrollment_policy").not_null(); table.timestamps();
        }).await?;
        Schema::create("cohorts", |table| {
            table.id(); table.string("cohort_key").not_null(); table.integer("school_id").not_null();
            table.integer("course_id").not_null(); table.string("name").not_null();
            table.string("status").not_null(); table.big_integer("starts_at_epoch").not_null();
            table.big_integer("ends_at_epoch").not_null(); table.timestamps();
        }).await?;
        Schema::create("cohort_memberships", |table| {
            table.id(); table.integer("cohort_id").not_null();
            table.integer("school_membership_id").not_null(); table.string("status").not_null();
            table.timestamps();
        }).await?;
        Schema::create("course_entitlements", |table| {
            table.id(); table.string("entitlement_key").not_null(); table.integer("school_id").not_null();
            table.integer("user_id").not_null(); table.integer("course_id").not_null();
            table.string("source_kind").not_null(); table.string("status").not_null();
            table.big_integer("starts_at_epoch").not_null(); table.big_integer("expires_at_epoch").not_null();
            table.timestamps();
        }).await?;

        let pool = Orm::pool()?;
        for statement in [
            "CREATE UNIQUE INDEX schools_tenant_key_unique ON schools(tenant_key)",
            "CREATE UNIQUE INDEX school_memberships_key_unique ON school_memberships(membership_key)",
            "CREATE UNIQUE INDEX school_memberships_user_school_unique ON school_memberships(user_id, school_id)",
            "CREATE INDEX school_memberships_active_idx ON school_memberships(user_id, status, valid_from_epoch, expires_at_epoch, is_default)",
            "CREATE UNIQUE INDEX course_school_scopes_course_unique ON course_school_scopes(course_id)",
            "CREATE INDEX course_school_scopes_school_idx ON course_school_scopes(school_id, course_id)",
            "CREATE UNIQUE INDEX cohorts_key_unique ON cohorts(cohort_key)",
            "CREATE INDEX cohorts_school_course_idx ON cohorts(school_id, course_id, status)",
            "CREATE UNIQUE INDEX cohort_memberships_unique ON cohort_memberships(cohort_id, school_membership_id)",
            "CREATE UNIQUE INDEX course_entitlements_key_unique ON course_entitlements(entitlement_key)",
            "CREATE UNIQUE INDEX course_entitlements_subject_unique ON course_entitlements(school_id, user_id, course_id, source_kind)",
            "CREATE INDEX course_entitlements_active_idx ON course_entitlements(school_id, user_id, course_id, status, starts_at_epoch, expires_at_epoch)",
        ] { sqlx::query(sqlx::AssertSqlSafe(statement)).execute(pool).await?; }

        for fixture in [
            "INSERT INTO schools (id, tenant_key, name, status, created_at, updated_at) VALUES (1, 'academy-demo', 'Rullst Academy Demo', 'active', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
            "INSERT INTO schools (id, tenant_key, name, status, created_at, updated_at) VALUES (2, 'academy-rival', 'Independent Rival School', 'active', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
            "INSERT INTO course_school_scopes (school_id, course_id, enrollment_policy, created_at, updated_at) VALUES (1, 1, 'open', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
            "INSERT INTO course_school_scopes (school_id, course_id, enrollment_policy, created_at, updated_at) VALUES (2, 2, 'entitled', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
            "INSERT INTO cohorts (id, cohort_key, school_id, course_id, name, status, starts_at_epoch, ends_at_epoch, created_at, updated_at) VALUES (1, 'demo-2026', 1, 1, 'Demo Cohort 2026', 'active', 1, 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        ] { sqlx::query(sqlx::AssertSqlSafe(fixture)).execute(pool).await?; }
        Ok(())
    }

    async fn down(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::drop_if_exists("course_entitlements").await?;
        Schema::drop_if_exists("cohort_memberships").await?;
        Schema::drop_if_exists("cohorts").await?;
        Schema::drop_if_exists("course_school_scopes").await?;
        Schema::drop_if_exists("school_memberships").await?;
        Schema::drop_if_exists("schools").await
    }
}
"##;
