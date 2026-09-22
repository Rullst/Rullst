use super::*;
use hmac::{Hmac, KeyInit, Mac};
use secrecy::{ExposeSecret, SecretBox};
use sha2::Sha256;
use std::sync::Arc;

/// Deployment-owned random key for pseudonymous state; never provider credentials.
#[derive(Clone)]
pub struct SuppressionKey(Arc<SecretBox<[u8; 32]>>);
impl SuppressionKey {
    pub fn new(key: [u8; 32]) -> Result<Self, SuppressionError> {
        if key
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            < 8
        {
            return Err(SuppressionError::InvalidConfiguration("HMAC key"));
        }
        Ok(Self(Arc::new(SecretBox::new(Box::new(key)))))
    }
}
impl std::fmt::Debug for SuppressionKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SuppressionKey([REDACTED])")
    }
}

/// Immutable application namespace and independent recipient/replay quotas.
#[derive(Clone)]
pub struct PostgresSuppressionConfig {
    pub(super) namespace: String,
    pub(super) max_recipients: usize,
    pub(super) max_events: usize,
}
impl PostgresSuppressionConfig {
    pub fn new(
        namespace: impl Into<String>,
        max_recipients: usize,
        max_events: usize,
    ) -> Result<Self, SuppressionError> {
        let namespace = namespace.into();
        if namespace.is_empty()
            || namespace.len() > 128
            || !namespace
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
        {
            return Err(SuppressionError::InvalidConfiguration("namespace"));
        }
        validate_limits(max_recipients, max_events)?;
        Ok(Self {
            namespace,
            max_recipients,
            max_events,
        })
    }
}
impl std::fmt::Debug for PostgresSuppressionConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PostgresSuppressionConfig")
            .field("max_recipients", &self.max_recipients)
            .field("max_events", &self.max_events)
            .finish_non_exhaustive()
    }
}

impl PostgresSuppressionStore {
    fn signer(&self, domain: &[u8], values: &[&[u8]]) -> Result<Hmac<Sha256>, SuppressionError> {
        let mut mac = Hmac::<Sha256>::new_from_slice(self.key.0.expose_secret())
            .map_err(|_| SuppressionError::InvalidConfiguration("HMAC key"))?;
        for value in [
            b"rullst-mail-suppression-v1".as_slice(),
            domain,
            self.config.namespace.as_bytes(),
        ]
        .into_iter()
        .chain(values.iter().copied())
        {
            mac.update(&(value.len() as u64).to_be_bytes());
            mac.update(value);
        }
        Ok(mac)
    }
    pub(super) fn tag(&self, domain: &[u8], values: &[&[u8]]) -> Result<Vec<u8>, SuppressionError> {
        Ok(self
            .signer(domain, values)?
            .finalize()
            .into_bytes()
            .to_vec())
    }
    pub(super) fn matches(
        &self,
        domain: &[u8],
        values: &[&[u8]],
        tag: &[u8],
    ) -> Result<bool, SuppressionError> {
        Ok(self.signer(domain, values)?.verify_slice(tag).is_ok())
    }
    pub(super) fn binding(&self) -> Result<Vec<u8>, SuppressionError> {
        self.tag(
            b"configuration",
            &[
                &(self.config.max_recipients as u64).to_be_bytes(),
                &(self.config.max_events as u64).to_be_bytes(),
            ],
        )
    }
}
