//! MySQL/MariaDB statements for [`super::SqlChatMemory`].
//!
//! Tenant and conversation IDs are case-sensitive ASCII tokens, but the default
//! MySQL/MariaDB collations fold case. New tables therefore declare both key
//! columns `CHARACTER SET ascii COLLATE ascii_bin`. `prepare_schema` never
//! alters an existing table; one created by an earlier release keeps its
//! collation until the operator applies the documented migration, and every
//! tenant-scoped statement below also compares the key byte-exactly so that
//! such a table fails closed instead of merging IDs that differ only by case.

pub(super) const SESSIONS_TABLE: &str = "CREATE TABLE IF NOT EXISTS rullst_ai_chat_sessions (tenant_id VARCHAR(128) CHARACTER SET ascii COLLATE ascii_bin NOT NULL, conversation_id VARCHAR(128) CHARACTER SET ascii COLLATE ascii_bin NOT NULL, conversation_revision BIGINT NOT NULL DEFAULT 0, created_at_epoch BIGINT NOT NULL, PRIMARY KEY (tenant_id, conversation_id), CHECK (conversation_revision >= 0 AND MOD(conversation_revision, 2) = 0)) ENGINE=InnoDB";
pub(super) const MESSAGES_TABLE: &str = "CREATE TABLE IF NOT EXISTS rullst_ai_chat_messages (tenant_id VARCHAR(128) CHARACTER SET ascii COLLATE ascii_bin NOT NULL, conversation_id VARCHAR(128) CHARACTER SET ascii COLLATE ascii_bin NOT NULL, turn_sequence BIGINT NOT NULL, role VARCHAR(16) NOT NULL CHECK (role IN ('user', 'assistant')), content MEDIUMTEXT NOT NULL, created_at_epoch BIGINT NOT NULL, PRIMARY KEY (tenant_id, conversation_id, turn_sequence), FOREIGN KEY (tenant_id, conversation_id) REFERENCES rullst_ai_chat_sessions (tenant_id, conversation_id) ON DELETE CASCADE) ENGINE=InnoDB";

/// Indexed key equality followed by a byte-exact comparison of the same key.
macro_rules! mysql_exact_key {
    () => {
        "tenant_id = ? AND conversation_id = ? \
         AND CAST(tenant_id AS BINARY) = CAST(? AS BINARY) \
         AND CAST(conversation_id AS BINARY) = CAST(? AS BINARY)"
    };
}

pub(super) const ENSURE: &str = "INSERT INTO rullst_ai_chat_sessions (tenant_id, conversation_id, conversation_revision, created_at_epoch) VALUES (?, ?, 0, ?) ON DUPLICATE KEY UPDATE tenant_id = tenant_id";
pub(super) const EXACT_SESSION: &str = concat!(
    "SELECT COUNT(*) FROM rullst_ai_chat_sessions WHERE ",
    mysql_exact_key!()
);
pub(super) const REVISION: &str = concat!(
    "SELECT conversation_revision FROM rullst_ai_chat_sessions WHERE ",
    mysql_exact_key!()
);
pub(super) const HISTORY: &str = concat!(
    "SELECT turn_sequence, role, content, created_at_epoch FROM rullst_ai_chat_messages WHERE ",
    mysql_exact_key!(),
    " AND turn_sequence <= ? ORDER BY turn_sequence DESC LIMIT ?"
);
pub(super) const ADVANCE: &str = concat!(
    "UPDATE rullst_ai_chat_sessions SET conversation_revision = ? WHERE ",
    mysql_exact_key!(),
    " AND conversation_revision = ?"
);
pub(super) const DELETE_MESSAGES: &str = concat!(
    "DELETE FROM rullst_ai_chat_messages WHERE ",
    mysql_exact_key!()
);
pub(super) const DELETE_SESSION: &str = concat!(
    "DELETE FROM rullst_ai_chat_sessions WHERE ",
    mysql_exact_key!()
);
/// Returned when a legacy case-insensitive MySQL/MariaDB table already holds a
/// key that differs from the requested one only by letter case.
pub(super) const LEGACY_COLLATION: &str = "MySQL/MariaDB chat-memory keys compare case-insensitively; migrate tenant_id and conversation_id to CHARACTER SET ascii COLLATE ascii_bin";

#[cfg(test)]
mod tests {
    use super::*;

    // TM-AI-08: tenant and conversation IDs that differ only by case stay
    // distinct keys on MySQL/MariaDB.
    #[test]
    fn mysql_keys_are_binary_and_every_scoped_statement_compares_exactly() {
        for table in [SESSIONS_TABLE, MESSAGES_TABLE] {
            for column in ["tenant_id", "conversation_id"] {
                let declaration =
                    format!("{column} VARCHAR(128) CHARACTER SET ascii COLLATE ascii_bin NOT NULL");
                assert!(table.contains(&declaration), "{column} DDL");
            }
        }
        for statement in [
            EXACT_SESSION,
            REVISION,
            HISTORY,
            ADVANCE,
            DELETE_MESSAGES,
            DELETE_SESSION,
        ] {
            assert!(
                statement.contains(
                    "tenant_id = ? AND conversation_id = ? \
                     AND CAST(tenant_id AS BINARY) = CAST(? AS BINARY) \
                     AND CAST(conversation_id AS BINARY) = CAST(? AS BINARY)"
                ),
                "{statement}"
            );
        }
        assert!(!ENSURE.contains("IGNORE"));
        assert!(ENSURE.ends_with("ON DUPLICATE KEY UPDATE tenant_id = tenant_id"));
    }
}
