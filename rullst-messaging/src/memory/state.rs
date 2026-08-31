use crate::{
    ConsumerGroup, ConsumerName, FailureCode, MessageEnvelope, PublishReceipt, StartPosition,
};
use std::collections::{BTreeMap, HashMap};

#[derive(Default)]
pub(super) struct State {
    pub(super) topics: HashMap<crate::TopicName, TopicState>,
    pub(super) leases: HashMap<String, LeasePointer>,
    pub(super) retained_messages: usize,
    pub(super) subscriptions: usize,
}

pub(super) struct TopicState {
    pub(super) next_sequence: u64,
    pub(super) messages: BTreeMap<u64, StoredMessage>,
    pub(super) idempotency: HashMap<String, IdempotencyRecord>,
    pub(super) subscriptions: HashMap<ConsumerGroup, SubscriptionState>,
}

impl Default for TopicState {
    fn default() -> Self {
        Self {
            next_sequence: 1,
            messages: BTreeMap::new(),
            idempotency: HashMap::new(),
            subscriptions: HashMap::new(),
        }
    }
}

pub(super) struct StoredMessage {
    pub(super) envelope: MessageEnvelope,
    pub(super) idempotency_key: String,
}

pub(super) struct IdempotencyRecord {
    pub(super) fingerprint: [u8; 32],
    pub(super) receipt: PublishReceipt,
}

pub(super) struct SubscriptionState {
    pub(super) start_sequence: u64,
    pub(super) pending: BTreeMap<u64, i64>,
    pub(super) in_flight: HashMap<u64, LeaseState>,
    pub(super) attempts: HashMap<u64, u32>,
    pub(super) terminal: BTreeMap<u64, TerminalState>,
}

impl SubscriptionState {
    pub(super) fn new(
        position: StartPosition,
        next_sequence: u64,
        messages: &BTreeMap<u64, StoredMessage>,
    ) -> Self {
        let start_sequence = match position {
            StartPosition::Earliest => messages.keys().next().copied().unwrap_or(next_sequence),
            StartPosition::Latest => next_sequence,
        };
        let pending = match position {
            StartPosition::Earliest => messages
                .iter()
                .map(|(sequence, message)| (*sequence, message.envelope.published_at_ms()))
                .collect(),
            StartPosition::Latest => BTreeMap::new(),
        };
        Self {
            start_sequence,
            pending,
            in_flight: HashMap::new(),
            attempts: HashMap::new(),
            terminal: BTreeMap::new(),
        }
    }
}

pub(super) struct LeaseState {
    pub(super) token: String,
    pub(super) consumer: ConsumerName,
    pub(super) expires_at_ms: i64,
    pub(super) attempt: u32,
}

#[derive(Clone)]
pub(super) struct LeasePointer {
    pub(super) topic: crate::TopicName,
    pub(super) group: ConsumerGroup,
    pub(super) sequence: u64,
}

pub(super) enum TerminalState {
    Acked,
    Dead {
        attempts: u32,
        failure_code: FailureCode,
        dead_lettered_at_ms: i64,
    },
}
