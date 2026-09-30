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
async fn conversation_continues_after_retention_removed_every_message() {
    let memory = SqlChatMemory::connect("sqlite::memory:", ChatMemoryConfig::default())
        .await
        .expect("SQLite memory");
    memory.prepare_schema().await.expect("chat schema");
    let tenant = TenantContext::try_new("tenant-retention").expect("tenant");
    let conversation = ConversationId::try_new("chat-expired").expect("conversation");
    memory
        .ensure_conversation(&tenant, &conversation)
        .await
        .expect("conversation");
    memory
        .append_exchange(&tenant, &conversation, 0, "hello", "one")
        .await
        .expect("first exchange");
    sqlx::query("DELETE FROM rullst_ai_chat_messages WHERE created_at_epoch >= 0")
        .execute(memory.pool())
        .await
        .expect("host retention job");

    let history = memory
        .history(&tenant, &conversation)
        .await
        .expect("an expired window is not corruption");
    assert_eq!(history.revision(), 2);
    assert!(history.entries().is_empty());
    assert_eq!(
        memory
            .append_exchange(&tenant, &conversation, 2, "again", "two")
            .await,
        Ok(4)
    );
    let history = memory
        .history(&tenant, &conversation)
        .await
        .expect("history");
    assert_eq!(history.entries().len(), 2);
    assert_eq!(history.entries()[0].sequence(), 3);
}

#[tokio::test]
async fn unsupported_database_urls_fail_before_network_io() {
    assert!(matches!(
        SqlChatMemory::connect("https://database.invalid", ChatMemoryConfig::default()).await,
        Err(ChatMemoryError::InvalidConfiguration(message))
            if message == "SQL chat memory requires a PostgreSQL, MySQL/MariaDB, or SQLite URL"
    ));
}
