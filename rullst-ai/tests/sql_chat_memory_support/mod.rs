#![cfg(feature = "sql-memory")]

use rullst_ai::{
    ChatMemory, ChatMemoryConfig, ChatMemoryError, ConversationId, SqlChatBackend, SqlChatMemory,
};
use rullst_core::security::TenantContext;

pub fn handle_container_start_error(provider: &str, error: impl std::fmt::Display) {
    if std::env::var("RULLST_REQUIRE_TESTCONTAINERS").as_deref() == Ok("true") {
        panic!("{provider} chat-memory testcontainer is required but failed to start: {error}");
    }
    eprintln!("skipping {provider} chat-memory matrix: {error}");
}

pub async fn exercise_sql_chat_memory(database_url: &str, backend: SqlChatBackend) {
    let config = ChatMemoryConfig::try_new(8, 1).expect("chat-memory config");
    let first = SqlChatMemory::connect(database_url, config)
        .await
        .expect("first SQL chat-memory connection");
    let second = SqlChatMemory::connect(database_url, config)
        .await
        .expect("second SQL chat-memory connection");
    assert_eq!(first.backend(), backend);
    first.prepare_schema().await.expect("chat-memory schema");

    let tenant = TenantContext::try_new("matrix-tenant").expect("matrix tenant");
    let other = TenantContext::try_new("matrix-other").expect("other tenant");
    let conversation = ConversationId::try_new("conversation-1").expect("conversation ID");
    first
        .ensure_conversation(&tenant, &conversation)
        .await
        .expect("first conversation");
    first
        .ensure_conversation(&other, &conversation)
        .await
        .expect("same ID in another tenant");

    let (left, right) = tokio::join!(
        first.append_exchange(&tenant, &conversation, 0, "question", "left"),
        second.append_exchange(&tenant, &conversation, 0, "question", "right")
    );
    assert!(matches!(
        (&left, &right),
        (Ok(2), Err(ChatMemoryError::RevisionConflict))
            | (Err(ChatMemoryError::RevisionConflict), Ok(2))
    ));
    let history = first
        .history(&tenant, &conversation)
        .await
        .expect("committed history");
    assert_eq!(history.revision(), 2);
    assert_eq!(history.entries().len(), 2);
    assert_eq!(history.entries()[0].message().content, "question");
    assert_eq!(
        first
            .history(&other, &conversation)
            .await
            .expect("tenant-isolated history")
            .revision(),
        0
    );

    assert!(
        second
            .delete_conversation(&tenant, &conversation)
            .await
            .expect("delete conversation")
    );
    assert_eq!(
        first.history(&tenant, &conversation).await,
        Err(ChatMemoryError::ConversationNotFound)
    );
}

/// Chat-memory tables as created by releases before the MySQL/MariaDB key
/// columns used `ascii_bin`; they take the server's case-insensitive default.
const LEGACY_MYSQL_TABLES: [&str; 2] = [
    "CREATE TABLE rullst_ai_chat_sessions (tenant_id VARCHAR(128) NOT NULL, conversation_id VARCHAR(128) NOT NULL, conversation_revision BIGINT NOT NULL DEFAULT 0, created_at_epoch BIGINT NOT NULL, PRIMARY KEY (tenant_id, conversation_id), CHECK (conversation_revision >= 0 AND MOD(conversation_revision, 2) = 0)) ENGINE=InnoDB",
    "CREATE TABLE rullst_ai_chat_messages (tenant_id VARCHAR(128) NOT NULL, conversation_id VARCHAR(128) NOT NULL, turn_sequence BIGINT NOT NULL, role VARCHAR(16) NOT NULL CHECK (role IN ('user', 'assistant')), content MEDIUMTEXT NOT NULL, created_at_epoch BIGINT NOT NULL, PRIMARY KEY (tenant_id, conversation_id, turn_sequence), FOREIGN KEY (tenant_id, conversation_id) REFERENCES rullst_ai_chat_sessions (tenant_id, conversation_id) ON DELETE CASCADE) ENGINE=InnoDB",
];

/// The operator migration documented in the rullst-ai README; keep in sync.
/// MariaDB refuses to change a column used by a foreign key even with
/// `FOREIGN_KEY_CHECKS = 0`, so the constraint is dropped and recreated.
const DOCUMENTED_MYSQL_MIGRATION: [&str; 3] = [
    "ALTER TABLE rullst_ai_chat_messages DROP FOREIGN KEY rullst_ai_chat_messages_ibfk_1",
    "ALTER TABLE rullst_ai_chat_sessions MODIFY tenant_id VARCHAR(128) CHARACTER SET ascii COLLATE ascii_bin NOT NULL, MODIFY conversation_id VARCHAR(128) CHARACTER SET ascii COLLATE ascii_bin NOT NULL",
    "ALTER TABLE rullst_ai_chat_messages MODIFY tenant_id VARCHAR(128) CHARACTER SET ascii COLLATE ascii_bin NOT NULL, MODIFY conversation_id VARCHAR(128) CHARACTER SET ascii COLLATE ascii_bin NOT NULL, ADD FOREIGN KEY (tenant_id, conversation_id) REFERENCES rullst_ai_chat_sessions (tenant_id, conversation_id) ON DELETE CASCADE",
];

async fn drop_chat_tables(memory: &SqlChatMemory) {
    for statement in [
        "DROP TABLE IF EXISTS rullst_ai_chat_messages",
        "DROP TABLE IF EXISTS rullst_ai_chat_sessions",
    ] {
        sqlx::query(statement)
            .execute(memory.pool())
            .await
            .expect("drop chat-memory table");
    }
}

async fn first_content(
    memory: &SqlChatMemory,
    tenant: &TenantContext,
    id: &ConversationId,
) -> String {
    let history = memory.history(tenant, id).await.expect("history");
    history
        .entries()
        .first()
        .map(|entry| entry.message().content.clone())
        .unwrap_or_default()
}

/// TM-AI-08: tenant and conversation IDs that differ only by case are distinct
/// on MySQL/MariaDB. A legacy case-insensitive table fails closed instead of
/// sharing a conversation, until the documented migration is applied.
#[allow(dead_code)] // Used by the MySQL and MariaDB matrix binaries only.
pub async fn exercise_case_sensitive_mysql_keys(database_url: &str) {
    let config = ChatMemoryConfig::try_new(8, 1).expect("chat-memory config");
    let memory = SqlChatMemory::connect(database_url, config)
        .await
        .expect("SQL chat-memory connection");
    let lower = TenantContext::try_new("acme").expect("lower-case tenant");
    let upper = TenantContext::try_new("ACME").expect("upper-case tenant");
    let conversation = ConversationId::try_new("support:case-7").expect("conversation ID");
    let other_case = ConversationId::try_new("SUPPORT:case-7").expect("case-variant ID");

    drop_chat_tables(&memory).await;
    memory.prepare_schema().await.expect("chat-memory schema");
    memory
        .ensure_conversation(&lower, &conversation)
        .await
        .expect("lower-case conversation");
    assert_eq!(
        memory
            .append_exchange(&lower, &conversation, 0, "lower question", "lower answer")
            .await,
        Ok(2)
    );
    memory
        .ensure_conversation(&upper, &conversation)
        .await
        .expect("case-variant tenant is a distinct key");
    let upper_history = memory
        .history(&upper, &conversation)
        .await
        .expect("history");
    assert_eq!(upper_history.revision(), 0);
    assert!(upper_history.entries().is_empty());
    memory
        .ensure_conversation(&lower, &other_case)
        .await
        .expect("case-variant conversation is a distinct key");
    assert_eq!(
        memory
            .history(&lower, &other_case)
            .await
            .expect("case-variant conversation history")
            .revision(),
        0
    );
    assert_eq!(
        memory
            .append_exchange(&upper, &conversation, 0, "upper question", "upper answer")
            .await,
        Ok(2)
    );
    assert_eq!(
        first_content(&memory, &lower, &conversation).await,
        "lower question"
    );
    assert_eq!(
        first_content(&memory, &upper, &conversation).await,
        "upper question"
    );
    assert_eq!(
        memory.delete_conversation(&upper, &conversation).await,
        Ok(true)
    );
    assert_eq!(
        first_content(&memory, &lower, &conversation).await,
        "lower question"
    );

    drop_chat_tables(&memory).await;
    for statement in LEGACY_MYSQL_TABLES {
        sqlx::query(statement)
            .execute(memory.pool())
            .await
            .expect("legacy chat-memory table");
    }
    memory
        .prepare_schema()
        .await
        .expect("existing tables are left unaltered");
    memory
        .ensure_conversation(&lower, &conversation)
        .await
        .expect("legacy lower-case conversation");
    assert_eq!(
        memory
            .append_exchange(&lower, &conversation, 0, "lower question", "lower answer")
            .await,
        Ok(2)
    );
    assert!(matches!(
        memory.ensure_conversation(&upper, &conversation).await,
        Err(ChatMemoryError::InvalidConfiguration(_))
    ));
    assert_eq!(
        memory.history(&upper, &conversation).await,
        Err(ChatMemoryError::ConversationNotFound)
    );
    assert_eq!(
        memory
            .append_exchange(&upper, &conversation, 2, "upper question", "upper answer")
            .await,
        Err(ChatMemoryError::RevisionConflict)
    );
    assert_eq!(
        memory.delete_conversation(&upper, &conversation).await,
        Ok(false)
    );
    let lower_history = memory
        .history(&lower, &conversation)
        .await
        .expect("history");
    assert_eq!(lower_history.revision(), 2);
    assert_eq!(lower_history.entries().len(), 2);

    // The documented lookup of the generated constraint name.
    let constraint: String = sqlx::query_scalar(
        "SELECT CONSTRAINT_NAME FROM information_schema.REFERENTIAL_CONSTRAINTS \
         WHERE CONSTRAINT_SCHEMA = DATABASE() AND TABLE_NAME = 'rullst_ai_chat_messages'",
    )
    .fetch_one(memory.pool())
    .await
    .expect("chat-message foreign key name");
    assert_eq!(constraint, "rullst_ai_chat_messages_ibfk_1");
    for statement in DOCUMENTED_MYSQL_MIGRATION {
        sqlx::query(statement)
            .execute(memory.pool())
            .await
            .expect("documented migration statement");
    }
    memory
        .ensure_conversation(&upper, &conversation)
        .await
        .expect("case-variant tenant after migration");
    assert_eq!(
        memory
            .history(&upper, &conversation)
            .await
            .expect("migrated history")
            .revision(),
        0
    );
    assert_eq!(
        first_content(&memory, &lower, &conversation).await,
        "lower question"
    );
}
