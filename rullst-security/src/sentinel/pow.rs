use super::SentinelError;
use dashmap::{DashMap, mapref::entry::Entry};
use rand::{TryRng, rngs::SysRng};
use serde::Serialize;
use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

mod codec;

use codec::{
    CHALLENGE_ID_BYTES, ParsedToken, SUBJECT_TAG_BYTES, TOKEN_VERSION, build_token, decode_token,
    proof_matches, subject_tag, verify_token_mac,
};

const MIN_KEY_BYTES: usize = 32;
const MAX_SUBJECT_BYTES: usize = 256;
const CLEANUP_INTERVAL_SECONDS: u64 = 30;

/// Resource limits for the process-local one-shot proof-of-work gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ProofOfWorkConfig {
    difficulty_bits: u8,
    ttl: Duration,
    max_active_challenges: usize,
}

impl ProofOfWorkConfig {
    /// Creates a policy bounded to 8–24 leading zero bits, 5–300 seconds, and
    /// 1–100,000 active process-local challenges.
    pub fn try_new(
        difficulty_bits: u8,
        ttl: Duration,
        max_active_challenges: usize,
    ) -> Result<Self, SentinelError> {
        if !(8..=24).contains(&difficulty_bits) {
            return Err(SentinelError::InvalidConfiguration("PoW difficulty"));
        }
        if !(5..=300).contains(&ttl.as_secs()) || ttl.subsec_nanos() != 0 {
            return Err(SentinelError::InvalidConfiguration("PoW TTL"));
        }
        if !(1..=100_000).contains(&max_active_challenges) {
            return Err(SentinelError::InvalidConfiguration("PoW capacity"));
        }
        Ok(Self {
            difficulty_bits,
            ttl,
            max_active_challenges,
        })
    }

    pub fn difficulty_bits(self) -> u8 {
        self.difficulty_bits
    }

    pub fn ttl(self) -> Duration {
        self.ttl
    }

    pub fn max_active_challenges(self) -> usize {
        self.max_active_challenges
    }
}

impl Default for ProofOfWorkConfig {
    fn default() -> Self {
        Self {
            difficulty_bits: 18,
            ttl: Duration::from_secs(120),
            max_active_challenges: 10_000,
        }
    }
}

/// Client-facing challenge. The opaque token is integrity- and subject-bound.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct ProofOfWorkChallenge {
    token: String,
    difficulty_bits: u8,
    expires_at_epoch: u64,
}

impl ProofOfWorkChallenge {
    pub fn token(&self) -> &str {
        &self.token
    }

    pub fn difficulty_bits(&self) -> u8 {
        self.difficulty_bits
    }

    pub fn expires_at_epoch(&self) -> u64 {
        self.expires_at_epoch
    }

    /// Checks a candidate locally. Successful admission still requires the
    /// server-side one-shot [`ProofOfWorkGate::verify`] call.
    pub fn is_solution(&self, nonce: u64) -> bool {
        decode_token(&self.token)
            .map(|bytes| proof_matches(&bytes, nonce, self.difficulty_bits))
            .unwrap_or(false)
    }
}

#[derive(Clone, Copy)]
struct ActiveChallenge {
    subject_tag: [u8; SUBJECT_TAG_BYTES],
    expires_at_epoch: u64,
}

struct ProofOfWorkInner {
    key: Zeroizing<Vec<u8>>,
    config: ProofOfWorkConfig,
    active: DashMap<[u8; CHALLENGE_ID_BYTES], ActiveChallenge>,
    active_count: AtomicUsize,
    last_cleanup_epoch: AtomicU64,
}

/// HMAC-authenticated, expiring and process-local one-shot proof-of-work gate.
#[derive(Clone)]
#[non_exhaustive]
pub struct ProofOfWorkGate {
    inner: Arc<ProofOfWorkInner>,
}

impl std::fmt::Debug for ProofOfWorkGate {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProofOfWorkGate")
            .field("config", &self.inner.config)
            .field(
                "active_challenges",
                &self.inner.active_count.load(Ordering::Relaxed),
            )
            .finish_non_exhaustive()
    }
}

impl ProofOfWorkGate {
    /// Creates a gate with a high-entropy application key.
    pub fn try_new(
        key: impl AsRef<[u8]>,
        config: ProofOfWorkConfig,
    ) -> Result<Self, SentinelError> {
        let key = key.as_ref();
        validate_key(key)?;
        Ok(Self {
            inner: Arc::new(ProofOfWorkInner {
                key: Zeroizing::new(key.to_vec()),
                config,
                active: DashMap::new(),
                active_count: AtomicUsize::new(0),
                last_cleanup_epoch: AtomicU64::new(0),
            }),
        })
    }

    /// Issues a random challenge bound to one canonical application subject.
    pub fn issue(&self, subject: impl Into<String>) -> Result<ProofOfWorkChallenge, SentinelError> {
        self.issue_at(subject.into(), current_epoch()?)
    }

    /// Verifies authenticity, subject, time and work before atomically consuming
    /// the challenge. Exactly one concurrent verifier can succeed locally.
    pub fn verify(
        &self,
        subject: impl Into<String>,
        token: impl Into<String>,
        solution_nonce: u64,
    ) -> Result<(), SentinelError> {
        self.verify_at(
            &subject.into(),
            &token.into(),
            solution_nonce,
            current_epoch()?,
        )
    }

    /// Current process-local challenge cardinality.
    pub fn active_challenges(&self) -> usize {
        self.inner.active_count.load(Ordering::Acquire)
    }

    pub(super) fn issue_at(
        &self,
        subject: String,
        now_epoch: u64,
    ) -> Result<ProofOfWorkChallenge, SentinelError> {
        validate_subject(&subject)?;
        self.cleanup_if_due(now_epoch);
        let subject_tag = subject_tag(&self.inner.key, &subject)?;
        let expires_at_epoch = now_epoch
            .checked_add(self.inner.config.ttl.as_secs())
            .ok_or(SentinelError::ClockUnavailable)?;
        self.reserve_slot()?;
        for _ in 0..4 {
            let mut challenge_id = [0_u8; CHALLENGE_ID_BYTES];
            if SysRng.try_fill_bytes(&mut challenge_id).is_err() {
                self.release_slot();
                return Err(SentinelError::RandomnessUnavailable);
            }
            let token = match build_token(
                &self.inner.key,
                challenge_id,
                subject_tag,
                now_epoch,
                expires_at_epoch,
                self.inner.config.difficulty_bits,
            ) {
                Ok(token) => token,
                Err(error) => {
                    self.release_slot();
                    return Err(error);
                }
            };
            match self.inner.active.entry(challenge_id) {
                Entry::Vacant(entry) => {
                    entry.insert(ActiveChallenge {
                        subject_tag,
                        expires_at_epoch,
                    });
                    return Ok(ProofOfWorkChallenge {
                        token,
                        difficulty_bits: self.inner.config.difficulty_bits,
                        expires_at_epoch,
                    });
                }
                Entry::Occupied(_) => {}
            }
        }
        self.release_slot();
        Err(SentinelError::RandomnessUnavailable)
    }

    pub(super) fn verify_at(
        &self,
        subject: &str,
        token: &str,
        solution_nonce: u64,
        now_epoch: u64,
    ) -> Result<(), SentinelError> {
        validate_subject(subject)?;
        let bytes = decode_token(token)?;
        verify_token_mac(&self.inner.key, &bytes)?;
        let parsed = ParsedToken::from_bytes(&bytes)?;
        if parsed.version != TOKEN_VERSION
            || parsed.difficulty_bits != self.inner.config.difficulty_bits
            || parsed.expires_at_epoch.checked_sub(parsed.issued_at_epoch)
                != Some(self.inner.config.ttl.as_secs())
        {
            return Err(SentinelError::InvalidToken);
        }
        let expected_subject = subject_tag(&self.inner.key, subject)?;
        if !bool::from(expected_subject.ct_eq(&parsed.subject_tag)) {
            return Err(SentinelError::InvalidToken);
        }
        if now_epoch < parsed.issued_at_epoch || now_epoch >= parsed.expires_at_epoch {
            self.remove_exact(&parsed);
            return Err(SentinelError::ExpiredChallenge);
        }
        if !proof_matches(&bytes, solution_nonce, parsed.difficulty_bits) {
            return Err(SentinelError::InvalidProof);
        }

        let removed = self
            .inner
            .active
            .remove_if(&parsed.challenge_id, |_, active| {
                active.expires_at_epoch == parsed.expires_at_epoch
                    && bool::from(active.subject_tag.ct_eq(&parsed.subject_tag))
            });
        if removed.is_some() {
            self.release_slot();
            Ok(())
        } else {
            Err(SentinelError::ReplayOrUnknownChallenge)
        }
    }

    fn reserve_slot(&self) -> Result<(), SentinelError> {
        let mut current = self.inner.active_count.load(Ordering::Acquire);
        loop {
            if current >= self.inner.config.max_active_challenges {
                return Err(SentinelError::CapacityReached);
            }
            match self.inner.active_count.compare_exchange_weak(
                current,
                current + 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return Ok(()),
                Err(observed) => current = observed,
            }
        }
    }

    fn release_slot(&self) {
        let mut current = self.inner.active_count.load(Ordering::Acquire);
        loop {
            match self.inner.active_count.compare_exchange_weak(
                current,
                current.saturating_sub(1),
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return,
                Err(observed) => current = observed,
            }
        }
    }

    fn remove_exact(&self, parsed: &ParsedToken) {
        if self
            .inner
            .active
            .remove_if(&parsed.challenge_id, |_, active| {
                active.expires_at_epoch == parsed.expires_at_epoch
                    && bool::from(active.subject_tag.ct_eq(&parsed.subject_tag))
            })
            .is_some()
        {
            self.release_slot();
        }
    }

    fn cleanup_if_due(&self, now_epoch: u64) {
        let previous = self.inner.last_cleanup_epoch.load(Ordering::Acquire);
        if now_epoch.saturating_sub(previous) < CLEANUP_INTERVAL_SECONDS
            || self
                .inner
                .last_cleanup_epoch
                .compare_exchange(previous, now_epoch, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
        {
            return;
        }
        let expired = self
            .inner
            .active
            .iter()
            .filter(|entry| entry.expires_at_epoch <= now_epoch)
            .map(|entry| *entry.key())
            .collect::<Vec<_>>();
        for challenge_id in expired {
            if self.inner.active.remove(&challenge_id).is_some() {
                self.release_slot();
            }
        }
    }
}

fn validate_key(key: &[u8]) -> Result<(), SentinelError> {
    let mut distinct = [false; 256];
    for byte in key {
        distinct[usize::from(*byte)] = true;
    }
    if key.len() < MIN_KEY_BYTES || distinct.iter().filter(|present| **present).count() < 12 {
        Err(SentinelError::WeakKey)
    } else {
        Ok(())
    }
}

fn validate_subject(subject: &str) -> Result<(), SentinelError> {
    if subject.is_empty()
        || subject.trim() != subject
        || subject.len() > MAX_SUBJECT_BYTES
        || subject.chars().any(char::is_control)
    {
        Err(SentinelError::InvalidSubject)
    } else {
        Ok(())
    }
}

fn current_epoch() -> Result<u64, SentinelError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|_| SentinelError::ClockUnavailable)
}
