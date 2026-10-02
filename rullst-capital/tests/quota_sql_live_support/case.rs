//! Case-sensitive quota keys on every live backend, including the 12.1
//! compatibility path and documented migration for MySQL/MariaDB tables of
//! earlier releases.

use rullst_capital::{BillingSubject, QuotaRequest, QuotaStore as _, SqlQuotaStore};

// The earlier release's DDL: key columns take the server's default collation,
// which folds case on MySQL 8 (`utf8mb4_0900_ai_ci`) and MariaDB.
const LEGACY_TABLES: [&str; 2] = [
    "CREATE TABLE rullst_capital_quota_counters (subject_kind VARCHAR(32) NOT NULL, subject_id VARCHAR(128) NOT NULL, feature VARCHAR(128) NOT NULL, used_units BIGINT NOT NULL DEFAULT 0 CHECK (used_units >= 0), PRIMARY KEY (subject_kind, subject_id, feature)) ENGINE=InnoDB",
    "CREATE TABLE rullst_capital_quota_claims (subject_kind VARCHAR(32) NOT NULL, subject_id VARCHAR(128) NOT NULL, feature VARCHAR(128) NOT NULL, event_key VARCHAR(128) NOT NULL, units BIGINT NOT NULL CHECK (units > 0), limit_at_claim BIGINT NOT NULL CHECK (limit_at_claim > 0), used_after BIGINT NOT NULL CHECK (used_after >= 0), claim_token VARCHAR(32) NOT NULL, PRIMARY KEY (subject_kind, subject_id, feature, event_key)) ENGINE=InnoDB",
];

// Keep identical to the migration in rullst-capital/README.md.
const MIGRATION: [&str; 2] = [
    "ALTER TABLE rullst_capital_quota_counters MODIFY subject_kind VARCHAR(32) CHARACTER SET ascii COLLATE ascii_bin NOT NULL, MODIFY subject_id VARCHAR(128) CHARACTER SET ascii COLLATE ascii_bin NOT NULL, MODIFY feature VARCHAR(128) CHARACTER SET ascii COLLATE ascii_bin NOT NULL",
    "ALTER TABLE rullst_capital_quota_claims MODIFY subject_kind VARCHAR(32) CHARACTER SET ascii COLLATE ascii_bin NOT NULL, MODIFY subject_id VARCHAR(128) CHARACTER SET ascii COLLATE ascii_bin NOT NULL, MODIFY feature VARCHAR(128) CHARACTER SET ascii COLLATE ascii_bin NOT NULL, MODIFY event_key VARCHAR(128) CHARACTER SET ascii COLLATE ascii_bin NOT NULL",
];

/// Creates 12.1 tables, proves the store keeps their 12.1 behaviour and takes
/// the warning path, then migrates them and proves the keys become distinct.
pub async fn exercise_legacy_mysql_tables(database_url: &str) {
    let store = SqlQuotaStore::connect(database_url)
        .await
        .expect("legacy SQL quota store");
    for statement in LEGACY_TABLES {
        rullst_orm::sqlx::query(statement)
            .execute(store.pool())
            .await
            .expect("legacy quota table");
    }
    let tenant = BillingSubject::try_new("tenant", "legacy-aB3x").expect("subject");
    let other = BillingSubject::try_new("tenant", "legacy-Ab3X").expect("subject");
    let request =
        QuotaRequest::try_new(tenant.clone(), "projects", "legacy-1", 1, 2).expect("request");

    // A 12.1 table keeps working and is detected (the warning path); keys that
    // differ only by case still share one counter until the migration runs.
    store.prepare_schema().await.expect("legacy quota schema");
    assert_eq!(store.legacy_case_insensitive_keys(), Some(true));
    assert!(!store.reserve(&request).await.unwrap().is_replay());
    assert_eq!(store.usage(&tenant, "projects").await.unwrap(), 1);
    assert_eq!(store.usage(&other, "projects").await.unwrap(), 1);

    for statement in MIGRATION {
        rullst_orm::sqlx::query(statement)
            .execute(store.pool())
            .await
            .expect("documented quota key migration");
    }
    store.pool().close().await;

    let migrated = SqlQuotaStore::connect(database_url)
        .await
        .expect("migrated SQL quota store");
    migrated
        .prepare_schema()
        .await
        .expect("migrated quota schema");
    assert_eq!(migrated.legacy_case_insensitive_keys(), Some(false));
    assert_eq!(migrated.usage(&other, "projects").await.unwrap(), 0);
    let other_request =
        QuotaRequest::try_new(other.clone(), "projects", "legacy-1", 1, 2).expect("request");
    assert!(!migrated.reserve(&other_request).await.unwrap().is_replay());
    assert_eq!(migrated.usage(&tenant, "projects").await.unwrap(), 1);
    assert_eq!(migrated.usage(&other, "projects").await.unwrap(), 1);
    migrated.pool().close().await;
}

/// Tenants, event keys and features that differ only by case stay distinct.
pub async fn exercise_case_sensitive_keys(store: &SqlQuotaStore) {
    let lower = BillingSubject::try_new("workspace", "case-ab3x").expect("subject");
    let upper = BillingSubject::try_new("workspace", "case-AB3X").expect("subject");
    for workspace in [&lower, &upper] {
        let request =
            QuotaRequest::try_new(workspace.clone(), "projects", "case-1", 1, 1).expect("request");
        let grant = store
            .reserve(&request)
            .await
            .expect("each tenant owns its limit");
        assert!(!grant.is_replay());
    }
    assert_eq!(store.usage(&lower, "projects").await.unwrap(), 1);
    assert_eq!(store.usage(&upper, "projects").await.unwrap(), 1);

    let workspace = BillingSubject::try_new("workspace", "case-keys").expect("subject");
    for event_key in ["req-Ab", "req-aB"] {
        let request =
            QuotaRequest::try_new(workspace.clone(), "seats", event_key, 1, 2).expect("request");
        let grant = store
            .reserve(&request)
            .await
            .expect("each event key is its own claim");
        assert!(!grant.is_replay());
    }
    assert_eq!(store.usage(&workspace, "seats").await.unwrap(), 2);
    let feature =
        QuotaRequest::try_new(workspace.clone(), "Seats", "req-Ab", 1, 1).expect("request");
    assert!(!store.reserve(&feature).await.unwrap().is_replay());
    assert_eq!(store.usage(&workspace, "Seats").await.unwrap(), 1);
}
