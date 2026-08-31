use super::state::{LeasePointer, LeaseState, State, SubscriptionState, TerminalState};
use crate::validation::MAX_RETRY_MILLIS;
use crate::{AckToken, FailureCode, MessagingError, Result};
use std::time::Duration;

pub(super) fn subscription_mut<'a>(
    state: &'a mut State,
    pointer: &LeasePointer,
) -> Result<&'a mut SubscriptionState> {
    state
        .topics
        .get_mut(&pointer.topic)
        .and_then(|topic| topic.subscriptions.get_mut(&pointer.group))
        .ok_or(MessagingError::InternalState {
            context: "lease subscription lookup",
        })
}

pub(super) fn take_valid_lease(
    state: &mut State,
    token: &AckToken,
    now: i64,
    max_attempts: u32,
) -> Result<(LeasePointer, LeaseState)> {
    let pointer = state
        .leases
        .remove(token.as_str())
        .ok_or(MessagingError::LeaseNotFound)?;
    let subscription = subscription_mut(state, &pointer)?;
    let lease =
        subscription
            .in_flight
            .remove(&pointer.sequence)
            .ok_or(MessagingError::InternalState {
                context: "lease delivery lookup",
            })?;
    if lease.token != token.as_str() {
        return Err(MessagingError::InternalState {
            context: "lease token binding",
        });
    }
    if lease.expires_at_ms <= now {
        expire_or_requeue(
            subscription,
            pointer.sequence,
            lease.attempt,
            now,
            max_attempts,
        );
        return Err(MessagingError::LeaseExpired);
    }
    Ok((pointer, lease))
}

pub(super) fn expire_or_requeue(
    subscription: &mut SubscriptionState,
    sequence: u64,
    attempt: u32,
    now: i64,
    max_attempts: u32,
) {
    if attempt >= max_attempts {
        subscription.terminal.insert(
            sequence,
            TerminalState::Dead {
                attempts: attempt,
                failure_code: FailureCode::max_attempts(),
                dead_lettered_at_ms: now,
            },
        );
    } else {
        subscription.pending.insert(sequence, now);
    }
}

pub(super) fn validate_retry_delay(delay: Duration) -> Result<u64> {
    let delay_millis = u64::try_from(delay.as_millis()).map_err(|_| MessagingError::Invalid {
        field: "retry delay",
        reason: "duration is outside the supported range",
    })?;
    if delay_millis > MAX_RETRY_MILLIS {
        return Err(MessagingError::Invalid {
            field: "retry delay",
            reason: "must not exceed seven days",
        });
    }
    Ok(delay_millis)
}

pub(super) fn add_millis(timestamp: i64, millis: u64) -> Result<i64> {
    let millis = i64::try_from(millis).map_err(|_| MessagingError::ClockOutOfRange)?;
    timestamp
        .checked_add(millis)
        .ok_or(MessagingError::ClockOutOfRange)
}
