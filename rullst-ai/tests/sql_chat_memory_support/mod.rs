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
