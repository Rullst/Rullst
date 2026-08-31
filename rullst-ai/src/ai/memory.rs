//! Tenant-aware conversational memory with optimistic cross-process ordering.

use super::{AiClient, AiError, Message};
use async_trait::async_trait;
use rullst_core::security::TenantContext;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[cfg(feature = "sql-memory")]
mod sql;
mod support;
#[cfg(feature = "sql-memory")]
pub use sql::{SqlChatBackend, SqlChatMemory};
use support::{unix_timestamp, validate_content};

const MAX_CONVERSATION_ID_BYTES: usize = 128;
const MAX_MESSAGE_BYTES: usize = 64 * 1024;
const MAX_HISTORY_MESSAGES: usize = 1_024;
const MAX_MEMORY_CONVERSATIONS: usize = 100_000;

/// Validated application conversation identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConversationId(String);

impl ConversationId {
    /// Creates an identifier suitable for SQL keys, logs, and tenant namespaces.
    pub fn try_new(value: impl Into<String>) -> Result<Self, ChatMemoryError> {
        let value = value.into();
        if value.is_empty()
            || value.len() > MAX_CONVERSATION_ID_BYTES
            || !value.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':')
            })
        {
            return Err(ChatMemoryError::InvalidConversationId);
        }
        Ok(Self(value))
    }

    /// Returns the validated identifier.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Bounded history and in-memory capacity policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChatMemoryConfig {
    history_messages: usize,
    memory_conversations: usize,
}

impl ChatMemoryConfig {
    /// Creates a configuration. SQL stores use `history_messages`; the second
    /// bound limits only [`InMemoryChatMemory`] cardinality.
    pub fn try_new(
        history_messages: usize,
        memory_conversations: usize,
    ) -> Result<Self, ChatMemoryError> {
        if !(2..=MAX_HISTORY_MESSAGES).contains(&history_messages)
            || !history_messages.is_multiple_of(2)
        {
            return Err(ChatMemoryError::InvalidConfiguration(format!(
                "history_messages must be an even value between 2 and {MAX_HISTORY_MESSAGES}"
            )));
        }
        if !(1..=MAX_MEMORY_CONVERSATIONS).contains(&memory_conversations) {
            return Err(ChatMemoryError::InvalidConfiguration(format!(
                "memory_conversations must be between 1 and {MAX_MEMORY_CONVERSATIONS}"
            )));
        }
        Ok(Self {
            history_messages,
            memory_conversations,
        })
    }

    /// Maximum recent messages returned to the provider.
    pub fn history_messages(self) -> usize {
        self.history_messages
    }

    /// Maximum conversation keys retained by the process-local store.
    pub fn memory_conversations(self) -> usize {
        self.memory_conversations
    }
}

impl Default for ChatMemoryConfig {
    fn default() -> Self {
        Self {
            history_messages: 64,
            memory_conversations: 10_000,
        }
    }
}

/// One ordered message recovered from a memory implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatMemoryEntry {
    sequence: i64,
    message: Message,
    created_at_epoch: i64,
}

impl ChatMemoryEntry {
    pub(crate) fn try_new(
        sequence: i64,
        role: impl Into<String>,
        content: impl Into<String>,
        created_at_epoch: i64,
    ) -> Result<Self, ChatMemoryError> {
        let role = role.into();
        if !matches!(role.as_str(), "user" | "assistant") {
            return Err(ChatMemoryError::CorruptHistory);
        }
        let content = content.into();
        validate_content(&content)?;
        if sequence <= 0 || created_at_epoch < 0 {
            return Err(ChatMemoryError::CorruptHistory);
        }
        Ok(Self {
            sequence,
            message: Message { role, content },
            created_at_epoch,
        })
    }

    /// Monotonic sequence inside the tenant-bound conversation.
    pub fn sequence(&self) -> i64 {
        self.sequence
    }

    /// Portable provider message.
    pub fn message(&self) -> &Message {
        &self.message
    }

    /// Application-host epoch timestamp recorded with the atomic append.
    pub fn created_at_epoch(&self) -> i64 {
        self.created_at_epoch
    }
}

/// Consistent recent history plus the revision used for compare-and-swap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatHistory {
    revision: i64,
    entries: Vec<ChatMemoryEntry>,
}

impl ChatHistory {
    pub(crate) fn try_new(
        revision: i64,
        entries: Vec<ChatMemoryEntry>,
    ) -> Result<Self, ChatMemoryError> {
        if revision < 0
            || revision & 1 != 0
            || entries.len() > MAX_HISTORY_MESSAGES
            || entries.len() & 1 != 0
            || (revision == 0) != entries.is_empty()
        {
            return Err(ChatMemoryError::CorruptHistory);
        }
        let entry_count =
            i64::try_from(entries.len()).map_err(|_| ChatMemoryError::CorruptHistory)?;
        let first_sequence = revision
            .checked_sub(entry_count)
            .and_then(|value| value.checked_add(1))
            .ok_or(ChatMemoryError::CorruptHistory)?;
        if first_sequence < 1 {
            return Err(ChatMemoryError::CorruptHistory);
        }
        for (offset, entry) in entries.iter().enumerate() {
            let offset = i64::try_from(offset).map_err(|_| ChatMemoryError::CorruptHistory)?;
            let expected_sequence = first_sequence
                .checked_add(offset)
                .ok_or(ChatMemoryError::CorruptHistory)?;
            let expected_role = if expected_sequence & 1 == 1 {
                "user"
            } else {
                "assistant"
            };
            if entry.sequence != expected_sequence || entry.message.role != expected_role {
                return Err(ChatMemoryError::CorruptHistory);
            }
        }
        Ok(Self { revision, entries })
    }

    /// Revision that must be supplied when appending the next exchange.
    pub fn revision(&self) -> i64 {
        self.revision
    }

    /// Recent ordered messages within the configured history budget.
    pub fn entries(&self) -> &[ChatMemoryEntry] {
        &self.entries
    }
}

/// Bounded conversational-memory failure.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum ChatMemoryError {
    /// A local capacity or history setting is invalid.
    #[error("invalid chat-memory configuration: {0}")]
    InvalidConfiguration(String),
    /// Conversation identifiers use a narrow bounded token format.
    #[error("conversation ID must be a 1-128 byte ASCII token")]
    InvalidConversationId,
    /// Empty or oversized message content is rejected before provider/storage use.
    #[error("chat message must contain 1-65536 bytes")]
    InvalidContent,
    /// The selected tenant/conversation pair has not been created.
    #[error("chat conversation was not found")]
    ConversationNotFound,
    /// Another writer advanced the conversation after history was loaded.
    #[error("chat conversation revision conflict")]
    RevisionConflict,
    /// Stored role, sequence, or timestamp data violated the memory contract.
    #[error("chat history failed structural validation")]
    CorruptHistory,
    /// The bounded process-local conversation cardinality was exhausted.
    #[error("in-memory chat conversation capacity reached")]
    CapacityReached,
    /// The durable adapter failed without exposing its SQL or credentials.
    #[error("chat-memory storage operation failed")]
    StorageUnavailable,
}

/// Static-dispatch storage boundary for one tenant-selected conversation.
#[async_trait]
pub trait ChatMemory: Send + Sync {
    /// Idempotently creates the tenant/conversation key at revision zero.
    async fn ensure_conversation(
        &self,
        tenant: &TenantContext,
        conversation: &ConversationId,
    ) -> Result<(), ChatMemoryError>;

    /// Loads a consistent recent history and its compare-and-swap revision.
    async fn history(
        &self,
        tenant: &TenantContext,
        conversation: &ConversationId,
    ) -> Result<ChatHistory, ChatMemoryError>;

    /// Atomically appends one user/assistant exchange only at `expected_revision`.
    async fn append_exchange(
        &self,
        tenant: &TenantContext,
        conversation: &ConversationId,
        expected_revision: i64,
        user: &str,
        assistant: &str,
    ) -> Result<i64, ChatMemoryError>;

    /// Deletes one exact tenant/conversation and its messages.
    async fn delete_conversation(
        &self,
        tenant: &TenantContext,
        conversation: &ConversationId,
    ) -> Result<bool, ChatMemoryError>;
}

#[derive(Default)]
struct MemoryConversation {
    revision: i64,
    entries: Vec<ChatMemoryEntry>,
}

type MemoryMap = HashMap<(String, String), MemoryConversation>;
type MemoryGuard<'a> = std::sync::MutexGuard<'a, MemoryMap>;

/// Deterministic bounded process-local memory for tests and offline development.
#[derive(Clone)]
pub struct InMemoryChatMemory {
    config: ChatMemoryConfig,
    conversations: Arc<Mutex<MemoryMap>>,
}

impl InMemoryChatMemory {
    /// Creates an empty store with validated capacity bounds.
    pub fn new(config: ChatMemoryConfig) -> Self {
        Self {
            config,
            conversations: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn key(tenant: &TenantContext, conversation: &ConversationId) -> (String, String) {
        (tenant.tenant_id.clone(), conversation.as_str().to_string())
    }

    fn lock(&self) -> Result<MemoryGuard<'_>, ChatMemoryError> {
        self.conversations
            .lock()
            .map_err(|_| ChatMemoryError::StorageUnavailable)
    }
}

#[async_trait]
impl ChatMemory for InMemoryChatMemory {
    async fn ensure_conversation(
        &self,
        tenant: &TenantContext,
        conversation: &ConversationId,
    ) -> Result<(), ChatMemoryError> {
        let key = Self::key(tenant, conversation);
        let mut conversations = self.lock()?;
        if !conversations.contains_key(&key)
            && conversations.len() >= self.config.memory_conversations
        {
            return Err(ChatMemoryError::CapacityReached);
        }
        conversations.entry(key).or_default();
        Ok(())
    }

    async fn history(
        &self,
        tenant: &TenantContext,
        conversation: &ConversationId,
    ) -> Result<ChatHistory, ChatMemoryError> {
        let conversations = self.lock()?;
        let state = conversations
            .get(&Self::key(tenant, conversation))
            .ok_or(ChatMemoryError::ConversationNotFound)?;
        ChatHistory::try_new(state.revision, state.entries.clone())
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
        let mut conversations = self.lock()?;
        let state = conversations
            .get_mut(&Self::key(tenant, conversation))
            .ok_or(ChatMemoryError::ConversationNotFound)?;
        if state.revision != expected_revision {
            return Err(ChatMemoryError::RevisionConflict);
        }
        let revision = expected_revision
            .checked_add(2)
            .ok_or(ChatMemoryError::RevisionConflict)?;
        let created_at = unix_timestamp();
        state.entries.push(ChatMemoryEntry::try_new(
            revision - 1,
            "user",
            user,
            created_at,
        )?);
        state.entries.push(ChatMemoryEntry::try_new(
            revision,
            "assistant",
            assistant,
            created_at,
        )?);
        state.revision = revision;
        let excess = state
            .entries
            .len()
            .saturating_sub(self.config.history_messages);
        if excess > 0 {
            state.entries.drain(..excess);
        }
        Ok(revision)
    }

    async fn delete_conversation(
        &self,
        tenant: &TenantContext,
        conversation: &ConversationId,
    ) -> Result<bool, ChatMemoryError> {
        Ok(self
            .lock()?
            .remove(&Self::key(tenant, conversation))
            .is_some())
    }
}

/// Persisted answer and committed conversation revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatTurn {
    response: String,
    revision: i64,
}

impl ChatTurn {
    /// Provider response persisted as the assistant half of the exchange.
    pub fn response(&self) -> &str {
        &self.response
    }

    /// Revision after both messages committed atomically.
    pub fn revision(&self) -> i64 {
        self.revision
    }
}

/// Orchestration failure from input, provider generation, or memory commit.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum StatefulChatError {
    /// A bounded memory operation failed.
    #[error(transparent)]
    Memory(#[from] ChatMemoryError),
    /// Guardrails or the selected provider failed before persistence.
    #[error("chat generation failed: {0}")]
    Generation(#[source] AiError),
}

/// Static-dispatch stateful chat over a caller-selected memory implementation.
pub struct StatefulChat<M> {
    client: AiClient,
    memory: M,
}

impl<M> StatefulChat<M>
where
    M: ChatMemory,
{
    /// Creates a reusable stateful chat service.
    pub fn new(client: AiClient, memory: M) -> Self {
        Self { client, memory }
    }

    /// Idempotently creates a tenant-bound conversation.
    pub async fn ensure_conversation(
        &self,
        tenant: &TenantContext,
        conversation: &ConversationId,
    ) -> Result<(), StatefulChatError> {
        self.memory
            .ensure_conversation(tenant, conversation)
            .await?;
        Ok(())
    }

    /// Loads history, performs guarded generation, and atomically persists the exchange.
    ///
    /// A competing writer causes `RevisionConflict`; Rullst does not automatically
    /// repeat the provider call because that could duplicate cost or side effects.
    pub async fn send(
        &self,
        tenant: &TenantContext,
        conversation: &ConversationId,
        user: impl Into<String>,
    ) -> Result<ChatTurn, StatefulChatError> {
        let user = user.into();
        validate_content(&user)?;
        let history = self.memory.history(tenant, conversation).await?;
        let mut builder = self.client.chat();
        for entry in history.entries() {
            builder = match entry.message().role.as_str() {
                "user" => builder.user(entry.message().content.clone()),
                "assistant" => builder.assistant(entry.message().content.clone()),
                _ => return Err(ChatMemoryError::CorruptHistory.into()),
            };
        }
        let response = builder
            .user(user.clone())
            .send()
            .await
            .map_err(StatefulChatError::Generation)?;
        validate_content(&response)?;
        let revision = self
            .memory
            .append_exchange(tenant, conversation, history.revision(), &user, &response)
            .await?;
        Ok(ChatTurn { response, revision })
    }

    /// Returns the configured memory implementation for explicit retention operations.
    pub fn memory(&self) -> &M {
        &self.memory
    }
}

#[cfg(test)]
mod tests;
