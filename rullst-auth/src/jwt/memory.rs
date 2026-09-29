//! Bounded process-local JWT revocation state.

use super::quota::must_widen;
use super::{
    ApplicationJwtClaims, JwtError, JwtRevocationMode, JwtRevocationStore, unix_time,
    valid_identifier, valid_identity,
};
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};

/// Bounded deterministic revocation state for development and single-process tests.
///
/// `max_entries` covers token and subject entries together. Token entries may
/// use at most three quarters of it and 64 per subject; beyond either bound,
/// `revoke_token` records a subject cutoff that rejects every token of that
/// subject issued no later than the revoked one. Subject entries are retained
/// for the lifetime of the store.
pub struct InMemoryJwtRevocationStore {
    state: Mutex<InMemoryRevocationState>,
    max_entries: usize,
}

#[derive(Default)]
struct InMemoryRevocationState {
    /// Active token revocations: `jti` to (`exp`, subject).
    revoked_tokens: HashMap<String, (u64, String)>,
    subjects: HashMap<String, SubjectRevocation>,
}

#[derive(Default)]
struct SubjectRevocation {
    /// Tokens with a lower `session_version` are revoked; zero revokes none.
    minimum_session_version: u64,
    /// Tokens issued at or before this `iat` are revoked.
    revoked_through_iat: Option<u64>,
}

impl InMemoryJwtRevocationStore {
    pub fn new(max_entries: usize) -> Result<Self, JwtError> {
        if !(1..=1_000_000).contains(&max_entries) {
            return Err(JwtError::InvalidConfiguration("max_entries"));
        }
        Ok(Self {
            state: Mutex::new(InMemoryRevocationState::default()),
            max_entries,
        })
    }

    /// Revokes one token until it expires. A subject at its token quota, or a
    /// store whose token share is full, widens to a subject cutoff instead.
    pub fn revoke_token(&self, claims: &ApplicationJwtClaims) -> Result<(), JwtError> {
        if !valid_identifier(&claims.jti, 64) {
            return Err(JwtError::InvalidConfiguration("jti"));
        }
        if !valid_identity(&claims.sub) {
            return Err(JwtError::InvalidConfiguration("subject"));
        }
        let now = unix_time()?;
        if claims.exp <= now {
            return Ok(());
        }
        let mut state = self.lock_state()?;
        state
            .revoked_tokens
            .retain(|_, (expires_at, _)| *expires_at > now);
        if let Some((expires_at, _)) = state.revoked_tokens.get_mut(&claims.jti) {
            *expires_at = (*expires_at).max(claims.exp);
            return Ok(());
        }
        let subject_tokens = state
            .revoked_tokens
            .values()
            .filter(|(_, subject)| *subject == claims.sub)
            .count();
        if must_widen(
            subject_tokens,
            state.revoked_tokens.len(),
            state.subjects.len(),
            self.max_entries,
        ) {
            let entry = self.subject_entry(&mut state, &claims.sub)?;
            entry.revoked_through_iat = Some(
                entry
                    .revoked_through_iat
                    .map_or(claims.iat, |cutoff| cutoff.max(claims.iat)),
            );
            return Ok(());
        }
        state
            .revoked_tokens
            .insert(claims.jti.clone(), (claims.exp, claims.sub.clone()));
        Ok(())
    }

    /// Rejects subject tokens whose `session_version` is lower than this value.
    pub fn revoke_subject_before(
        &self,
        subject: impl Into<String>,
        minimum_session_version: u64,
    ) -> Result<(), JwtError> {
        let subject = subject.into();
        if !valid_identity(&subject) || minimum_session_version == 0 {
            return Err(JwtError::InvalidConfiguration("subject revocation"));
        }
        let now = unix_time()?;
        let mut state = self.lock_state()?;
        state
            .revoked_tokens
            .retain(|_, (expires_at, _)| *expires_at > now);
        let entry = self.subject_entry(&mut state, &subject)?;
        entry.minimum_session_version = entry.minimum_session_version.max(minimum_session_version);
        Ok(())
    }

    pub fn entry_count(&self) -> Result<usize, JwtError> {
        let state = self.lock_state()?;
        Ok(Self::entry_count_locked(&state))
    }

    fn subject_entry<'a>(
        &self,
        state: &'a mut InMemoryRevocationState,
        subject: &str,
    ) -> Result<&'a mut SubjectRevocation, JwtError> {
        if !state.subjects.contains_key(subject)
            && Self::entry_count_locked(state) >= self.max_entries
        {
            return Err(JwtError::RevocationStoreCapacity);
        }
        Ok(state.subjects.entry(subject.to_owned()).or_default())
    }

    fn entry_count_locked(state: &InMemoryRevocationState) -> usize {
        state.revoked_tokens.len() + state.subjects.len()
    }

    fn lock_state(&self) -> Result<MutexGuard<'_, InMemoryRevocationState>, JwtError> {
        self.state
            .lock()
            .map_err(|_| JwtError::RevocationBackend("in-memory lock poisoned".to_string()))
    }
}

impl JwtRevocationStore for InMemoryJwtRevocationStore {
    fn mode(&self) -> JwtRevocationMode {
        JwtRevocationMode::ProcessLocal
    }

    fn is_revoked(&self, claims: &ApplicationJwtClaims, now: u64) -> Result<bool, JwtError> {
        let mut state = self.lock_state()?;
        state
            .revoked_tokens
            .retain(|_, (expires_at, _)| *expires_at > now);
        if state.revoked_tokens.contains_key(&claims.jti) {
            return Ok(true);
        }
        Ok(state.subjects.get(&claims.sub).is_some_and(|revocation| {
            claims.session_version < revocation.minimum_session_version
                || revocation
                    .revoked_through_iat
                    .is_some_and(|cutoff| claims.iat <= cutoff)
        }))
    }
}
