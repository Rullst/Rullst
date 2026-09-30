//! SQL statements of the durable outbox, per dialect.

pub(super) const POSTGRES_TABLE: &str = "CREATE TABLE IF NOT EXISTS rullst_outbox (id BIGSERIAL PRIMARY KEY, stream VARCHAR(128) NOT NULL, event_key VARCHAR(128) NOT NULL, event_kind VARCHAR(128) NOT NULL, payload_json TEXT NOT NULL, status VARCHAR(16) NOT NULL, attempts INTEGER NOT NULL, claimed_by VARCHAR(128) NOT NULL, claim_key VARCHAR(128) NOT NULL, claim_expires_at_epoch BIGINT NOT NULL, last_error VARCHAR(512) NOT NULL, available_at_epoch BIGINT NOT NULL, created_at_epoch BIGINT NOT NULL, delivered_at_epoch BIGINT, insert_token VARCHAR(32) NOT NULL, UNIQUE (stream, event_key))";
// MySQL/MariaDB default collations fold case; keys are case-sensitive ASCII.
pub(super) const MYSQL_TABLE: &str = "CREATE TABLE IF NOT EXISTS rullst_outbox (id BIGINT AUTO_INCREMENT PRIMARY KEY, stream VARCHAR(128) CHARACTER SET ascii COLLATE ascii_bin NOT NULL, event_key VARCHAR(128) CHARACTER SET ascii COLLATE ascii_bin NOT NULL, event_kind VARCHAR(128) NOT NULL, payload_json LONGTEXT NOT NULL, status VARCHAR(16) NOT NULL, attempts INT NOT NULL, claimed_by VARCHAR(128) NOT NULL, claim_key VARCHAR(128) CHARACTER SET ascii COLLATE ascii_bin NOT NULL, claim_expires_at_epoch BIGINT NOT NULL, last_error VARCHAR(512) NOT NULL, available_at_epoch BIGINT NOT NULL, created_at_epoch BIGINT NOT NULL, delivered_at_epoch BIGINT NULL, insert_token VARCHAR(32) NOT NULL, UNIQUE KEY rullst_outbox_stream_event_unique (stream, event_key), INDEX rullst_outbox_delivery_idx (stream, status, available_at_epoch, claim_expires_at_epoch, id))";
pub(super) const MYSQL_KEY_COLLATION_CHECK: &str = "SELECT COUNT(*) FROM information_schema.COLUMNS WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'rullst_outbox' AND COLUMN_NAME IN ('stream', 'event_key', 'claim_key') AND (COLLATION_NAME IS NULL OR COLLATION_NAME <> 'ascii_bin')";
pub(super) const MYSQL_KEY_COLLATION_UPGRADE: &str = "ALTER TABLE rullst_outbox MODIFY stream VARCHAR(128) CHARACTER SET ascii COLLATE ascii_bin NOT NULL, MODIFY event_key VARCHAR(128) CHARACTER SET ascii COLLATE ascii_bin NOT NULL, MODIFY claim_key VARCHAR(128) CHARACTER SET ascii COLLATE ascii_bin NOT NULL";
pub(super) const SQLITE_TABLE: &str = "CREATE TABLE IF NOT EXISTS rullst_outbox (id INTEGER PRIMARY KEY AUTOINCREMENT, stream TEXT NOT NULL, event_key TEXT NOT NULL, event_kind TEXT NOT NULL, payload_json TEXT NOT NULL, status TEXT NOT NULL, attempts INTEGER NOT NULL, claimed_by TEXT NOT NULL, claim_key TEXT NOT NULL, claim_expires_at_epoch BIGINT NOT NULL, last_error TEXT NOT NULL, available_at_epoch BIGINT NOT NULL, created_at_epoch BIGINT NOT NULL, delivered_at_epoch BIGINT, insert_token TEXT NOT NULL, UNIQUE (stream, event_key))";

pub(super) const POSTGRES_INSERT: &str = "INSERT INTO rullst_outbox (stream, event_key, event_kind, payload_json, status, attempts, claimed_by, claim_expires_at_epoch, last_error, available_at_epoch, created_at_epoch, insert_token, claim_key) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, '') ON CONFLICT (stream, event_key) DO NOTHING";
pub(super) const MYSQL_INSERT: &str = "INSERT INTO rullst_outbox (stream, event_key, event_kind, payload_json, status, attempts, claimed_by, claim_expires_at_epoch, last_error, available_at_epoch, created_at_epoch, insert_token, claim_key) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, '') ON DUPLICATE KEY UPDATE id = LAST_INSERT_ID(id)";
pub(super) const SQLITE_INSERT: &str = "INSERT INTO rullst_outbox (stream, event_key, event_kind, payload_json, status, attempts, claimed_by, claim_expires_at_epoch, last_error, available_at_epoch, created_at_epoch, insert_token, claim_key) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, '') ON CONFLICT (stream, event_key) DO NOTHING";

pub(super) const POSTGRES_ENQUEUED_SELECT: &str = "SELECT id, event_kind, payload_json, insert_token FROM rullst_outbox WHERE stream = $1 AND event_key = $2";
// InnoDB's default REPEATABLE READ serves a plain SELECT from the snapshot of
// the caller's first read, which misses a key committed later by a concurrent
// enqueue. A locking read returns the latest committed row, which the
// preceding upsert has already locked.
pub(super) const MYSQL_ENQUEUED_SELECT: &str = "SELECT id, event_kind, payload_json, insert_token FROM rullst_outbox WHERE stream = ? AND event_key = ? FOR UPDATE";
pub(super) const SQLITE_ENQUEUED_SELECT: &str = "SELECT id, event_kind, payload_json, insert_token FROM rullst_outbox WHERE stream = ? AND event_key = ?";

pub(super) const POSTGRES_EXHAUST: &str = "UPDATE rullst_outbox SET status = $1, claimed_by = $2, claim_key = $3, claim_expires_at_epoch = $4, last_error = $5 WHERE stream = $6 AND attempts >= $7 AND (status = $8 OR (status = $9 AND claim_expires_at_epoch <= $10))";
pub(super) const PORTABLE_EXHAUST: &str = "UPDATE rullst_outbox SET status = ?, claimed_by = ?, claim_key = ?, claim_expires_at_epoch = ?, last_error = ? WHERE stream = ? AND attempts >= ? AND (status = ? OR (status = ? AND claim_expires_at_epoch <= ?))";
pub(super) const POSTGRES_CLAIM_SELECT: &str = "SELECT id FROM rullst_outbox WHERE stream = $1 AND ((status = $2 AND available_at_epoch <= $3) OR (status = $4 AND claim_expires_at_epoch <= $5)) AND attempts < $6 ORDER BY id ASC LIMIT 1 FOR UPDATE SKIP LOCKED";
pub(super) const MYSQL_CLAIM_SELECT: &str = "SELECT id FROM rullst_outbox WHERE stream = ? AND ((status = ? AND available_at_epoch <= ?) OR (status = ? AND claim_expires_at_epoch <= ?)) AND attempts < ? ORDER BY id ASC LIMIT 1 FOR UPDATE SKIP LOCKED";
pub(super) const PORTABLE_CLAIM_SELECT: &str = "SELECT id FROM rullst_outbox WHERE stream = ? AND ((status = ? AND available_at_epoch <= ?) OR (status = ? AND claim_expires_at_epoch <= ?)) AND attempts < ? ORDER BY id ASC LIMIT 1";
pub(super) const POSTGRES_CLAIM_UPDATE: &str = "UPDATE rullst_outbox SET status = $1, attempts = attempts + 1, claimed_by = $2, claim_key = $3, claim_expires_at_epoch = $4, last_error = $5 WHERE id = $6 AND stream = $7 AND ((status = $8 AND available_at_epoch <= $9) OR (status = $10 AND claim_expires_at_epoch <= $11)) AND attempts < $12";
pub(super) const PORTABLE_CLAIM_UPDATE: &str = "UPDATE rullst_outbox SET status = ?, attempts = attempts + 1, claimed_by = ?, claim_key = ?, claim_expires_at_epoch = ?, last_error = ? WHERE id = ? AND stream = ? AND ((status = ? AND available_at_epoch <= ?) OR (status = ? AND claim_expires_at_epoch <= ?)) AND attempts < ?";
pub(super) const POSTGRES_CLAIM_FETCH: &str = "SELECT id, stream, event_key, event_kind, payload_json, attempts, claim_key, claim_expires_at_epoch FROM rullst_outbox WHERE id = $1 AND stream = $2 AND status = $3 AND claim_key = $4";
pub(super) const PORTABLE_CLAIM_FETCH: &str = "SELECT id, stream, event_key, event_kind, payload_json, attempts, claim_key, claim_expires_at_epoch FROM rullst_outbox WHERE id = ? AND stream = ? AND status = ? AND claim_key = ?";
pub(super) const POSTGRES_ACK: &str = "UPDATE rullst_outbox SET status = $1, claimed_by = $2, claim_key = $3, claim_expires_at_epoch = $4, delivered_at_epoch = $5 WHERE id = $6 AND status = $7 AND claim_key = $8 AND claim_expires_at_epoch > $9";
pub(super) const PORTABLE_ACK: &str = "UPDATE rullst_outbox SET status = ?, claimed_by = ?, claim_key = ?, claim_expires_at_epoch = ?, delivered_at_epoch = ? WHERE id = ? AND status = ? AND claim_key = ? AND claim_expires_at_epoch > ?";
pub(super) const POSTGRES_FAIL: &str = "UPDATE rullst_outbox SET status = CASE WHEN attempts >= $1 THEN $2 ELSE $3 END, available_at_epoch = $4, claimed_by = $5, claim_key = $6, claim_expires_at_epoch = $7, last_error = $8 WHERE id = $9 AND status = $10 AND claim_key = $11 AND claim_expires_at_epoch > $12";
pub(super) const PORTABLE_FAIL: &str = "UPDATE rullst_outbox SET status = CASE WHEN attempts >= ? THEN ? ELSE ? END, available_at_epoch = ?, claimed_by = ?, claim_key = ?, claim_expires_at_epoch = ?, last_error = ? WHERE id = ? AND status = ? AND claim_key = ? AND claim_expires_at_epoch > ?";

#[cfg(test)]
mod tests {
    use super::{MYSQL_KEY_COLLATION_UPGRADE, MYSQL_TABLE};

    #[test]
    fn mysql_outbox_keys_compare_case_sensitively() {
        for column in ["stream", "event_key", "claim_key"] {
            let declaration =
                format!("{column} VARCHAR(128) CHARACTER SET ascii COLLATE ascii_bin NOT NULL");
            assert!(MYSQL_TABLE.contains(&declaration), "{column} table DDL");
            assert!(
                MYSQL_KEY_COLLATION_UPGRADE.contains(&format!("MODIFY {declaration}")),
                "{column} upgrade DDL"
            );
        }
    }
}
