use crate::error::SecurityError;
use hmac::{Hmac, KeyInit, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::sync::Arc;
use subtle::ConstantTimeEq;
use tokio::sync::Mutex;

type HmacSha256 = Hmac<Sha256>;

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

pub struct AuditChain {
    secret_key: Vec<u8>,
    last_hash: Arc<Mutex<String>>,
    sequence_counter: Arc<Mutex<u64>>,
    logger: Arc<dyn AuditLogger>,
}

impl AuditChain {
    pub fn new(secret_key: &[u8], logger: Arc<dyn AuditLogger>) -> Self {
        Self {
            secret_key: secret_key.to_vec(),
            last_hash: Arc::new(Mutex::new("GENESIS_HASH".to_string())),
            sequence_counter: Arc::new(Mutex::new(0)),
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
        let mut last_hash_guard = self.last_hash.lock().await;
        let mut seq_guard = self.sequence_counter.lock().await;

        *seq_guard += 1;
        let seq = *seq_guard;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let prev_hash = last_hash_guard.clone();

        let data_to_sign = format!(
            "{}:{}:{}:{}:{}:{}:{}",
            seq, now, actor, action, resource, payload, prev_hash
        );

        let mut mac = HmacSha256::new_from_slice(&self.secret_key).map_err(|e| {
            SecurityError::AuditChainError(format!("HMAC Key Initialization Error: {}", e))
        })?;
        mac.update(data_to_sign.as_bytes());
        let result = mac.finalize();
        let current_hash = hex::encode(result.into_bytes());

        let record = AuditRecord {
            sequence_id: seq,
            timestamp: now,
            actor: actor.to_string(),
            action: action.to_string(),
            resource: resource.to_string(),
            payload: payload.to_string(),
            previous_hash: prev_hash,
            hash: current_hash.clone(),
        };

        self.logger.log(&record)?;
        *last_hash_guard = current_hash;

        Ok(record)
    }

    pub fn verify_record(secret_key: &[u8], record: &AuditRecord) -> bool {
        let data_to_sign = format!(
            "{}:{}:{}:{}:{}:{}:{}",
            record.sequence_id,
            record.timestamp,
            record.actor,
            record.action,
            record.resource,
            record.payload,
            record.previous_hash
        );

        let mut mac = match HmacSha256::new_from_slice(secret_key) {
            Ok(m) => m,
            Err(_) => return false,
        };
        mac.update(data_to_sign.as_bytes());
        let expected_hash = hex::encode(mac.finalize().into_bytes());

        expected_hash
            .as_bytes()
            .ct_eq(record.hash.as_bytes())
            .into()
    }
}
