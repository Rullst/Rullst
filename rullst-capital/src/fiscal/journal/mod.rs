//! Authenticated, bounded local command evidence for prepared NFS-e requests.

mod api;
mod evidence;
mod format;
mod types;

pub use types::{
    FiscalCommandReceipt, FiscalCommandStatus, FiscalJournalCheckpoint, FiscalJournalDisposition,
    FiscalJournalError, FiscalJournalKey, FiscalJournalSnapshot, FiscalPendingCommand,
    MAX_FISCAL_JOURNAL_BYTES, MAX_FISCAL_JOURNAL_RECORDS,
};

use crate::fiscal::NfseEnvironment;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use format::JournalFileState;

const MIN_FISCAL_JOURNAL_BYTES: u64 = 512;
const MAX_COMMAND_ID_BYTES: usize = 128;
const SCHEMA_VERSION: u8 = 1;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum JournalEnvironment {
    Homologation,
    Production,
}

impl JournalEnvironment {
    fn from_execution(environment: NfseEnvironment) -> Result<Self, FiscalJournalError> {
        match environment {
            NfseEnvironment::Homologation => Ok(Self::Homologation),
            NfseEnvironment::Production => Ok(Self::Production),
            NfseEnvironment::Mock => Err(FiscalJournalError::InvalidEnvironment),
        }
    }

    const fn execution(self) -> NfseEnvironment {
        match self {
            Self::Homologation => NfseEnvironment::Homologation,
            Self::Production => NfseEnvironment::Production,
        }
    }

    const fn from_api(environment: crate::fiscal::NfseApiEnvironment) -> Self {
        match environment {
            crate::fiscal::NfseApiEnvironment::Homologation => Self::Homologation,
            crate::fiscal::NfseApiEnvironment::Production => Self::Production,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct JournalEvent {
    schema_version: u8,
    sequence: u64,
    command_id: String,
    environment: JournalEnvironment,
    request_digest: String,
    observed_at_unix_ms: i64,
    outcome: JournalOutcome,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind", deny_unknown_fields)]
enum JournalOutcome {
    Prepared,
    Authorized {
        result_digest: String,
        processed_at_unix_ms: i64,
    },
    Rejected {
        result_digest: String,
        http_status: u16,
        processed_at_unix_ms: i64,
    },
}

struct CommandState {
    environment: JournalEnvironment,
    request_digest: String,
    prepared_at_unix_ms: i64,
    prepared_sequence: u64,
    status: FiscalCommandStatus,
    result_digest: Option<String>,
    sequence: u64,
}

struct JournalState {
    file: JournalFileState,
    commands: HashMap<String, CommandState>,
}

/// Single-active-writer durable local NFS-e command journal.
pub struct FiscalCommandJournal {
    state: Mutex<JournalState>,
    key: FiscalJournalKey,
    max_bytes: u64,
}

fn build_index(
    events: &[JournalEvent],
) -> Result<HashMap<String, CommandState>, FiscalJournalError> {
    let mut commands = HashMap::new();
    for (offset, event) in events.iter().enumerate() {
        let expected_sequence = (offset as u64).saturating_add(1);
        validate_event(event, expected_sequence)?;
        match &event.outcome {
            JournalOutcome::Prepared => {
                if commands.contains_key(&event.command_id) {
                    return Err(corrupt(offset + 1, "duplicate command preparation"));
                }
                commands.insert(
                    event.command_id.clone(),
                    CommandState {
                        environment: event.environment,
                        request_digest: event.request_digest.clone(),
                        prepared_at_unix_ms: event.observed_at_unix_ms,
                        prepared_sequence: event.sequence,
                        status: FiscalCommandStatus::Prepared,
                        result_digest: None,
                        sequence: event.sequence,
                    },
                );
            }
            JournalOutcome::Authorized { result_digest, .. }
            | JournalOutcome::Rejected { result_digest, .. } => {
                let command = commands
                    .get_mut(&event.command_id)
                    .ok_or_else(|| corrupt(offset + 1, "terminal event has no preparation"))?;
                if command.status != FiscalCommandStatus::Prepared
                    || command.environment != event.environment
                    || command.request_digest != event.request_digest
                    || event.observed_at_unix_ms < command.prepared_at_unix_ms
                {
                    return Err(corrupt(offset + 1, "invalid command state transition"));
                }
                command.status = match event.outcome {
                    JournalOutcome::Authorized { .. } => FiscalCommandStatus::Authorized,
                    _ => FiscalCommandStatus::Rejected,
                };
                command.result_digest = Some(result_digest.clone());
                command.sequence = event.sequence;
            }
        }
    }
    Ok(commands)
}

fn validate_event(event: &JournalEvent, expected_sequence: u64) -> Result<(), FiscalJournalError> {
    if event.schema_version != SCHEMA_VERSION || event.sequence != expected_sequence {
        return Err(corrupt(
            expected_sequence as usize,
            "invalid schema version or sequence",
        ));
    }
    validate_command_id(&event.command_id)
        .map_err(|_| corrupt(expected_sequence as usize, "invalid command ID"))?;
    if !valid_digest(&event.request_digest) || event.observed_at_unix_ms < 0 {
        return Err(corrupt(
            expected_sequence as usize,
            "invalid request digest",
        ));
    }
    match &event.outcome {
        JournalOutcome::Prepared => Ok(()),
        JournalOutcome::Authorized { result_digest, .. } if valid_digest(result_digest) => Ok(()),
        JournalOutcome::Rejected {
            result_digest,
            http_status,
            ..
        } if valid_digest(result_digest) && matches!(http_status, 400 | 403 | 500) => Ok(()),
        _ => Err(corrupt(
            expected_sequence as usize,
            "invalid terminal evidence",
        )),
    }
}

fn receipt(command: &CommandState, disposition: FiscalJournalDisposition) -> FiscalCommandReceipt {
    FiscalCommandReceipt {
        disposition,
        status: command.status,
        sequence: command.sequence,
    }
}

fn next_sequence(records: usize) -> Result<u64, FiscalJournalError> {
    u64::try_from(records)
        .ok()
        .and_then(|value| value.checked_add(1))
        .ok_or(FiscalJournalError::RecordCapacityExceeded)
}

fn unix_now_ms() -> Result<i64, FiscalJournalError> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| FiscalJournalError::ClockUnavailable)?;
    i64::try_from(duration.as_millis()).map_err(|_| FiscalJournalError::ClockUnavailable)
}

fn validate_command_id(value: &str) -> Result<(), FiscalJournalError> {
    if value.is_empty()
        || value.len() > MAX_COMMAND_ID_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        return Err(FiscalJournalError::InvalidCommandId);
    }
    Ok(())
}

fn validate_observed_at(value: i64) -> Result<(), FiscalJournalError> {
    if value < 0 {
        return Err(FiscalJournalError::ClockUnavailable);
    }
    Ok(())
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

const fn corrupt(record: usize, reason: &'static str) -> FiscalJournalError {
    FiscalJournalError::CorruptRecord { record, reason }
}

#[cfg(test)]
mod tests;
