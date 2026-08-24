use crate::error::AuthError;
use base64::Engine as _;
use rand::Rng;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, Instant},
};
use subtle::ConstantTimeEq;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Ceremony {
    Registration,
    Authentication,
}

#[derive(Debug)]
struct PendingChallenge {
    ceremony: Ceremony,
    expires_at: Instant,
    allowed_credential_ids: Vec<Vec<u8>>,
}

/// Generates a cryptographically random 256-bit WebAuthn challenge encoded as base64url.
#[cfg_attr(mutants, mutants::skip)]
pub fn generate_challenge() -> String {
    let mut bytes = [0_u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn passkey_error(message: impl Into<String>) -> AuthError {
    AuthError::PasskeyError(message.into())
}

#[derive(Clone)]
pub(super) struct ChallengeStore {
    ttl: Duration,
    maximum: usize,
    pending: Arc<Mutex<HashMap<String, PendingChallenge>>>,
}

impl ChallengeStore {
    pub(super) fn new(ttl: Duration, maximum: usize) -> Self {
        Self {
            ttl,
            maximum,
            pending: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn pending(&self) -> Result<MutexGuard<'_, HashMap<String, PendingChallenge>>, AuthError> {
        self.pending
            .lock()
            .map_err(|_| passkey_error("WebAuthn challenge store is unavailable"))
    }

    pub(super) fn issue(
        &self,
        ceremony: Ceremony,
        allowed_credential_ids: Vec<Vec<u8>>,
    ) -> Result<String, AuthError> {
        let now = Instant::now();
        let mut pending = self.pending()?;
        pending.retain(|_, challenge| challenge.expires_at > now);
        if pending.len() >= self.maximum {
            return Err(passkey_error("too many outstanding WebAuthn challenges"));
        }

        for _ in 0..4 {
            let challenge = generate_challenge();
            if !pending.contains_key(&challenge) {
                pending.insert(
                    challenge.clone(),
                    PendingChallenge {
                        ceremony,
                        expires_at: now + self.ttl,
                        allowed_credential_ids,
                    },
                );
                return Ok(challenge);
            }
        }
        Err(passkey_error(
            "could not allocate a unique WebAuthn challenge",
        ))
    }

    pub(super) fn consume(
        &self,
        expected: &str,
        received: &str,
        ceremony: Ceremony,
        credential_id: Option<&[u8]>,
    ) -> Result<(), AuthError> {
        if expected.as_bytes().ct_eq(received.as_bytes()).unwrap_u8() != 1 {
            return Err(passkey_error("Challenge mismatch"));
        }

        let challenge = self
            .pending()?
            .remove(expected)
            .ok_or_else(|| passkey_error("challenge is unknown, expired, or already consumed"))?;
        if challenge.expires_at <= Instant::now() {
            return Err(passkey_error("challenge has expired"));
        }
        if challenge.ceremony != ceremony {
            return Err(passkey_error(
                "challenge was issued for a different ceremony",
            ));
        }
        if ceremony == Ceremony::Authentication
            && !challenge.allowed_credential_ids.is_empty()
            && credential_id.is_none_or(|credential_id| {
                !challenge
                    .allowed_credential_ids
                    .iter()
                    .any(|allowed| allowed.as_slice().ct_eq(credential_id).unwrap_u8() == 1)
            })
        {
            return Err(passkey_error(
                "credential was not allowed for this authentication ceremony",
            ));
        }
        Ok(())
    }
}
