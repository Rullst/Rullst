// src/drivers/memory.rs — In-memory test driver, MailTrap and assertion helper.

use super::{MailDriver, MailError};
use crate::message::Message;
use async_trait::async_trait;
use std::sync::{Arc, Mutex};

static GLOBAL_MAIL_TRAP_STORE: std::sync::OnceLock<Arc<Mutex<Vec<Message>>>> =
    std::sync::OnceLock::new();

fn get_global_mail_trap_store() -> &'static Arc<Mutex<Vec<Message>>> {
    GLOBAL_MAIL_TRAP_STORE.get_or_init(|| Arc::new(Mutex::new(Vec::new())))
}

/// In-memory mock mail driver for fast unit and integration testing without network I/O.
pub struct MemoryDriver {
    store: Arc<Mutex<Vec<Message>>>,
}

impl Default for MemoryDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryDriver {
    /// Creates a new `MemoryDriver` connected to the global `MailTrap` storage.
    pub fn new() -> Self {
        Self {
            store: get_global_mail_trap_store().clone(),
        }
    }

    /// Creates an isolated in-memory driver with its own private message store.
    pub fn isolated() -> (Self, Arc<Mutex<Vec<Message>>>) {
        let store = Arc::new(Mutex::new(Vec::new()));
        (
            Self {
                store: store.clone(),
            },
            store,
        )
    }
}

#[async_trait]
impl MailDriver for MemoryDriver {
    async fn send(&self, message: &Message) -> Result<(), MailError> {
        if let Ok(mut lock) = self.store.lock() {
            lock.push(message.clone());
            Ok(())
        } else {
            Err(MailError::DriverError(
                "Memory driver lock poisoned".to_string(),
            ))
        }
    }
}

/// Fluent assertions and mock inspector for email testing.
pub struct MailTrap;

impl MailTrap {
    /// Clears all captured emails from the global test trap.
    pub fn clear() {
        if let Ok(mut lock) = get_global_mail_trap_store().lock() {
            lock.clear();
        }
    }

    /// Returns a list of all emails captured by the trap.
    pub fn sent_messages() -> Vec<Message> {
        get_global_mail_trap_store()
            .lock()
            .map(|l| l.clone())
            .unwrap_or_default()
    }

    /// Returns the total count of captured emails.
    pub fn count() -> usize {
        get_global_mail_trap_store()
            .lock()
            .map(|l| l.len())
            .unwrap_or(0)
    }

    /// Returns the latest captured email, if any.
    pub fn last_message() -> Option<Message> {
        get_global_mail_trap_store()
            .lock()
            .ok()
            .and_then(|l| l.last().cloned())
    }

    /// Asserts that no emails were sent.
    pub fn assert_nothing_sent() {
        let c = Self::count();
        assert_eq!(
            c, 0,
            "Expected zero emails to be sent, but found {} in MailTrap",
            c
        );
    }

    /// Asserts that an email was sent to the given recipient and returns a fluent assertion builder.
    pub fn assert_sent_to(recipient: &str) -> MailAssertion {
        let messages = Self::sent_messages();
        let found = messages
            .iter()
            .find(|m| m.to.eq_ignore_ascii_case(recipient))
            .cloned();

        assert!(
            found.is_some(),
            "Expected an email sent to '{}', but found messages to: {:?}",
            recipient,
            messages.iter().map(|m| &m.to).collect::<Vec<_>>()
        );

        MailAssertion {
            message: found.unwrap_or_default(),
        }
    }

    /// Creates a driver instance connected to the global test store.
    pub fn driver() -> MemoryDriver {
        MemoryDriver::new()
    }
}

/// Fluent assertion builder for deep inspection of email messages.
pub struct MailAssertion {
    pub message: Message,
}

impl MailAssertion {
    /// Asserts exact subject match.
    pub fn with_subject(self, subject: &str) -> Self {
        assert_eq!(
            self.message.subject, subject,
            "Expected subject '{}', but got '{}'",
            subject, self.message.subject
        );
        self
    }

    /// Asserts that subject contains substring.
    pub fn with_subject_contains(self, fragment: &str) -> Self {
        assert!(
            self.message.subject.contains(fragment),
            "Expected subject '{}' to contain '{}'",
            self.message.subject,
            fragment
        );
        self
    }

    /// Asserts that the HTML or Text body contains the specified substring.
    pub fn with_body_contains(self, fragment: &str) -> Self {
        let body_html = self.message.body_html.as_deref().unwrap_or("");
        let body_text = self.message.body_text.as_deref().unwrap_or("");
        let found = body_html.contains(fragment) || body_text.contains(fragment);
        assert!(
            found,
            "Expected email body to contain '{}'. Text was: '{}', HTML was: '{}'",
            fragment, body_text, body_html
        );
        self
    }

    /// Asserts the from sender address.
    pub fn with_from(self, from: &str) -> Self {
        let actual = self.message.from.as_deref().unwrap_or("");
        assert_eq!(
            actual, from,
            "Expected from sender '{}', but got '{}'",
            from, actual
        );
        self
    }

    /// Asserts that the email contains the specified RFC 8058 One-Click unsubscribe URL.
    pub fn with_unsubscribe_url(self, expected_url: &str) -> Self {
        assert_eq!(
            self.message.unsubscribe_url.as_deref(),
            Some(expected_url),
            "Expected unsubscribe_url '{}', but got '{:?}'",
            expected_url,
            self.message.unsubscribe_url
        );
        self
    }
}
