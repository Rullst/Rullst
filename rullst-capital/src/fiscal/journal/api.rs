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
            return Err(FiscalJournalError::EnvironmentMismatch);
        }
        let request_digest = evidence::request_fingerprint(request)?;
        let mut state = self.lock_and_refresh()?;
        if let Some(existing) = state.commands.get(&command_id) {
            if existing.environment == environment && existing.request_digest == request_digest {
                return Ok(receipt(existing, FiscalJournalDisposition::Replay));
            }
            return Err(FiscalJournalError::CommandConflict);
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
        reserve_terminal_capacity(&state, &event, self.max_bytes)?;
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
    ///
    /// The wall clock is not monotonic: if it stepped back below the
    /// preparation time, the observation is recorded at the preparation time.
    pub fn record_response(
        &self,
        command_id: &str,
        request: &NfseIssueRequest,
        response: &NfseIssueResponse,
    ) -> Result<FiscalCommandReceipt, FiscalJournalError> {
        self.record_terminal(command_id, request, response, unix_now_ms()?, true)
    }

    /// Synchronizes one parsed terminal response with an explicit trusted time.
    ///
    /// A time before the command's preparation returns `ClockRegression`.
    pub fn record_response_at(
        &self,
        command_id: &str,
        request: &NfseIssueRequest,
        response: &NfseIssueResponse,
        observed_at_unix_ms: i64,
    ) -> Result<FiscalCommandReceipt, FiscalJournalError> {
        self.record_terminal(command_id, request, response, observed_at_unix_ms, false)
    }

    fn record_terminal(
        &self,
        command_id: &str,
        request: &NfseIssueRequest,
        response: &NfseIssueResponse,
        observed_at_unix_ms: i64,
        clamp_to_preparation: bool,
    ) -> Result<FiscalCommandReceipt, FiscalJournalError> {
        validate_command_id(command_id)?;
        validate_observed_at(observed_at_unix_ms)?;
        let request_digest = evidence::request_fingerprint(request)?;
        let terminal = evidence::response_evidence(request, response)?;
        // An HTTP 500 may follow an issued NFS-e (for example a lost 201 and a
        // retry), so it must not become a final rejection.
        if terminal.http_status == Some(500) {
            return Err(FiscalJournalError::IndeterminateResponse);
        }
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
        let observed_at_unix_ms = if observed_at_unix_ms >= existing.prepared_at_unix_ms {
            observed_at_unix_ms
        } else if clamp_to_preparation {
            existing.prepared_at_unix_ms
        } else {
            return Err(FiscalJournalError::ClockRegression);
        };
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
    ///
    /// A crash after an append but before the new checkpoint is persisted makes
    /// an intact journal fail this exact check; see
    /// [`Self::verify_checkpoint_prefix`].
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

    /// Verifies that an independently retained checkpoint is an authenticated
    /// prefix of the current journal and returns how many events follow it.
    ///
    /// `Ok(0)` means the checkpoint is the exact tip. A positive count covers a
    /// crash after an append was synchronized but before its new checkpoint
    /// was persisted; truncation or substitution still returns
    /// `CheckpointMismatch`. New in 13.0.
    pub fn verify_checkpoint_prefix(
        &self,
        expected: &FiscalJournalCheckpoint,
    ) -> Result<u64, FiscalJournalError> {
        let mut state = self.lock_and_refresh()?;
        let boundaries = format::chain_boundaries(&mut state.file, self.max_bytes, &self.key)?;
        let (end_offset, tag) = usize::try_from(expected.sequence)
            .ok()
            .and_then(|index| boundaries.get(index))
            .ok_or(FiscalJournalError::CheckpointMismatch)?;
        let commitment_matches = hex::encode(tag)
            .as_bytes()
            .ct_eq(expected.commitment.as_bytes())
            .unwrap_u8()
            == 1;
        if *end_offset != expected.end_offset || !commitment_matches {
            return Err(FiscalJournalError::CheckpointMismatch);
        }
        Ok((state.file.records as u64).saturating_sub(expected.sequence))
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

// A prepared command is transmitted after this call, so the journal must still
// be able to record its terminal result and that of every other pending
// command. Reject the preparation, before anything is sent, when the records or
// bytes that those terminal events may need are not available.
fn reserve_terminal_capacity(
    state: &JournalState,
    prepared: &JournalEvent,
    max_bytes: u64,
) -> Result<(), FiscalJournalError> {
    let pending = state
        .commands
        .iter()
        .filter(|(_, command)| command.status == FiscalCommandStatus::Prepared);
    let (pending_count, pending_id_bytes) =
        pending.fold((0_usize, 0_u64), |(count, bytes), (id, _)| {
            (
                count.saturating_add(1),
                bytes.saturating_add(id.len() as u64),
            )
        });
    // This preparation plus one terminal event for it and for each pending one.
    if state
        .file
        .records
        .saturating_add(pending_count)
        .saturating_add(2)
        > MAX_FISCAL_JOURNAL_RECORDS
    {
        return Err(FiscalJournalError::RecordCapacityExceeded);
    }
    let terminal_base = terminal_frame_bytes_without_id()?;
    let terminals = terminal_base
        .saturating_mul(pending_count.saturating_add(1) as u64)
        .saturating_add(pending_id_bytes)
        .saturating_add(prepared.command_id.len() as u64);
    let required = state
        .file
        .bytes
        .saturating_add(format::frame_bytes(prepared)?)
        .saturating_add(terminals);
    if required > max_bytes {
        return Err(FiscalJournalError::CapacityExceeded);
    }
    Ok(())
}

// Largest terminal frame for an empty command ID. Command IDs never need JSON
// escaping, so a command's terminal frame is at most this plus its ID length.
fn terminal_frame_bytes_without_id() -> Result<u64, FiscalJournalError> {
    let digest = "f".repeat(64);
    format::frame_bytes(&JournalEvent {
        schema_version: SCHEMA_VERSION,
        sequence: u64::MAX,
        command_id: String::new(),
        environment: JournalEnvironment::Homologation,
        request_digest: digest.clone(),
        observed_at_unix_ms: i64::MAX,
        outcome: JournalOutcome::Rejected {
            result_digest: digest,
            http_status: 500,
            processed_at_unix_ms: i64::MAX,
        },
    })
}
