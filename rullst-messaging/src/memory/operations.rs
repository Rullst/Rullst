use super::InMemoryBroker;
use super::helpers::{
    add_millis, expire_or_requeue, subscription_mut, take_valid_lease, validate_retry_delay,
};
use super::state::{
    IdempotencyRecord, LeasePointer, LeaseState, StoredMessage, SubscriptionState, TerminalState,
    TopicState,
};
use crate::{
    AckToken, Clock, DeadLetter, DeadLetterQuery, Delivery, FailureCode, MessageId, MessagingError,
    PublishReceipt, PublishRequest, PurgeReceipt, PurgeRequest, ReceiveRequest, Result,
    RetryDisposition, SubscriptionReceipt, SubscriptionRequest,
};
use std::time::Duration;

impl<C: Clock> InMemoryBroker<C> {
    pub(super) async fn publish_inner(&self, request: PublishRequest) -> Result<PublishReceipt> {
        request.validate_payload(self.config.max_payload_bytes())?;
        let fingerprint = request.fingerprint()?;
        let now = self.now()?;
        let mut state = self.state.lock().await;

        if let Some(record) = state
            .topics
            .get(request.topic())
            .and_then(|topic| topic.idempotency.get(request.idempotency_key().as_str()))
        {
            if record.fingerprint == fingerprint {
                tracing::debug!(
                    namespace = %self.config.namespace(),
                    topic = %request.topic(),
                    duplicate = true,
                    "messaging publication replayed"
                );
                return Ok(record.receipt.as_duplicate());
            }
            return Err(MessagingError::IdempotencyConflict);
        }

        if state.retained_messages >= self.config.max_retained_messages() {
            return Err(MessagingError::CapacityExceeded {
                resource: "retained messages",
                limit: self.config.max_retained_messages(),
            });
        }

        let id = MessageId::random();
        let envelope = crate::MessageEnvelope::from_request(
            &request,
            self.config.namespace().clone(),
            id.clone(),
            now,
        );
        let receipt = PublishReceipt::new(id, false, now);
        let topic = state
            .topics
            .entry(request.topic().clone())
            .or_insert_with(TopicState::default);
        let sequence = topic.next_sequence;
        topic.next_sequence =
            topic
                .next_sequence
                .checked_add(1)
                .ok_or(MessagingError::CapacityExceeded {
                    resource: "topic sequence",
                    limit: usize::MAX,
                })?;
        topic.messages.insert(
            sequence,
            StoredMessage {
                envelope,
                idempotency_key: request.idempotency_key().as_str().to_string(),
            },
        );
        topic.idempotency.insert(
            request.idempotency_key().as_str().to_string(),
            IdempotencyRecord {
                fingerprint,
                receipt: receipt.clone(),
            },
        );
        for subscription in topic.subscriptions.values_mut() {
            subscription.pending.insert(sequence, now);
        }
        state.retained_messages =
            state
                .retained_messages
                .checked_add(1)
                .ok_or(MessagingError::InternalState {
                    context: "retained message increment",
                })?;

        tracing::debug!(
            namespace = %self.config.namespace(),
            topic = %request.topic(),
            duplicate = false,
            payload_bytes = request.payload().len(),
            "messaging publication accepted"
        );
        Ok(receipt)
    }

    pub(super) async fn subscribe_inner(
        &self,
        request: SubscriptionRequest,
    ) -> Result<SubscriptionReceipt> {
        let mut state = self.state.lock().await;
        if let Some(existing) = state
            .topics
            .get(request.topic())
            .and_then(|topic| topic.subscriptions.get(request.group()))
        {
            return Ok(SubscriptionReceipt::new(false, existing.pending.len()));
        }
        if state.subscriptions >= self.config.max_subscriptions() {
            return Err(MessagingError::CapacityExceeded {
                resource: "message subscriptions",
                limit: self.config.max_subscriptions(),
            });
        }
        let topic = state
            .topics
            .entry(request.topic().clone())
            .or_insert_with(TopicState::default);
        let subscription =
            SubscriptionState::new(request.start(), topic.next_sequence, &topic.messages);
        let pending = subscription.pending.len();
        topic
            .subscriptions
            .insert(request.group().clone(), subscription);
        state.subscriptions =
            state
                .subscriptions
                .checked_add(1)
                .ok_or(MessagingError::InternalState {
                    context: "subscription increment",
                })?;
        tracing::debug!(
            namespace = %self.config.namespace(),
            topic = %request.topic(),
            group = %request.group(),
            pending,
            "messaging subscription registered"
        );
        Ok(SubscriptionReceipt::new(true, pending))
    }

    pub(super) async fn receive_inner(&self, request: ReceiveRequest) -> Result<Vec<Delivery>> {
        let now = self.now()?;
        let lease_expires_at_ms = add_millis(now, request.lease_millis())?;
        let mut state = self.state.lock().await;
        let mut expired_tokens = Vec::new();
        let mut new_leases = Vec::new();

        let deliveries = {
            let topic = state
                .topics
                .get_mut(request.topic())
                .ok_or(MessagingError::SubscriptionNotFound)?;
            let messages = &topic.messages;
            let subscription = topic
                .subscriptions
                .get_mut(request.group())
                .ok_or(MessagingError::SubscriptionNotFound)?;

            let expired_sequences: Vec<u64> = subscription
                .in_flight
                .iter()
                .filter_map(|(sequence, lease)| (lease.expires_at_ms <= now).then_some(*sequence))
                .collect();
            for sequence in expired_sequences {
                let Some(lease) = subscription.in_flight.remove(&sequence) else {
                    return Err(MessagingError::InternalState {
                        context: "expired lease removal",
                    });
                };
                expired_tokens.push(lease.token.clone());
                expire_or_requeue(
                    subscription,
                    sequence,
                    lease.attempt,
                    now,
                    self.config.max_attempts(),
                );
            }

            let candidates: Vec<u64> = subscription
                .pending
                .iter()
                .filter_map(|(sequence, available_at)| (*available_at <= now).then_some(*sequence))
                .take(request.max_messages())
                .collect();
            let mut deliveries = Vec::with_capacity(candidates.len());
            for sequence in candidates {
                let prior_attempts = subscription.attempts.get(&sequence).copied().unwrap_or(0);
                if prior_attempts >= self.config.max_attempts() {
                    subscription.pending.remove(&sequence);
                    subscription.terminal.insert(
                        sequence,
                        TerminalState::Dead {
                            attempts: prior_attempts,
                            failure_code: FailureCode::max_attempts(),
                            dead_lettered_at_ms: now,
                        },
                    );
                    continue;
                }
                let attempt =
                    prior_attempts
                        .checked_add(1)
                        .ok_or(MessagingError::InternalState {
                            context: "delivery attempt increment",
                        })?;
                let stored = messages
                    .get(&sequence)
                    .ok_or(MessagingError::InternalState {
                        context: "pending message lookup",
                    })?;
                let token = AckToken::random();
                subscription.pending.remove(&sequence);
                subscription.attempts.insert(sequence, attempt);
                subscription.in_flight.insert(
                    sequence,
                    LeaseState {
                        token: token.as_str().to_string(),
                        consumer: request.consumer().clone(),
                        expires_at_ms: lease_expires_at_ms,
                        attempt,
                    },
                );
                new_leases.push((
                    token.as_str().to_string(),
                    LeasePointer {
                        topic: request.topic().clone(),
                        group: request.group().clone(),
                        sequence,
                    },
                ));
                deliveries.push(Delivery::new(
                    stored.envelope.clone(),
                    request.group().clone(),
                    request.consumer().clone(),
                    attempt,
                    lease_expires_at_ms,
                    token,
                ));
            }
            deliveries
        };

        for token in expired_tokens {
            state.leases.remove(&token);
        }
        for (token, pointer) in new_leases {
            state.leases.insert(token, pointer);
        }
        tracing::debug!(
            namespace = %self.config.namespace(),
            topic = %request.topic(),
            group = %request.group(),
            consumer = %request.consumer(),
            count = deliveries.len(),
            "messaging receive completed"
        );
        Ok(deliveries)
    }

    pub(super) async fn ack_inner(&self, token: &AckToken) -> Result<()> {
        let now = self.now()?;
        let mut state = self.state.lock().await;
        let (pointer, lease) =
            take_valid_lease(&mut state, token, now, self.config.max_attempts())?;
        let subscription = subscription_mut(&mut state, &pointer)?;
        subscription
            .terminal
            .insert(pointer.sequence, TerminalState::Acked);
        tracing::debug!(
            namespace = %self.config.namespace(),
            topic = %pointer.topic,
            group = %pointer.group,
            consumer = %lease.consumer,
            attempt = lease.attempt,
            "messaging delivery acknowledged"
        );
        Ok(())
    }

    pub(super) async fn retry_inner(
        &self,
        token: &AckToken,
        delay: Duration,
        failure_code: FailureCode,
    ) -> Result<RetryDisposition> {
        let delay_millis = validate_retry_delay(delay)?;
        let now = self.now()?;
        let available_at_ms = add_millis(now, delay_millis)?;
        let mut state = self.state.lock().await;
        let (pointer, lease) =
            take_valid_lease(&mut state, token, now, self.config.max_attempts())?;
        let subscription = subscription_mut(&mut state, &pointer)?;
        if lease.attempt >= self.config.max_attempts() {
            subscription.terminal.insert(
                pointer.sequence,
                TerminalState::Dead {
                    attempts: lease.attempt,
                    failure_code,
                    dead_lettered_at_ms: now,
                },
            );
            return Ok(RetryDisposition::DeadLettered);
        }
        subscription
            .pending
            .insert(pointer.sequence, available_at_ms);
        Ok(RetryDisposition::Scheduled { available_at_ms })
    }

    pub(super) async fn dead_letter_inner(
        &self,
        token: &AckToken,
        failure_code: FailureCode,
    ) -> Result<()> {
        let now = self.now()?;
        let mut state = self.state.lock().await;
        let (pointer, lease) =
            take_valid_lease(&mut state, token, now, self.config.max_attempts())?;
        let subscription = subscription_mut(&mut state, &pointer)?;
        subscription.terminal.insert(
            pointer.sequence,
            TerminalState::Dead {
                attempts: lease.attempt,
                failure_code,
                dead_lettered_at_ms: now,
            },
        );
        Ok(())
    }

    pub(super) async fn dead_letters_inner(
        &self,
        query: DeadLetterQuery,
    ) -> Result<Vec<DeadLetter>> {
        let state = self.state.lock().await;
        let topic = state
            .topics
            .get(query.topic())
            .ok_or(MessagingError::SubscriptionNotFound)?;
        let subscription = topic
            .subscriptions
            .get(query.group())
            .ok_or(MessagingError::SubscriptionNotFound)?;
        let mut dead_letters = Vec::new();
        for (sequence, terminal) in &subscription.terminal {
            let TerminalState::Dead {
                attempts,
                failure_code,
                dead_lettered_at_ms,
            } = terminal
            else {
                continue;
            };
            let message = topic
                .messages
                .get(sequence)
                .ok_or(MessagingError::InternalState {
                    context: "dead-letter message lookup",
                })?;
            dead_letters.push(DeadLetter::new(
                message.envelope.clone(),
                query.group().clone(),
                *attempts,
                failure_code.clone(),
                *dead_lettered_at_ms,
            ));
            if dead_letters.len() == query.limit() {
                break;
            }
        }
        Ok(dead_letters)
    }

    pub(super) async fn purge_terminal_inner(&self, request: PurgeRequest) -> Result<PurgeReceipt> {
        let mut state = self.state.lock().await;
        let removed = {
            let Some(topic) = state.topics.get_mut(request.topic()) else {
                return Ok(PurgeReceipt::new(0));
            };
            if topic.subscriptions.is_empty() {
                return Ok(PurgeReceipt::new(0));
            }
            let removable: Vec<u64> = topic
                .messages
                .keys()
                .filter(|sequence| {
                    topic.subscriptions.values().all(|subscription| {
                        **sequence < subscription.start_sequence
                            || subscription.terminal.contains_key(sequence)
                    })
                })
                .take(request.limit())
                .copied()
                .collect();
            for sequence in &removable {
                let message =
                    topic
                        .messages
                        .remove(sequence)
                        .ok_or(MessagingError::InternalState {
                            context: "terminal message purge",
                        })?;
                topic.idempotency.remove(&message.idempotency_key);
                for subscription in topic.subscriptions.values_mut() {
                    subscription.pending.remove(sequence);
                    subscription.in_flight.remove(sequence);
                    subscription.attempts.remove(sequence);
                    subscription.terminal.remove(sequence);
                }
            }
            removable.len()
        };
        state.retained_messages =
            state
                .retained_messages
                .checked_sub(removed)
                .ok_or(MessagingError::InternalState {
                    context: "retained message decrement",
                })?;
        Ok(PurgeReceipt::new(removed))
    }

    fn now(&self) -> Result<i64> {
        let now = self.clock.now_millis()?;
        if now < 0 {
            return Err(MessagingError::ClockOutOfRange);
        }
        Ok(now)
    }
}
