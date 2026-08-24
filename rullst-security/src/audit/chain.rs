use crate::error::SecurityError;
use hmac::{Hmac, KeyInit, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::sync::Arc;
use subtle::ConstantTimeEq;
use tokio::sync::Mutex;
use zeroize::Zeroizing;

type HmacSha256 = Hmac<Sha256>;

const AUDIT_DOMAIN: &[u8] = b"RULLST-AUDIT-CHAIN\0V1";
const GENESIS_HASH: &str = "GENESIS_HASH";

/// Minimum HMAC key length accepted by the tamper-evident audit chain.
pub const MIN_AUDIT_KEY_BYTES: usize = 32;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditRecord {
    pub sequence_id: u64,
    pub timestamp: u64,
    pub actor: String,
    pub action: String,
    pub resource: String,
    pub payload: String,
    pub previous_hash: String,
    pub hash: String,
}

pub trait AuditLogger: Send + Sync {
    fn log(&self, record: &AuditRecord) -> Result<(), SecurityError>;
}

#[derive(Default)]
pub struct StdoutAuditLogger;

impl AuditLogger for StdoutAuditLogger {
    fn log(&self, record: &AuditRecord) -> Result<(), SecurityError> {
        println!(
            "[AUDIT LOG #{}] actor={} action={} resource={} hash={}",
            record.sequence_id, record.actor, record.action, record.resource, record.hash
        );
        Ok(())
    }
}

struct AuditState {
    last_hash: String,
    sequence: u64,
}

pub struct AuditChain {
    secret_key: Zeroizing<Vec<u8>>,
    state: Arc<Mutex<AuditState>>,
    logger: Arc<dyn AuditLogger>,
}

impl AuditChain {
    /// Creates an audit chain after enforcing a 256-bit-or-longer HMAC key.
    pub fn try_new(secret_key: &[u8], logger: Arc<dyn AuditLogger>) -> Result<Self, SecurityError> {
        validate_secret_key(secret_key)?;
        Ok(Self::new_inner(secret_key, logger))
    }

    /// Compatibility constructor. Prefer [`AuditChain::try_new`] for startup-time validation.
    ///
    /// A chain built with an invalid key is inert: [`AuditChain::record_event`] returns a typed
    /// error and verification always fails. It never signs data with a weak or empty key.
    #[deprecated(
        since = "12.0.0",
        note = "use `AuditChain::try_new` to reject weak HMAC keys during startup"
    )]
    pub fn new(secret_key: &[u8], logger: Arc<dyn AuditLogger>) -> Self {
        Self::new_inner(secret_key, logger)
    }

    fn new_inner(secret_key: &[u8], logger: Arc<dyn AuditLogger>) -> Self {
        Self {
            secret_key: Zeroizing::new(secret_key.to_vec()),
            state: Arc::new(Mutex::new(AuditState {
                last_hash: GENESIS_HASH.to_string(),
                sequence: 0,
            })),
            logger,
        }
    }

    pub async fn record_event(
        &self,
        actor: &str,
        action: &str,
        resource: &str,
        payload: &str,
    ) -> Result<AuditRecord, SecurityError> {
        validate_secret_key(&self.secret_key)?;

        // Sequence and predecessor are one atomic state transition. The state is committed only
        // after the logger accepts the record, so logger failures cannot leave a sequence gap.
        let mut state = self.state.lock().await;
        let sequence_id = state.sequence.checked_add(1).ok_or_else(|| {
            SecurityError::AuditChainError("audit sequence counter exhausted".to_string())
        })?;
        let timestamp = unix_timestamp_secs();
        let previous_hash = state.last_hash.clone();
        let material = canonical_record_material(
            sequence_id,
            timestamp,
            actor,
            action,
            resource,
            payload,
            &previous_hash,
        )?;
        let hash = sign_material(&self.secret_key, &material)?;

        let record = AuditRecord {
            sequence_id,
            timestamp,
            actor: actor.to_string(),
            action: action.to_string(),
            resource: resource.to_string(),
            payload: payload.to_string(),
            previous_hash,
            hash: hash.clone(),
        };

        self.logger.log(&record)?;
        state.sequence = sequence_id;
        state.last_hash = hash;
        Ok(record)
    }

    /// Verifies the HMAC of one record. Use [`AuditChain::verify_sequence`] when validating a
    /// persisted chain because an isolated valid record does not prove sequence continuity.
    pub fn verify_record(secret_key: &[u8], record: &AuditRecord) -> bool {
        if validate_secret_key(secret_key).is_err() {
            return false;
        }
        let Ok(material) = canonical_record_material(
            record.sequence_id,
            record.timestamp,
            &record.actor,
            &record.action,
            &record.resource,
            &record.payload,
            &record.previous_hash,
        ) else {
            return false;
        };
        let Ok(expected_hash) = sign_material(secret_key, &material) else {
            return false;
        };

        constant_time_equal(expected_hash.as_bytes(), record.hash.as_bytes())
    }

    /// Verifies every HMAC plus genesis, sequence-number, and predecessor continuity.
    pub fn verify_sequence(secret_key: &[u8], records: &[AuditRecord]) -> bool {
        if validate_secret_key(secret_key).is_err() {
            return false;
        }

        let mut expected_sequence = 1_u64;
        let mut expected_previous_hash = GENESIS_HASH;
        for record in records {
            if record.sequence_id != expected_sequence
                || !constant_time_equal(
                    record.previous_hash.as_bytes(),
                    expected_previous_hash.as_bytes(),
                )
                || !Self::verify_record(secret_key, record)
            {
                return false;
            }
            let Some(next_sequence) = expected_sequence.checked_add(1) else {
                return false;
            };
            expected_sequence = next_sequence;
            expected_previous_hash = &record.hash;
        }
        true
    }
}

fn validate_secret_key(secret_key: &[u8]) -> Result<(), SecurityError> {
    if secret_key.len() < MIN_AUDIT_KEY_BYTES {
        return Err(SecurityError::AuditChainError(format!(
            "audit HMAC key must contain at least {MIN_AUDIT_KEY_BYTES} bytes"
        )));
    }
    let mut observed = [false; 256];
    for byte in secret_key {
        observed[usize::from(*byte)] = true;
    }
    if observed.into_iter().filter(|seen| *seen).count() < 8 {
        return Err(SecurityError::AuditChainError(
            "audit HMAC key has insufficient byte diversity; use a random 256-bit key".to_string(),
        ));
    }
    Ok(())
}

fn unix_timestamp_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn canonical_record_material(
    sequence_id: u64,
    timestamp: u64,
    actor: &str,
    action: &str,
    resource: &str,
    payload: &str,
    previous_hash: &str,
) -> Result<Vec<u8>, SecurityError> {
    let mut material = Vec::new();
    append_field(&mut material, AUDIT_DOMAIN)?;
    material.extend_from_slice(&sequence_id.to_be_bytes());
    material.extend_from_slice(&timestamp.to_be_bytes());
    append_field(&mut material, actor.as_bytes())?;
    append_field(&mut material, action.as_bytes())?;
    append_field(&mut material, resource.as_bytes())?;
    append_field(&mut material, payload.as_bytes())?;
    append_field(&mut material, previous_hash.as_bytes())?;
    Ok(material)
}

fn append_field(material: &mut Vec<u8>, field: &[u8]) -> Result<(), SecurityError> {
    let length = u64::try_from(field.len()).map_err(|_| {
        SecurityError::AuditChainError("audit field is too large to serialize".to_string())
    })?;
    material.extend_from_slice(&length.to_be_bytes());
    material.extend_from_slice(field);
    Ok(())
}

fn sign_material(secret_key: &[u8], material: &[u8]) -> Result<String, SecurityError> {
    let mut mac = HmacSha256::new_from_slice(secret_key).map_err(|error| {
        SecurityError::AuditChainError(format!("HMAC key initialization failed: {error}"))
    })?;
    mac.update(material);
    Ok(hex::encode(mac.finalize().into_bytes()))
}

fn constant_time_equal(candidate: &[u8], expected: &[u8]) -> bool {
    candidate.len() == expected.len() && bool::from(candidate.ct_eq(expected))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    struct FailOnceLogger(AtomicBool);

    impl AuditLogger for FailOnceLogger {
        fn log(&self, _record: &AuditRecord) -> Result<(), SecurityError> {
            if self.0.swap(false, Ordering::SeqCst) {
                Err(SecurityError::AuditChainError(
                    "simulated durable logger failure".to_string(),
                ))
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn length_prefixing_prevents_delimiter_collisions() {
        let left =
            canonical_record_material(1, 2, "a:b", "c", "r", "p", "h").expect("canonical material");
        let right =
            canonical_record_material(1, 2, "a", "b:c", "r", "p", "h").expect("canonical material");

        assert_ne!(left, right);
    }

    #[test]
    fn weak_keys_are_rejected() {
        let logger = Arc::new(StdoutAuditLogger);
        assert!(AuditChain::try_new(b"", logger.clone()).is_err());
        assert!(AuditChain::try_new(b"short", logger.clone()).is_err());
        assert!(AuditChain::try_new(&[0_u8; MIN_AUDIT_KEY_BYTES], logger).is_err());
    }

    #[tokio::test]
    async fn logger_failure_does_not_create_a_sequence_gap() {
        let secret = b"audit-key-material-with-32-plus-bytes";
        let chain = AuditChain::try_new(secret, Arc::new(FailOnceLogger(AtomicBool::new(true))))
            .expect("strong audit key");

        assert!(
            chain
                .record_event("actor", "action", "resource", "payload")
                .await
                .is_err()
        );
        let first_durable = chain
            .record_event("actor", "action", "resource", "payload")
            .await
            .expect("second log succeeds");
        assert_eq!(first_durable.sequence_id, 1);
        assert_eq!(first_durable.previous_hash, GENESIS_HASH);
    }
}
