pub(super) fn get_files() -> Vec<(&'static str, String)> {
    vec![
        ("src/models/school.rs", SCHOOL_MODEL.to_string()),
        (
            "src/models/school_membership.rs",
            SCHOOL_MEMBERSHIP_MODEL.to_string(),
        ),
        (
            "src/models/course_school_scope.rs",
            COURSE_SCHOOL_SCOPE_MODEL.to_string(),
        ),
        ("src/models/cohort.rs", COHORT_MODEL.to_string()),
        (
            "src/models/cohort_membership.rs",
            COHORT_MEMBERSHIP_MODEL.to_string(),
        ),
        (
            "src/models/course_entitlement.rs",
            COURSE_ENTITLEMENT_MODEL.to_string(),
        ),
    ]
}

const SCHOOL_MODEL: &str = r##"use rullst::db::{FromRow, Orm};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "schools")]
pub struct School {
    pub id: i32,
    pub tenant_key: String,
    pub name: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}
"##;

const SCHOOL_MEMBERSHIP_MODEL: &str = r##"use rullst::db::{FromRow, Orm};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "school_memberships")]
pub struct SchoolMembership {
    pub id: i32,
    pub membership_key: String,
    pub school_id: i32,
    pub user_id: i32,
    pub status: String,
    pub is_default: i32,
    pub valid_from_epoch: i64,
    pub expires_at_epoch: i64,
    pub created_at: String,
    pub updated_at: String,
}
"##;

const COURSE_SCHOOL_SCOPE_MODEL: &str = r##"use rullst::db::{FromRow, Orm};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "course_school_scopes")]
pub struct CourseSchoolScope {
    pub id: i32,
    pub school_id: i32,
    pub course_id: i32,
    pub enrollment_policy: String,
    pub created_at: String,
    pub updated_at: String,
}
"##;

const COHORT_MODEL: &str = r##"use rullst::db::{FromRow, Orm};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "cohorts")]
pub struct Cohort {
    pub id: i32,
    pub cohort_key: String,
    pub school_id: i32,
    pub course_id: i32,
    pub name: String,
    pub status: String,
    pub starts_at_epoch: i64,
    pub ends_at_epoch: i64,
    pub created_at: String,
    pub updated_at: String,
}
"##;

const COHORT_MEMBERSHIP_MODEL: &str = r##"use rullst::db::{FromRow, Orm};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "cohort_memberships")]
pub struct CohortMembership {
    pub id: i32,
    pub cohort_id: i32,
    pub school_membership_id: i32,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}
"##;

const COURSE_ENTITLEMENT_MODEL: &str = r##"use rullst::db::{FromRow, Orm};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "course_entitlements")]
pub struct CourseEntitlement {
    pub id: i32,
    pub entitlement_key: String,
    pub school_id: i32,
    pub user_id: i32,
    pub course_id: i32,
    pub source_kind: String,
    pub status: String,
    pub starts_at_epoch: i64,
    pub expires_at_epoch: i64,
    pub created_at: String,
    pub updated_at: String,
}
"##;
