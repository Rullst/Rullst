use super::{
    CommandState, FiscalCommandJournal, FiscalCommandReceipt, FiscalCommandStatus,
    FiscalJournalCheckpoint, FiscalJournalDisposition, FiscalJournalError, FiscalJournalKey,
    FiscalJournalSnapshot, FiscalPendingCommand, JournalEnvironment, JournalEvent, JournalOutcome,
    JournalState, MAX_FISCAL_JOURNAL_BYTES, MAX_FISCAL_JOURNAL_RECORDS, MIN_FISCAL_JOURNAL_BYTES,
    SCHEMA_VERSION, build_index, evidence, format, next_sequence, receipt, unix_now_ms,
    validate_command_id, validate_observed_at,
};
use crate::fiscal::{NfseEnvironment, NfseIssueRequest, NfseIssueResponse};
use std::{path::PathBuf, sync::Mutex};
use subtle::ConstantTimeEq as _;

impl FiscalCommandJournal {
    /// Opens or creates a journal with the crate's 16 MiB ceiling.
    pub fn try_open(
        path: impl Into<PathBuf>,
        key: FiscalJournalKey,
    ) -> Result<Self, FiscalJournalError> {
        Self::try_open_with_max_bytes(path, key, MAX_FISCAL_JOURNAL_BYTES)
    }

    /// Opens or creates a journal with an explicit smaller byte ceiling.
    pub fn try_open_with_max_bytes(
        path: impl Into<PathBuf>,
        key: FiscalJournalKey,
        max_bytes: u64,
    ) -> Result<Self, FiscalJournalError> {
        if !(MIN_FISCAL_JOURNAL_BYTES..=MAX_FISCAL_JOURNAL_BYTES).contains(&max_bytes) {
            return Err(FiscalJournalError::InvalidCapacity);
        }
        let path = path.into();
        let (file, events) = format::open(&path, max_bytes, &key)?;
        let commands = build_index(&events)?;
        Ok(Self {
            state: Mutex::new(JournalState { file, commands }),
            key,
            max_bytes,
        })
    }

    /// Synchronizes one prepared command using the current system time.
    ///
    /// The opaque command ID must not contain personal or fiscal data. An exact
    /// command/request replay returns `Replay`; reuse with different material fails.
    pub fn prepare(
        &self,
        command_id: impl Into<String>,
        environment: NfseEnvironment,
        request: &NfseIssueRequest,
    ) -> Result<FiscalCommandReceipt, FiscalJournalError> {
        self.prepare_at(command_id, environment, request, unix_now_ms()?)
    }

    /// Synchronizes one prepared command with an explicit trusted Unix-millisecond time.
    pub fn prepare_at(
        &self,
        command_id: impl Into<String>,
        environment: NfseEnvironment,
        request: &NfseIssueRequest,
        observed_at_unix_ms: i64,
    ) -> Result<FiscalCommandReceipt, FiscalJournalError> {
        let command_id = command_id.into();
        validate_command_id(&command_id)?;
        validate_observed_at(observed_at_unix_ms)?;
        let environment = JournalEnvironment::from_execution(environment)?;
        if environment != JournalEnvironment::from_api(request.environment()) {
            return Err(FiscalJournalError::ResponseMismatch);
        }
        let request_digest = evidence::request_fingerprint(request)?;
        let mut state = self.lock_and_refresh()?;
        if let Some(existing) = state.commands.get(&command_id) {
            if existing.environment == environment && existing.request_digest == request_digest {
                return Ok(receipt(existing, FiscalJournalDisposition::Replay));
            }
            return Err(FiscalJournalError::CommandConflict);
        }
        if state.file.records >= MAX_FISCAL_JOURNAL_RECORDS {
            return Err(FiscalJournalError::RecordCapacityExceeded);
        }
        let sequence = next_sequence(state.file.records)?;
        let event = JournalEvent {
            schema_version: SCHEMA_VERSION,
            sequence,
            command_id: command_id.clone(),
            environment,
            request_digest: request_digest.clone(),
            observed_at_unix_ms,
            outcome: JournalOutcome::Prepared,
        };
        format::append(&mut state.file, self.max_bytes, &self.key, &event)?;
        state.commands.insert(
            command_id,
            CommandState {
                environment,
                request_digest,
                prepared_at_unix_ms: observed_at_unix_ms,
                prepared_sequence: sequence,
                status: FiscalCommandStatus::Prepared,
                result_digest: None,
                sequence,
            },
        );
        Ok(FiscalCommandReceipt {
            disposition: FiscalJournalDisposition::Recorded,
            status: FiscalCommandStatus::Prepared,
            sequence,
        })
    }

    /// Synchronizes one parsed terminal response using the current system time.
    pub fn record_response(
        &self,
        command_id: &str,
        request: &NfseIssueRequest,
        response: &NfseIssueResponse,
    ) -> Result<FiscalCommandReceipt, FiscalJournalError> {
        self.record_response_at(command_id, request, response, unix_now_ms()?)
    }

    /// Synchronizes one parsed terminal response with an explicit trusted time.
    pub fn record_response_at(
        &self,
        command_id: &str,
        request: &NfseIssueRequest,
        response: &NfseIssueResponse,
        observed_at_unix_ms: i64,
    ) -> Result<FiscalCommandReceipt, FiscalJournalError> {
        validate_command_id(command_id)?;
        validate_observed_at(observed_at_unix_ms)?;
        let request_digest = evidence::request_fingerprint(request)?;
        let terminal = evidence::response_evidence(request, response)?;
        let mut state = self.lock_and_refresh()?;
        let existing = state
            .commands
            .get(command_id)
            .ok_or(FiscalJournalError::MissingCommand)?;
        if existing.environment != terminal.environment || existing.request_digest != request_digest
        {
            return Err(FiscalJournalError::CommandConflict);
        }
        if existing.status != FiscalCommandStatus::Prepared {
            if existing.status == terminal.status
                && existing.result_digest.as_deref() == Some(terminal.result_digest.as_str())
            {
                return Ok(receipt(existing, FiscalJournalDisposition::Replay));
            }
            return Err(FiscalJournalError::CommandConflict);
        }
        if observed_at_unix_ms < existing.prepared_at_unix_ms {
            return Err(FiscalJournalError::ResponseMismatch);
        }
        if state.file.records >= MAX_FISCAL_JOURNAL_RECORDS {
            return Err(FiscalJournalError::RecordCapacityExceeded);
        }
        let sequence = next_sequence(state.file.records)?;
        let outcome = match terminal.status {
            FiscalCommandStatus::Authorized => JournalOutcome::Authorized {
                result_digest: terminal.result_digest.clone(),
                processed_at_unix_ms: terminal.processed_at_unix_ms,
            },
            FiscalCommandStatus::Rejected => JournalOutcome::Rejected {
                result_digest: terminal.result_digest.clone(),
                http_status: terminal
                    .http_status
                    .ok_or(FiscalJournalError::ResponseMismatch)?,
                processed_at_unix_ms: terminal.processed_at_unix_ms,
            },
            FiscalCommandStatus::Prepared => return Err(FiscalJournalError::ResponseMismatch),
        };
        let event = JournalEvent {
            schema_version: SCHEMA_VERSION,
            sequence,
            command_id: command_id.to_string(),
            environment: terminal.environment,
            request_digest,
            observed_at_unix_ms,
            outcome,
        };
        format::append(&mut state.file, self.max_bytes, &self.key, &event)?;
        let existing = state
            .commands
            .get_mut(command_id)
            .ok_or(FiscalJournalError::MissingCommand)?;
        existing.status = terminal.status;
        existing.result_digest = Some(terminal.result_digest);
        existing.sequence = sequence;
        Ok(FiscalCommandReceipt {
            disposition: FiscalJournalDisposition::Recorded,
            status: existing.status,
            sequence,
        })
    }

    /// Returns ordered minimized descriptors for commands without terminal evidence.
    pub fn pending(&self) -> Result<Vec<FiscalPendingCommand>, FiscalJournalError> {
        let state = self.lock_and_refresh()?;
        let mut pending = state
            .commands
            .iter()
            .filter(|(_, command)| command.status == FiscalCommandStatus::Prepared)
            .map(|(command_id, command)| FiscalPendingCommand {
                command_id: command_id.clone(),
                environment: command.environment.execution(),
                request_digest: command.request_digest.clone(),
                prepared_at_unix_ms: command.prepared_at_unix_ms,
                sequence: command.prepared_sequence,
            })
            .collect::<Vec<_>>();
        pending.sort_by_key(FiscalPendingCommand::sequence);
        Ok(pending)
    }

    /// Returns the current state for one opaque command, if present.
    pub fn status(
        &self,
        command_id: &str,
    ) -> Result<Option<FiscalCommandStatus>, FiscalJournalError> {
        validate_command_id(command_id)?;
        let state = self.lock_and_refresh()?;
        Ok(state.commands.get(command_id).map(|command| command.status))
    }

    /// Returns bounded record, command and byte counters after authenticating the file.
    pub fn snapshot(&self) -> Result<FiscalJournalSnapshot, FiscalJournalError> {
        let state = self.lock_and_refresh()?;
        let pending = state
            .commands
            .values()
            .filter(|command| command.status == FiscalCommandStatus::Prepared)
            .count();
        Ok(FiscalJournalSnapshot {
            records: state.file.records,
            pending,
            terminal: state.commands.len().saturating_sub(pending),
            bytes: state.file.bytes,
            max_bytes: self.max_bytes,
        })
    }

    /// Returns the authenticated exact journal tip for independent persistence.
    pub fn checkpoint(&self) -> Result<FiscalJournalCheckpoint, FiscalJournalError> {
        let state = self.lock_and_refresh()?;
        Ok(FiscalJournalCheckpoint {
            sequence: state.file.records as u64,
            end_offset: state.file.bytes,
            commitment: hex::encode(state.file.last_tag),
        })
    }

    /// Requires the current journal tip to equal an independently retained checkpoint.
    pub fn verify_checkpoint(
        &self,
        expected: &FiscalJournalCheckpoint,
    ) -> Result<(), FiscalJournalError> {
        let actual = self.checkpoint()?;
        let commitments_match = actual
            .commitment
            .as_bytes()
            .ct_eq(expected.commitment.as_bytes())
            .unwrap_u8()
            == 1;
        if actual.sequence != expected.sequence
            || actual.end_offset != expected.end_offset
            || !commitments_match
        {
            return Err(FiscalJournalError::CheckpointMismatch);
        }
        Ok(())
    }

    fn lock_and_refresh(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, JournalState>, FiscalJournalError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| FiscalJournalError::LockUnavailable)?;
        let events = format::verify_and_read(&mut state.file, self.max_bytes, &self.key)?;
        state.commands = build_index(&events)?;
        Ok(state)
    }
}
