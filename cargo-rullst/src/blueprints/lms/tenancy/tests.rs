use super::{migration::SCHOOL_TENANCY_MIGRATION, models, service::SCHOOL_SERVICE};

#[test]
fn tenancy_schema_and_service_are_fail_closed() {
    let files = models::get_files();
    assert_eq!(files.len(), 6);
    for required in [
        "schools_tenant_key_unique",
        "school_memberships_user_school_unique",
        "course_school_scopes_course_unique",
        "cohort_memberships_unique",
        "course_entitlements_subject_unique",
        "course_entitlements_active_idx",
    ] {
        assert!(SCHOOL_TENANCY_MIGRATION.contains(required));
    }
    assert!(SCHOOL_SERVICE.contains("resolve_membership_at"));
    assert!(SCHOOL_SERVICE.contains("AmbiguousMembership"));
    assert!(SCHOOL_SERVICE.contains("authorize_course_enrollment_at"));
    assert!(SCHOOL_SERVICE.contains("authorize_tenant("));
    assert!(SCHOOL_SERVICE.contains("s.tenant_key = $2"));
    assert!(!SCHOOL_SERVICE.contains("unwrap()"));
    assert!(!SCHOOL_SERVICE.contains("expect("));
}
