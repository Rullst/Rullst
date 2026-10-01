//! MySQL/MariaDB statements for [`super::SqlQuotaStore`].
//!
//! Subjects, features and event keys are case-sensitive ASCII identifiers, but
//! the default MySQL/MariaDB collations fold case. New tables therefore declare
//! every key column `CHARACTER SET ascii COLLATE ascii_bin`. The store never
//! alters an existing table: before its first statement it counts the key
//! columns that compare case-sensitively and fails closed on a table created
//! by an earlier release, so different tenants or event keys can never share
//! one counter or claim.

pub(super) const COUNTERS: &str = "CREATE TABLE IF NOT EXISTS rullst_capital_quota_counters (subject_kind VARCHAR(32) CHARACTER SET ascii COLLATE ascii_bin NOT NULL, subject_id VARCHAR(128) CHARACTER SET ascii COLLATE ascii_bin NOT NULL, feature VARCHAR(128) CHARACTER SET ascii COLLATE ascii_bin NOT NULL, used_units BIGINT NOT NULL DEFAULT 0 CHECK (used_units >= 0), PRIMARY KEY (subject_kind, subject_id, feature)) ENGINE=InnoDB";
pub(super) const CLAIMS: &str = "CREATE TABLE IF NOT EXISTS rullst_capital_quota_claims (subject_kind VARCHAR(32) CHARACTER SET ascii COLLATE ascii_bin NOT NULL, subject_id VARCHAR(128) CHARACTER SET ascii COLLATE ascii_bin NOT NULL, feature VARCHAR(128) CHARACTER SET ascii COLLATE ascii_bin NOT NULL, event_key VARCHAR(128) CHARACTER SET ascii COLLATE ascii_bin NOT NULL, units BIGINT NOT NULL CHECK (units > 0), limit_at_claim BIGINT NOT NULL CHECK (limit_at_claim > 0), used_after BIGINT NOT NULL CHECK (used_after >= 0), claim_token VARCHAR(32) NOT NULL, PRIMARY KEY (subject_kind, subject_id, feature, event_key)) ENGINE=InnoDB";
pub(super) const INSERT_CLAIM: &str = "INSERT INTO rullst_capital_quota_claims (subject_kind, subject_id, feature, event_key, units, limit_at_claim, used_after, claim_token) VALUES (?, ?, ?, ?, ?, ?, ?, ?) ON DUPLICATE KEY UPDATE subject_id = VALUES(subject_id)";
pub(super) const INSERT_COUNTER: &str = "INSERT INTO rullst_capital_quota_counters (subject_kind, subject_id, feature, used_units) VALUES (?, ?, ?, ?) ON DUPLICATE KEY UPDATE subject_id = VALUES(subject_id)";

/// Counts the quota key columns that compare byte-exactly: a binary string
/// type, or a binary (`_bin`) or case-sensitive (`_cs`) collation.
pub(super) const CASE_SENSITIVE_KEY_COLUMNS: &str = "SELECT COUNT(*) FROM information_schema.COLUMNS WHERE TABLE_SCHEMA = DATABASE() AND ((TABLE_NAME = 'rullst_capital_quota_counters' AND COLUMN_NAME IN ('subject_kind', 'subject_id', 'feature')) OR (TABLE_NAME = 'rullst_capital_quota_claims' AND COLUMN_NAME IN ('subject_kind', 'subject_id', 'feature', 'event_key'))) AND (DATA_TYPE IN ('binary', 'varbinary') OR RIGHT(COLLATION_NAME, 4) = '_bin' OR RIGHT(COLLATION_NAME, 3) = '_cs')";
/// Three counter key columns plus four claim key columns.
pub(super) const KEY_COLUMNS: i64 = 7;

#[cfg(test)]
mod tests {
    use super::*;

    // Tenants or event keys that differ only by case stay distinct on MySQL/MariaDB.
    #[test]
    fn mysql_quota_keys_are_binary_and_legacy_tables_are_detected() {
        for (table, columns) in [
            (COUNTERS, &["subject_kind", "subject_id", "feature"][..]),
            (
                CLAIMS,
                &["subject_kind", "subject_id", "feature", "event_key"][..],
            ),
        ] {
            for column in columns {
                let width = if *column == "subject_kind" { 32 } else { 128 };
                let declaration = format!(
                    "{column} VARCHAR({width}) CHARACTER SET ascii COLLATE ascii_bin NOT NULL"
                );
                assert!(table.contains(&declaration), "{column} DDL");
                assert!(CASE_SENSITIVE_KEY_COLUMNS.contains(&format!("'{column}'")));
            }
        }
        assert_eq!(KEY_COLUMNS, 3 + 4);
        assert!(CASE_SENSITIVE_KEY_COLUMNS.contains("TABLE_SCHEMA = DATABASE()"));
    }
}
