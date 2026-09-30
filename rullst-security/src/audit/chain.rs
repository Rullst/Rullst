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

/// Writes one line per record to standard output.
///
/// `actor`, `action` and `resource` are printed as quoted, escaped strings, so
/// a value containing a line break, quote or `key=` text cannot forge another
/// audit line or field. The payload is not printed.
#[derive(Default)]
pub struct StdoutAuditLogger;

impl AuditLogger for StdoutAuditLogger {
    fn log(&self, record: &AuditRecord) -> Result<(), SecurityError> {
        println!("{}", stdout_audit_line(record));
        Ok(())
    }
}

fn stdout_audit_line(record: &AuditRecord) -> String {
    format!(
        "[AUDIT LOG #{}] actor={:?} action={:?} resource={:?} hash={}",
        record.sequence_id, record.actor, record.action, record.resource, record.hash
    )
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

    /// Continues a persisted chain after its newest durable record.
    ///
    /// Every constructor otherwise starts at sequence 1 from the genesis
    /// predecessor, so a restarted writer would begin a second chain that
    /// [`AuditChain::verify_sequence`] rejects over the retained trail. Load
    /// `tip`, the record with the highest `sequence_id`, from the persisted
    /// trail on startup: its HMAC must verify with `secret_key`, and the next
    /// record continues at `tip.sequence_id + 1` with `tip.hash` as its
    /// predecessor.
    ///
    /// One writer must own a persisted chain. Two processes that resume from
    /// the same tip fork it, and verification of the merged trail fails. The
    /// tip is only as trustworthy as the store it came from: deleting the
    /// newest records before a restart is not detected without an external
    /// checkpoint of the last sequence and hash. Unpublished v13 API.
    pub fn try_resume(
        secret_key: &[u8],
        logger: Arc<dyn AuditLogger>,
        tip: &AuditRecord,
    ) -> Result<Self, SecurityError> {
        validate_secret_key(secret_key)?;
        if tip.sequence_id == 0 || !Self::verify_record(secret_key, tip) {
            return Err(SecurityError::AuditChainError(
                "audit chain tip does not verify with this key".to_string(),
            ));
        }
        let chain = Self::new_inner(secret_key, logger);
        Ok(Self {
            state: Arc::new(Mutex::new(AuditState {
                last_hash: tip.hash.clone(),
                sequence: tip.sequence_id,
            })),
            ..chain
        })
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
    fn stdout_lines_escape_user_controlled_fields() {
        let record = AuditRecord {
            sequence_id: 7,
            timestamp: 1,
            actor: "bob\n[AUDIT LOG #1] actor=admin action=grant_role".to_string(),
            action: "rename\r\"x\"".to_string(),
            resource: "user:42 hash=00".to_string(),
            payload: "{}".to_string(),
            previous_hash: GENESIS_HASH.to_string(),
            hash: "abc".to_string(),
        };
        let line = stdout_audit_line(&record);
        assert!(!line.contains('\n') && !line.contains('\r'));
        assert_eq!(
            line,
            r#"[AUDIT LOG #7] actor="bob\n[AUDIT LOG #1] actor=admin action=grant_role" action="rename\r\"x\"" resource="user:42 hash=00" hash=abc"#
        );
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

    #[tokio::test]
    async fn a_resumed_chain_continues_the_persisted_trail() {
        let secret = b"audit-key-material-with-32-plus-bytes";
        let logger: Arc<dyn AuditLogger> = Arc::new(StdoutAuditLogger);
        let first_run = AuditChain::try_new(secret, logger.clone()).expect("strong audit key");
        let mut trail = Vec::new();
        for action in ["create", "publish"] {
            trail.push(
                first_run
                    .record_event("actor", action, "course:1", "{}")
                    .await
                    .expect("durable record"),
            );
        }

        // A restarted writer resumes from the newest persisted record.
        let tip = trail.last().expect("tip").clone();
        let resumed = AuditChain::try_resume(secret, logger.clone(), &tip).expect("valid tip");
        let next = resumed
            .record_event("actor", "archive", "course:1", "{}")
            .await
            .expect("durable record");
        assert_eq!(next.sequence_id, 3);
        assert_eq!(next.previous_hash, tip.hash);
        trail.push(next);
        assert!(AuditChain::verify_sequence(secret, &trail));

        // A fresh chain after a restart restarts at genesis and breaks the trail.
        let restarted = AuditChain::try_new(secret, logger.clone()).expect("strong audit key");
        let mut broken = trail.clone();
        broken.push(
            restarted
                .record_event("actor", "archive", "course:1", "{}")
                .await
                .expect("durable record"),
        );
        assert!(!AuditChain::verify_sequence(secret, &broken));

        let mut forged = tip.clone();
        forged.sequence_id = 99;
        assert!(AuditChain::try_resume(secret, logger.clone(), &forged).is_err());
        assert!(
            AuditChain::try_resume(
                b"another-audit-key-with-32-plus-bytes",
                logger.clone(),
                &tip
            )
            .is_err()
        );
        assert!(AuditChain::try_resume(b"weak", logger, &tip).is_err());
    }

    #[tokio::test]
    async fn sequence_verification_rejects_weak_keys_reordering_and_tampering() {
        let secret = b"audit-key-material-with-32-plus-bytes";
        let chain =
            AuditChain::try_new(secret, Arc::new(StdoutAuditLogger)).expect("strong audit key");
        let first = chain
            .record_event("actor", "create", "course:1", "{}")
            .await
            .expect("first record should be durable");
        let second = chain
            .record_event("actor", "publish", "course:1", "{}")
            .await
            .expect("second record should be durable");
        let records = vec![first.clone(), second.clone()];

        assert!(AuditChain::verify_sequence(secret, &records));
        assert!(!AuditChain::verify_record(b"weak", &first));
        assert!(!AuditChain::verify_sequence(b"weak", &records));
        assert!(!AuditChain::verify_sequence(
            secret,
            &[second.clone(), first.clone()]
        ));

        let mut broken_predecessor = second.clone();
        broken_predecessor.previous_hash = "forged".to_string();
        assert!(!AuditChain::verify_sequence(
            secret,
            &[first.clone(), broken_predecessor]
        ));

        let mut tampered = first.clone();
        tampered.payload = "{\"role\":\"admin\"}".to_string();
        assert!(!AuditChain::verify_record(secret, &tampered));
        assert!(!AuditChain::verify_sequence(secret, &[tampered]));
    }
}
