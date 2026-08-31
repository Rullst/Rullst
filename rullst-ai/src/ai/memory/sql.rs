//! SQLx `AnyPool` conversational memory for SQLite, PostgreSQL, MySQL, and MariaDB.

use super::{
    ChatHistory, ChatMemory, ChatMemoryConfig, ChatMemoryEntry, ChatMemoryError, ConversationId,
    support::{unix_timestamp, validate_content},
};
use async_trait::async_trait;
use rullst_core::security::TenantContext;
use sqlx::{AnyPool, Row, any::AnyPoolOptions};
use std::time::Duration;

/// SQL dialect used by a [`SqlChatMemory`] pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqlChatBackend {
    /// PostgreSQL wire protocol and placeholders.
    Postgres,
    /// MySQL wire protocol, including MariaDB.
    Mysql,
    /// Local or file-backed SQLite.
    Sqlite,
}

/// Reusable durable chat memory over a dedicated SQLx `AnyPool`.
#[derive(Clone)]
pub struct SqlChatMemory {
    pool: AnyPool,
    backend: SqlChatBackend,
    config: ChatMemoryConfig,
}

impl std::fmt::Debug for SqlChatMemory {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SqlChatMemory")
            .field("backend", &self.backend)
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl SqlChatMemory {
    /// Connects to SQLite, PostgreSQL, MySQL, or MariaDB and retains a bounded
    /// recent history. Call [`Self::prepare_schema`] before serving traffic.
    pub async fn connect(
        database_url: impl Into<String>,
        config: ChatMemoryConfig,
    ) -> Result<Self, ChatMemoryError> {
        let database_url = database_url.into();
        let backend = backend_from_url(&database_url)?;
        sqlx::any::install_default_drivers();
        let max_connections =
            if database_url.contains(":memory:") || database_url.contains("mode=memory") {
                1
            } else {
                5
            };
        let pool = AnyPoolOptions::new()
            .max_connections(max_connections)
            .acquire_timeout(Duration::from_secs(10))
            .connect(&database_url)
            .await
            .map_err(|_| ChatMemoryError::StorageUnavailable)?;
        Ok(Self {
            pool,
            backend,
            config,
        })
    }

    /// Wraps an application-created Any pool with an explicit matching dialect.
    pub fn from_pool(pool: AnyPool, backend: SqlChatBackend, config: ChatMemoryConfig) -> Self {
        Self {
            pool,
            backend,
            config,
        }
    }

    /// Returns the selected SQL dialect.
    pub fn backend(&self) -> SqlChatBackend {
        self.backend
    }

    /// Creates the two fixed-name tables and their tenant-bound constraints.
    pub async fn prepare_schema(&self) -> Result<(), ChatMemoryError> {
        let (sessions, messages) = schema_sql(self.backend);
        sqlx::query(sessions)
            .execute(&self.pool)
            .await
            .map_err(|_| ChatMemoryError::StorageUnavailable)?;
        sqlx::query(messages)
            .execute(&self.pool)
            .await
            .map_err(|_| ChatMemoryError::StorageUnavailable)?;
        Ok(())
    }

    /// Returns a reference to the dedicated pool for health checks and shutdown.
    pub fn pool(&self) -> &AnyPool {
        &self.pool
    }
}

#[async_trait]
impl ChatMemory for SqlChatMemory {
    async fn ensure_conversation(
        &self,
        tenant: &TenantContext,
        conversation: &ConversationId,
    ) -> Result<(), ChatMemoryError> {
        let sql = match self.backend {
            SqlChatBackend::Postgres => {
                "INSERT INTO rullst_ai_chat_sessions (tenant_id, conversation_id, conversation_revision, created_at_epoch) VALUES ($1, $2, 0, $3) ON CONFLICT (tenant_id, conversation_id) DO NOTHING"
            }
            SqlChatBackend::Mysql => {
                "INSERT IGNORE INTO rullst_ai_chat_sessions (tenant_id, conversation_id, conversation_revision, created_at_epoch) VALUES (?, ?, 0, ?)"
            }
            SqlChatBackend::Sqlite => {
                "INSERT INTO rullst_ai_chat_sessions (tenant_id, conversation_id, conversation_revision, created_at_epoch) VALUES (?, ?, 0, ?) ON CONFLICT (tenant_id, conversation_id) DO NOTHING"
            }
        };
        sqlx::query(sql)
            .bind(&tenant.tenant_id)
            .bind(conversation.as_str())
            .bind(unix_timestamp())
            .execute(&self.pool)
            .await
            .map_err(|_| ChatMemoryError::StorageUnavailable)?;
        Ok(())
    }

    async fn history(
        &self,
        tenant: &TenantContext,
        conversation: &ConversationId,
    ) -> Result<ChatHistory, ChatMemoryError> {
        let revision_sql = match self.backend {
            SqlChatBackend::Postgres => {
                "SELECT conversation_revision FROM rullst_ai_chat_sessions WHERE tenant_id = $1 AND conversation_id = $2"
            }
            _ => {
                "SELECT conversation_revision FROM rullst_ai_chat_sessions WHERE tenant_id = ? AND conversation_id = ?"
            }
        };
        let revision = sqlx::query_scalar::<_, i64>(revision_sql)
            .bind(&tenant.tenant_id)
            .bind(conversation.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(|_| ChatMemoryError::StorageUnavailable)?
            .ok_or(ChatMemoryError::ConversationNotFound)?;

        let history_sql = match self.backend {
            SqlChatBackend::Postgres => {
                "SELECT turn_sequence, role, content, created_at_epoch FROM rullst_ai_chat_messages WHERE tenant_id = $1 AND conversation_id = $2 AND turn_sequence <= $3 ORDER BY turn_sequence DESC LIMIT $4"
            }
            _ => {
                "SELECT turn_sequence, role, content, created_at_epoch FROM rullst_ai_chat_messages WHERE tenant_id = ? AND conversation_id = ? AND turn_sequence <= ? ORDER BY turn_sequence DESC LIMIT ?"
            }
        };
        let limit = i64::try_from(self.config.history_messages()).map_err(|_| {
            ChatMemoryError::InvalidConfiguration("history limit overflow".to_string())
        })?;
        let rows = sqlx::query(history_sql)
            .bind(&tenant.tenant_id)
            .bind(conversation.as_str())
            .bind(revision)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(|_| ChatMemoryError::StorageUnavailable)?;
        let mut entries = Vec::with_capacity(rows.len());
        for row in rows.into_iter().rev() {
            entries.push(ChatMemoryEntry::try_new(
                row.try_get::<i64, _>("turn_sequence")
                    .map_err(|_| ChatMemoryError::CorruptHistory)?,
                row.try_get::<String, _>("role")
                    .map_err(|_| ChatMemoryError::CorruptHistory)?,
                row.try_get::<String, _>("content")
                    .map_err(|_| ChatMemoryError::CorruptHistory)?,
                row.try_get::<i64, _>("created_at_epoch")
                    .map_err(|_| ChatMemoryError::CorruptHistory)?,
            )?);
        }
        ChatHistory::try_new(revision, entries)
    }

    async fn append_exchange(
        &self,
        tenant: &TenantContext,
        conversation: &ConversationId,
        expected_revision: i64,
        user: &str,
        assistant: &str,
    ) -> Result<i64, ChatMemoryError> {
        validate_content(user)?;
        validate_content(assistant)?;
        if expected_revision < 0 || expected_revision & 1 != 0 {
            return Err(ChatMemoryError::RevisionConflict);
        }
        let revision = expected_revision
            .checked_add(2)
            .ok_or(ChatMemoryError::RevisionConflict)?;
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| ChatMemoryError::StorageUnavailable)?;
        let update_sql = match self.backend {
            SqlChatBackend::Postgres => {
                "UPDATE rullst_ai_chat_sessions SET conversation_revision = $1 WHERE tenant_id = $2 AND conversation_id = $3 AND conversation_revision = $4"
            }
            _ => {
                "UPDATE rullst_ai_chat_sessions SET conversation_revision = ? WHERE tenant_id = ? AND conversation_id = ? AND conversation_revision = ?"
            }
        };
        let updated = sqlx::query(update_sql)
            .bind(revision)
            .bind(&tenant.tenant_id)
            .bind(conversation.as_str())
            .bind(expected_revision)
            .execute(&mut *tx)
            .await
            .map_err(|_| ChatMemoryError::StorageUnavailable)?;
        if updated.rows_affected() != 1 {
            tx.rollback()
                .await
                .map_err(|_| ChatMemoryError::StorageUnavailable)?;
            return Err(ChatMemoryError::RevisionConflict);
        }

        let insert_sql = match self.backend {
            SqlChatBackend::Postgres => {
                "INSERT INTO rullst_ai_chat_messages (tenant_id, conversation_id, turn_sequence, role, content, created_at_epoch) VALUES ($1, $2, $3, $4, $5, $6)"
            }
            _ => {
                "INSERT INTO rullst_ai_chat_messages (tenant_id, conversation_id, turn_sequence, role, content, created_at_epoch) VALUES (?, ?, ?, ?, ?, ?)"
            }
        };
        let created_at = unix_timestamp();
        for (sequence, role, content) in [
            (revision - 1, "user", user),
            (revision, "assistant", assistant),
        ] {
            sqlx::query(insert_sql)
                .bind(&tenant.tenant_id)
                .bind(conversation.as_str())
                .bind(sequence)
                .bind(role)
                .bind(content)
                .bind(created_at)
                .execute(&mut *tx)
                .await
                .map_err(|_| ChatMemoryError::StorageUnavailable)?;
        }
        tx.commit()
            .await
            .map_err(|_| ChatMemoryError::StorageUnavailable)?;
        Ok(revision)
    }

    async fn delete_conversation(
        &self,
        tenant: &TenantContext,
        conversation: &ConversationId,
    ) -> Result<bool, ChatMemoryError> {
        let (messages_sql, session_sql) = match self.backend {
            SqlChatBackend::Postgres => (
                "DELETE FROM rullst_ai_chat_messages WHERE tenant_id = $1 AND conversation_id = $2",
                "DELETE FROM rullst_ai_chat_sessions WHERE tenant_id = $1 AND conversation_id = $2",
            ),
            _ => (
                "DELETE FROM rullst_ai_chat_messages WHERE tenant_id = ? AND conversation_id = ?",
                "DELETE FROM rullst_ai_chat_sessions WHERE tenant_id = ? AND conversation_id = ?",
            ),
        };
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| ChatMemoryError::StorageUnavailable)?;
        sqlx::query(messages_sql)
            .bind(&tenant.tenant_id)
            .bind(conversation.as_str())
            .execute(&mut *tx)
            .await
            .map_err(|_| ChatMemoryError::StorageUnavailable)?;
        let deleted = sqlx::query(session_sql)
            .bind(&tenant.tenant_id)
            .bind(conversation.as_str())
            .execute(&mut *tx)
            .await
            .map_err(|_| ChatMemoryError::StorageUnavailable)?;
        tx.commit()
            .await
            .map_err(|_| ChatMemoryError::StorageUnavailable)?;
        Ok(deleted.rows_affected() == 1)
    }
}

fn backend_from_url(database_url: &str) -> Result<SqlChatBackend, ChatMemoryError> {
    if database_url.starts_with("postgres://") || database_url.starts_with("postgresql://") {
        Ok(SqlChatBackend::Postgres)
    } else if database_url.starts_with("mysql://") {
        Ok(SqlChatBackend::Mysql)
    } else if database_url.starts_with("sqlite:") {
        Ok(SqlChatBackend::Sqlite)
    } else {
        Err(ChatMemoryError::InvalidConfiguration(
            "SQL chat memory requires a PostgreSQL, MySQL/MariaDB, or SQLite URL".to_string(),
        ))
    }
}

fn schema_sql(backend: SqlChatBackend) -> (&'static str, &'static str) {
    match backend {
        SqlChatBackend::Postgres => (
            "CREATE TABLE IF NOT EXISTS rullst_ai_chat_sessions (tenant_id VARCHAR(128) NOT NULL, conversation_id VARCHAR(128) NOT NULL, conversation_revision BIGINT NOT NULL DEFAULT 0, created_at_epoch BIGINT NOT NULL, PRIMARY KEY (tenant_id, conversation_id), CHECK (conversation_revision >= 0 AND MOD(conversation_revision, 2) = 0))",
            "CREATE TABLE IF NOT EXISTS rullst_ai_chat_messages (tenant_id VARCHAR(128) NOT NULL, conversation_id VARCHAR(128) NOT NULL, turn_sequence BIGINT NOT NULL, role VARCHAR(16) NOT NULL CHECK (role IN ('user', 'assistant')), content TEXT NOT NULL, created_at_epoch BIGINT NOT NULL, PRIMARY KEY (tenant_id, conversation_id, turn_sequence), FOREIGN KEY (tenant_id, conversation_id) REFERENCES rullst_ai_chat_sessions (tenant_id, conversation_id) ON DELETE CASCADE)",
        ),
        SqlChatBackend::Mysql => (
            "CREATE TABLE IF NOT EXISTS rullst_ai_chat_sessions (tenant_id VARCHAR(128) NOT NULL, conversation_id VARCHAR(128) NOT NULL, conversation_revision BIGINT NOT NULL DEFAULT 0, created_at_epoch BIGINT NOT NULL, PRIMARY KEY (tenant_id, conversation_id), CHECK (conversation_revision >= 0 AND MOD(conversation_revision, 2) = 0)) ENGINE=InnoDB",
            "CREATE TABLE IF NOT EXISTS rullst_ai_chat_messages (tenant_id VARCHAR(128) NOT NULL, conversation_id VARCHAR(128) NOT NULL, turn_sequence BIGINT NOT NULL, role VARCHAR(16) NOT NULL CHECK (role IN ('user', 'assistant')), content MEDIUMTEXT NOT NULL, created_at_epoch BIGINT NOT NULL, PRIMARY KEY (tenant_id, conversation_id, turn_sequence), FOREIGN KEY (tenant_id, conversation_id) REFERENCES rullst_ai_chat_sessions (tenant_id, conversation_id) ON DELETE CASCADE) ENGINE=InnoDB",
        ),
        SqlChatBackend::Sqlite => (
            "CREATE TABLE IF NOT EXISTS rullst_ai_chat_sessions (tenant_id TEXT NOT NULL, conversation_id TEXT NOT NULL, conversation_revision INTEGER NOT NULL DEFAULT 0 CHECK (conversation_revision >= 0 AND conversation_revision % 2 = 0), created_at_epoch INTEGER NOT NULL, PRIMARY KEY (tenant_id, conversation_id))",
            "CREATE TABLE IF NOT EXISTS rullst_ai_chat_messages (tenant_id TEXT NOT NULL, conversation_id TEXT NOT NULL, turn_sequence INTEGER NOT NULL, role TEXT NOT NULL CHECK (role IN ('user', 'assistant')), content TEXT NOT NULL, created_at_epoch INTEGER NOT NULL, PRIMARY KEY (tenant_id, conversation_id, turn_sequence), FOREIGN KEY (tenant_id, conversation_id) REFERENCES rullst_ai_chat_sessions (tenant_id, conversation_id) ON DELETE CASCADE)",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TM-AI-08: tenant isolation, atomic pairs, stale-writer rejection and erasure.
    #[tokio::test]
    async fn sqlite_history_is_atomic_tenant_bound_and_deletable() {
        let memory = SqlChatMemory::connect("sqlite::memory:", ChatMemoryConfig::default())
            .await
            .expect("SQLite memory");
        memory.prepare_schema().await.expect("chat schema");
        let tenant = TenantContext::try_new("tenant-sql").expect("tenant");
        let other = TenantContext::try_new("tenant-other").expect("other tenant");
        let conversation = ConversationId::try_new("chat-1").expect("conversation");
        memory
            .ensure_conversation(&tenant, &conversation)
            .await
            .expect("tenant conversation");
        memory
            .ensure_conversation(&other, &conversation)
            .await
            .expect("other conversation");

        let (first, second) = tokio::join!(
            memory.append_exchange(&tenant, &conversation, 0, "hello", "one"),
            memory.append_exchange(&tenant, &conversation, 0, "hello", "two")
        );
        assert!(matches!(
            (&first, &second),
            (Ok(2), Err(ChatMemoryError::RevisionConflict))
                | (Err(ChatMemoryError::RevisionConflict), Ok(2))
        ));
        let history = memory
            .history(&tenant, &conversation)
            .await
            .expect("tenant history");
        assert_eq!(history.revision(), 2);
        assert_eq!(history.entries().len(), 2);
        assert_eq!(
            memory
                .history(&other, &conversation)
                .await
                .expect("other history")
                .revision(),
            0
        );
        sqlx::query("PRAGMA foreign_keys = OFF")
            .execute(memory.pool())
            .await
            .expect("disable SQLite foreign keys for explicit-delete proof");
        assert!(
            memory
                .delete_conversation(&tenant, &conversation)
                .await
                .expect("delete conversation")
        );
        assert_eq!(
            memory.history(&tenant, &conversation).await,
            Err(ChatMemoryError::ConversationNotFound)
        );
        let remaining_messages: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM rullst_ai_chat_messages WHERE tenant_id = ? AND conversation_id = ?",
        )
        .bind(&tenant.tenant_id)
        .bind(conversation.as_str())
        .fetch_one(memory.pool())
        .await
        .expect("count orphaned messages");
        assert_eq!(remaining_messages, 0);
    }

    #[tokio::test]
    async fn unsupported_database_urls_fail_before_network_io() {
        assert!(matches!(
            SqlChatMemory::connect("https://database.invalid", ChatMemoryConfig::default()).await,
            Err(ChatMemoryError::InvalidConfiguration(message))
                if message == "SQL chat memory requires a PostgreSQL, MySQL/MariaDB, or SQLite URL"
        ));
    }
}
