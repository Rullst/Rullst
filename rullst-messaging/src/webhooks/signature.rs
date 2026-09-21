use super::*;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use hmac::{Hmac, KeyInit, Mac};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

#[derive(Clone)]
pub struct WebhookSigningKey {
    id: String,
    secret: Arc<Zeroizing<Vec<u8>>>,
    pub(super) offline: bool,
}
impl WebhookSigningKey {
    /// Live keys are unpadded base64url encoding exactly 32 CSPRNG bytes.
    /// Empty/`mock_*` credentials select deterministic offline delivery.
    pub fn new(id: impl Into<String>, encoded: impl Into<String>) -> Result<Self> {
        let id = id.into();
        if !crate::storage_keys::valid_key_id(&id) {
            return Err(WebhookError::InvalidInput("signing key ID"));
        }
        let encoded = Zeroizing::new(encoded.into());
        if encoded.len() > 128 {
            return Err(WebhookError::InvalidInput("signing key"));
        }
        let offline = encoded.is_empty() || encoded.starts_with("mock_");
        let bytes = if offline {
            vec![0x57; 32]
        } else {
            let bytes = Zeroizing::new(
                URL_SAFE_NO_PAD
                    .decode(encoded.as_bytes())
                    .map_err(|_| WebhookError::InvalidInput("signing key"))?,
            );
            if bytes.len() != 32 || URL_SAFE_NO_PAD.encode(&bytes) != encoded.as_str() {
                return Err(WebhookError::InvalidInput("signing key"));
            }
            bytes.to_vec()
        };
        Ok(Self {
            id,
            secret: Arc::new(Zeroizing::new(bytes)),
            offline,
        })
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn is_offline(&self) -> bool {
        self.offline
    }
    pub(super) fn binding(&self) -> Vec<u8> {
        let mut digest = Sha256::new();
        digest.update(b"rullst.webhook.configuration.key.v1");
        digest.update(self.secret.as_slice());
        digest.finalize().to_vec()
    }
    pub(super) fn event_tag(&self, namespace: &str, id: &str) -> Result<String> {
        let mut signer = Hmac::<Sha256>::new_from_slice(self.secret.as_slice())
            .map_err(|_| WebhookError::Configuration)?;
        for value in [
            b"rullst.webhook.event-key.v1".as_slice(),
            namespace.as_bytes(),
            id.as_bytes(),
        ] {
            signer.update(&(value.len() as u64).to_be_bytes());
            signer.update(value);
        }
        Ok(URL_SAFE_NO_PAD.encode(signer.finalize().into_bytes()))
    }
    fn signer(&self, id: &str, kind: &str, timestamp: i64, body: &[u8]) -> Result<Hmac<Sha256>> {
        let mut signer = Hmac::<Sha256>::new_from_slice(self.secret.as_slice())
            .map_err(|_| WebhookError::InvalidSignature)?;
        for value in [
            b"rullst.webhook.signature.v1".as_slice(),
            self.id.as_bytes(),
            id.as_bytes(),
            kind.as_bytes(),
            timestamp.to_string().as_bytes(),
            body,
        ] {
            signer.update(&(value.len() as u64).to_be_bytes());
            signer.update(value);
        }
        Ok(signer)
    }
    /// Authenticates the exact body and metadata; never records replay state.
    /// The receiver must additionally deduplicate the returned stable delivery ID.
    pub fn verify<C: Clock>(
        &self,
        signature: &WebhookSignature,
        body: &[u8],
        clock: &C,
        max_skew: Duration,
    ) -> Result<VerifiedWebhook> {
        let skew = i64::try_from(max_skew.as_secs())
            .map_err(|_| WebhookError::InvalidInput("signature skew"))?;
        if !(1..=300).contains(&skew) || max_skew.subsec_nanos() != 0 {
            return Err(WebhookError::InvalidInput("signature skew"));
        }
        if self.offline || body.len() > MAX_BODY || signature.key_id != self.id {
            return Err(WebhookError::InvalidSignature);
        }
        let current = now(clock)? / 1000;
        if (current - signature.timestamp).abs() > skew {
            return Err(WebhookError::InvalidSignature);
        }
        self.signer(&signature.id, &signature.kind, signature.timestamp, body)?
            .verify_slice(&signature.tag)
            .map_err(|_| WebhookError::InvalidSignature)?;
        Ok(VerifiedWebhook {
            id: signature.id.clone(),
            kind: signature.kind.clone(),
            timestamp: signature.timestamp,
        })
    }
    pub(super) fn sign(
        &self,
        id: &str,
        kind: &str,
        timestamp: i64,
        body: &[u8],
    ) -> Result<WebhookSignature> {
        let bytes = self
            .signer(id, kind, timestamp, body)?
            .finalize()
            .into_bytes();
        Ok(WebhookSignature {
            id: id.to_owned(),
            kind: kind.to_owned(),
            timestamp,
            key_id: self.id.clone(),
            tag: bytes.to_vec(),
        })
    }
}
impl std::fmt::Debug for WebhookSigningKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("WebhookSigningKey([REDACTED])")
    }
}

/// Parsed exact HTTP metadata. Reject duplicate instances of these headers before construction.
#[derive(Clone)]
pub struct WebhookSignature {
    id: String,
    kind: String,
    timestamp: i64,
    key_id: String,
    tag: Vec<u8>,
}
impl WebhookSignature {
    /// Headers: Rullst-Webhook-Id/Type/Timestamp/Key-Id/Signature.
    /// Signature grammar is `v1=` followed by canonical unpadded base64url.
    pub fn from_headers(
        id: impl Into<String>,
        kind: impl Into<String>,
        timestamp: impl Into<String>,
        key_id: impl Into<String>,
        signature: impl Into<String>,
    ) -> Result<Self> {
        let (id, kind, timestamp, key_id, signature) = (
            id.into(),
            kind.into(),
            timestamp.into(),
            key_id.into(),
            signature.into(),
        );
        if id.len() != 36
            || kind.len() > 128
            || timestamp.len() > 12
            || signature.len() != 46
            || !crate::storage_keys::valid_key_id(&key_id)
        {
            return Err(WebhookError::InvalidSignature);
        }
        crate::MessageId::from_stored(id.clone()).map_err(|_| WebhookError::InvalidSignature)?;
        crate::EventKind::try_new(&kind).map_err(|_| WebhookError::InvalidSignature)?;
        let time: i64 = timestamp
            .parse()
            .map_err(|_| WebhookError::InvalidSignature)?;
        if !(0..=MAX_TIMESTAMP / 1000).contains(&time) || time.to_string() != timestamp {
            return Err(WebhookError::InvalidSignature);
        }
        let encoded = signature
            .strip_prefix("v1=")
            .ok_or(WebhookError::InvalidSignature)?;
        let tag = URL_SAFE_NO_PAD
            .decode(encoded)
            .map_err(|_| WebhookError::InvalidSignature)?;
        if tag.len() != 32 || URL_SAFE_NO_PAD.encode(&tag) != encoded {
            return Err(WebhookError::InvalidSignature);
        }
        Ok(Self {
            id,
            kind,
            timestamp: time,
            key_id,
            tag,
        })
    }
    /// Parses HTTP headers, requiring exactly one occurrence of every signature field.
    /// Axum/http 1 header maps share this type; duplicates are always rejected.
    pub fn from_http_headers(headers: &reqwest::header::HeaderMap) -> Result<Self> {
        let [id, kind, timestamp, key_id, signature] = [
            "rullst-webhook-id",
            "rullst-webhook-type",
            "rullst-webhook-timestamp",
            "rullst-webhook-key-id",
            "rullst-webhook-signature",
        ]
        .map(|name| {
            let mut values = headers.get_all(name).iter();
            let value = values.next().ok_or(WebhookError::InvalidSignature)?;
            if values.next().is_some() {
                return Err(WebhookError::InvalidSignature);
            }
            value.to_str().map_err(|_| WebhookError::InvalidSignature)
        });
        Self::from_headers(id?, kind?, timestamp?, key_id?, signature?)
    }
    pub fn delivery_id(&self) -> &str {
        &self.id
    }
    pub fn event_kind(&self) -> &str {
        &self.kind
    }
    pub fn timestamp_seconds(&self) -> i64 {
        self.timestamp
    }
    pub fn key_id(&self) -> &str {
        &self.key_id
    }
    pub fn signature_header(&self) -> String {
        format!("v1={}", URL_SAFE_NO_PAD.encode(&self.tag))
    }
}
impl std::fmt::Debug for WebhookSignature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("WebhookSignature([REDACTED])")
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedWebhook {
    id: String,
    kind: String,
    timestamp: i64,
}
impl VerifiedWebhook {
    pub fn delivery_id(&self) -> &str {
        &self.id
    }
    pub fn event_kind(&self) -> &str {
        &self.kind
    }
    pub fn timestamp_seconds(&self) -> i64 {
        self.timestamp
    }
}
