//! MySQL/MariaDB statements for [`super::SqlQuotaStore`].
//!
//! Subjects, features and event keys are case-sensitive ASCII identifiers, but
//! the default MySQL/MariaDB collations fold case. New tables therefore declare
//! every key column `CHARACTER SET ascii COLLATE ascii_bin`. The store never
//! alters an existing table. A table created by 12.1 or earlier keeps the
//! 12.1 behaviour, in which keys that differ only by case share one counter or
//! claim; the store detects it once and logs a warning that names the
//! documented `ALTER TABLE` migration.

pub(super) const COUNTERS: &str = "CREATE TABLE IF NOT EXISTS rullst_capital_quota_counters (subject_kind VARCHAR(32) CHARACTER SET ascii COLLATE ascii_bin NOT NULL, subject_id VARCHAR(128) CHARACTER SET ascii COLLATE ascii_bin NOT NULL, feature VARCHAR(128) CHARACTER SET ascii COLLATE ascii_bin NOT NULL, used_units BIGINT NOT NULL DEFAULT 0 CHECK (used_units >= 0), PRIMARY KEY (subject_kind, subject_id, feature)) ENGINE=InnoDB";
pub(super) const CLAIMS: &str = "CREATE TABLE IF NOT EXISTS rullst_capital_quota_claims (subject_kind VARCHAR(32) CHARACTER SET ascii COLLATE ascii_bin NOT NULL, subject_id VARCHAR(128) CHARACTER SET ascii COLLATE ascii_bin NOT NULL, feature VARCHAR(128) CHARACTER SET ascii COLLATE ascii_bin NOT NULL, event_key VARCHAR(128) CHARACTER SET ascii COLLATE ascii_bin NOT NULL, units BIGINT NOT NULL CHECK (units > 0), limit_at_claim BIGINT NOT NULL CHECK (limit_at_claim > 0), used_after BIGINT NOT NULL CHECK (used_after >= 0), claim_token VARCHAR(32) NOT NULL, PRIMARY KEY (subject_kind, subject_id, feature, event_key)) ENGINE=InnoDB";
pub(super) const INSERT_CLAIM: &str = "INSERT INTO rullst_capital_quota_claims (subject_kind, subject_id, feature, event_key, units, limit_at_claim, used_after, claim_token) VALUES (?, ?, ?, ?, ?, ?, ?, ?) ON DUPLICATE KEY UPDATE subject_id = VALUES(subject_id)";
pub(super) const INSERT_COUNTER: &str = "INSERT INTO rullst_capital_quota_counters (subject_kind, subject_id, feature, used_units) VALUES (?, ?, ?, ?) ON DUPLICATE KEY UPDATE subject_id = VALUES(subject_id)";

/// Counts the quota key columns that exist and, of those, the ones that
/// compare byte-exactly: a binary string type, or a binary (`_bin`) or
/// case-sensitive (`_cs`) collation.
pub(super) const KEY_COLUMN_COLLATIONS: &str = "SELECT COUNT(*), CAST(COALESCE(SUM(CASE WHEN DATA_TYPE IN ('binary', 'varbinary') OR RIGHT(COLLATION_NAME, 4) = '_bin' OR RIGHT(COLLATION_NAME, 3) = '_cs' THEN 1 ELSE 0 END), 0) AS SIGNED) FROM information_schema.COLUMNS WHERE TABLE_SCHEMA = DATABASE() AND ((TABLE_NAME = 'rullst_capital_quota_counters' AND COLUMN_NAME IN ('subject_kind', 'subject_id', 'feature')) OR (TABLE_NAME = 'rullst_capital_quota_claims' AND COLUMN_NAME IN ('subject_kind', 'subject_id', 'feature', 'event_key')))";
/// Three counter key columns plus four claim key columns.
pub(super) const KEY_COLUMNS: i64 = 7;

/// How the MySQL/MariaDB quota key columns compare.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum KeyCollation {
    /// Every key column compares byte-exactly, as in tables created by 12.2.
    CaseSensitive,
    /// A key column folds case (a table created by 12.1 or earlier), so keys
    /// that differ only by case share one counter or claim.
    LegacyCaseInsensitive,
}

impl KeyCollation {
    /// Classifies the counts returned by [`KEY_COLUMN_COLLATIONS`]. `None`
    /// means the tables do not exist yet, or only partly, so nothing is known.
    pub(super) fn classify(present: i64, case_sensitive: i64) -> Option<Self> {
        if case_sensitive < present {
            Some(Self::LegacyCaseInsensitive)
        } else if present == KEY_COLUMNS {
            Some(Self::CaseSensitive)
        } else {
            None
        }
    }
}

/// Records the first classification of a store and reports whether it is the
/// one that must log the legacy-collation warning, so each store warns once.
pub(super) fn record_key_collation(
    recorded: &std::sync::OnceLock<KeyCollation>,
    observed: KeyCollation,
) -> bool {
    recorded.set(observed).is_ok() && observed == KeyCollation::LegacyCaseInsensitive
}

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
                assert!(KEY_COLUMN_COLLATIONS.contains(&format!("'{column}'")));
            }
        }
        assert_eq!(KEY_COLUMNS, 3 + 4);
        assert!(KEY_COLUMN_COLLATIONS.contains("TABLE_SCHEMA = DATABASE()"));
    }

    // 12.1 tables keep working; only a complete, byte-exact schema is quiet.
    #[test]
    fn key_collation_counts_classify_new_legacy_and_missing_tables() {
        assert_eq!(
            KeyCollation::classify(7, 7),
            Some(KeyCollation::CaseSensitive)
        );
        for case_sensitive in [0, 3, 6] {
            assert_eq!(
                KeyCollation::classify(7, case_sensitive),
                Some(KeyCollation::LegacyCaseInsensitive)
            );
        }
        assert_eq!(
            KeyCollation::classify(3, 0),
            Some(KeyCollation::LegacyCaseInsensitive)
        );
        // Tables not created yet, or created only in part, decide nothing.
        assert_eq!(KeyCollation::classify(0, 0), None);
        assert_eq!(KeyCollation::classify(3, 3), None);
    }

    #[test]
    fn a_store_warns_once_and_only_for_legacy_tables() {
        let legacy = std::sync::OnceLock::new();
        assert!(record_key_collation(
            &legacy,
            KeyCollation::LegacyCaseInsensitive
        ));
        assert!(!record_key_collation(
            &legacy,
            KeyCollation::LegacyCaseInsensitive
        ));
        assert_eq!(legacy.get(), Some(&KeyCollation::LegacyCaseInsensitive));

        let migrated = std::sync::OnceLock::new();
        assert!(!record_key_collation(
            &migrated,
            KeyCollation::CaseSensitive
        ));
        assert!(!record_key_collation(
            &migrated,
            KeyCollation::LegacyCaseInsensitive
        ));
        assert_eq!(migrated.get(), Some(&KeyCollation::CaseSensitive));
    }
}
