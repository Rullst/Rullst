use super::*;
use crate::ai::{AiProvider, ProviderCapabilities};

struct EchoProvider;

#[async_trait]
impl AiProvider for EchoProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::PORTABLE
    }

    async fn prompt(&self, text: &str) -> Result<String, AiError> {
        Ok(text.to_string())
    }

    async fn chat(&self, messages: &[Message]) -> Result<String, AiError> {
        Ok(format!("answer-{}", messages.len()))
    }

    async fn embed(&self, _text: &str) -> Result<Vec<f32>, AiError> {
        Ok(vec![1.0])
    }
}

#[tokio::test]
async fn in_memory_chat_is_tenant_bound_bounded_and_compare_and_swap_ordered() {
    let config = ChatMemoryConfig::try_new(2, 2).expect("memory config");
    let memory = InMemoryChatMemory::new(config);
    let conversation = ConversationId::try_new("support:42").expect("conversation ID");
    let first = TenantContext::try_new("tenant-a").expect("first tenant");
    let second = TenantContext::try_new("tenant-b").expect("second tenant");
    memory
        .ensure_conversation(&first, &conversation)
        .await
        .expect("first conversation");
    memory
        .ensure_conversation(&second, &conversation)
        .await
        .expect("second conversation");

    assert_eq!(
        memory
            .append_exchange(&first, &conversation, 0, "one", "two")
            .await
            .expect("first exchange"),
        2
    );
    assert_eq!(
        memory
            .append_exchange(&first, &conversation, 0, "stale", "stale")
            .await,
        Err(ChatMemoryError::RevisionConflict)
    );
    assert_eq!(
        memory
            .append_exchange(&first, &conversation, 2, "three", "four")
            .await
            .expect("second exchange"),
        4
    );
    let history = memory
        .history(&first, &conversation)
        .await
        .expect("bounded history");
    assert_eq!(history.revision(), 4);
    assert_eq!(history.entries().len(), 2);
    assert_eq!(history.entries()[0].sequence(), 3);
    assert_eq!(
        memory
            .history(&second, &conversation)
            .await
            .expect("isolated history")
            .revision(),
        0
    );
}

#[tokio::test]
async fn stateful_chat_persists_both_halves_after_guarded_generation() {
    let memory = InMemoryChatMemory::new(ChatMemoryConfig::default());
    let service = StatefulChat::new(AiClient::new(EchoProvider), memory);
    let tenant = TenantContext::try_new("tenant-chat").expect("tenant");
    let conversation = ConversationId::try_new("conversation-1").expect("conversation");
    service
        .ensure_conversation(&tenant, &conversation)
        .await
        .expect("create conversation");

    let turn = service
        .send(&tenant, &conversation, "hello")
        .await
        .expect("first turn");
    assert_eq!(turn.response(), "answer-1");
    assert_eq!(turn.revision(), 2);
    let history = service
        .memory()
        .history(&tenant, &conversation)
        .await
        .expect("stored history");
    assert_eq!(history.entries().len(), 2);
    assert_eq!(history.entries()[0].message().content, "hello");
    assert_eq!(history.entries()[1].message().content, "answer-1");
}

#[test]
fn identifiers_and_configuration_fail_closed() {
    for invalid in ["", "../escape", "has spaces", "x/y"] {
        assert_eq!(
            ConversationId::try_new(invalid),
            Err(ChatMemoryError::InvalidConversationId)
        );
    }
    assert!(ChatMemoryConfig::try_new(0, 1).is_err());
    assert!(ChatMemoryConfig::try_new(3, 1).is_err());
    assert!(ChatMemoryConfig::try_new(1, 0).is_err());
}

#[test]
fn histories_fail_closed_on_gaps_partial_exchanges_and_role_inversion() {
    let entry = |sequence, role| {
        ChatMemoryEntry::try_new(sequence, role, "bounded", 1).expect("valid entry")
    };
    assert_eq!(
        ChatHistory::try_new(2, vec![entry(1, "user")]),
        Err(ChatMemoryError::CorruptHistory)
    );
    assert_eq!(
        ChatHistory::try_new(4, vec![entry(1, "user"), entry(4, "assistant")]),
        Err(ChatMemoryError::CorruptHistory)
    );
    assert_eq!(
        ChatHistory::try_new(2, vec![entry(1, "assistant"), entry(2, "user")]),
        Err(ChatMemoryError::CorruptHistory)
    );
    assert_eq!(
        ChatHistory::try_new(2, Vec::new()),
        Err(ChatMemoryError::CorruptHistory)
    );
}
