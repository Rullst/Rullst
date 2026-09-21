use super::RecoveryError;
use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit, Payload},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use ring::{
    hmac,
    rand::{SecureRandom, SystemRandom},
};
use zeroize::Zeroizing;

/// Random 256-bit bearer token. Never log, serialize as an event or persist it directly.
pub struct SecretToken(Zeroizing<String>);

impl std::fmt::Debug for SecretToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SecretToken([REDACTED])")
    }
}

impl SecretToken {
    #[cfg(any(
        feature = "email-login-sqlite",
        feature = "email-login-postgres",
        feature = "api-tokens-sqlite",
        feature = "api-tokens-postgres"
    ))]
    pub(super) fn from_encoded(value: &str) -> Result<Self, RecoveryError> {
        if value.len() != 43
            || URL_SAFE_NO_PAD
                .decode(value)
                .map_or(true, |bytes| bytes.len() != 32)
        {
            return Err(RecoveryError::InvalidAction);
        }
        Ok(Self(Zeroizing::new(value.to_owned())))
    }

    pub(super) fn generate() -> Result<Self, RecoveryError> {
        let mut bytes = Zeroizing::new([0u8; 32]);
        SystemRandom::new()
            .fill(bytes.as_mut())
            .map_err(|_| RecoveryError::Crypto)?;
        Ok(Self(Zeroizing::new(URL_SAFE_NO_PAD.encode(bytes.as_ref()))))
    }

    /// Sensitive plaintext for the intended recipient/cookie only.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

/// Separate stable digest and rotating-storage encryption key material.
/// Supply independently generated random 32-byte keys from a secret manager.
/// Key rotation requires an explicit data migration; wrong keys fail closed.
pub struct RecoverySecrets {
    digest: Zeroizing<[u8; 32]>,
    encryption: Zeroizing<[u8; 32]>,
}

impl RecoverySecrets {
    pub fn new(digest: [u8; 32], encryption: [u8; 32]) -> Result<Self, RecoveryError> {
        if digest == [0; 32] || encryption == [0; 32] || digest == encryption {
            return Err(RecoveryError::Configuration);
        }
        Ok(Self {
            digest: Zeroizing::new(digest),
            encryption: Zeroizing::new(encryption),
        })
    }

    pub(super) fn digest(&self, domain: &str, value: &str) -> String {
        let key = hmac::Key::new(hmac::HMAC_SHA256, self.digest.as_ref());
        let mut context = hmac::Context::with_key(&key);
        context.update(b"rullst.auth.recovery.v1\0");
        context.update(domain.as_bytes());
        context.update(b"\0");
        context.update(value.as_bytes());
        URL_SAFE_NO_PAD.encode(context.sign().as_ref())
    }

    pub(super) fn seal(&self, context: &str, value: &[u8]) -> Result<String, RecoveryError> {
        if value.len() > 16384 {
            return Err(RecoveryError::InvalidInput);
        }
        let cipher = Aes256Gcm::new_from_slice(self.encryption.as_ref())
            .map_err(|_| RecoveryError::Crypto)?;
        let mut nonce = [0u8; 12];
        SystemRandom::new()
            .fill(&mut nonce)
            .map_err(|_| RecoveryError::Crypto)?;
        let mut bytes = nonce.to_vec();
        bytes.extend(
            cipher
                .encrypt(
                    &Nonce::from(nonce),
                    Payload {
                        msg: value,
                        aad: context.as_bytes(),
                    },
                )
                .map_err(|_| RecoveryError::Crypto)?,
        );
        Ok(URL_SAFE_NO_PAD.encode(bytes))
    }

    pub(super) fn open(
        &self,
        context: &str,
        value: &str,
    ) -> Result<Zeroizing<Vec<u8>>, RecoveryError> {
        if value.len() > 24000 {
            return Err(RecoveryError::Crypto);
        }
        let bytes = URL_SAFE_NO_PAD
            .decode(value)
            .map_err(|_| RecoveryError::Crypto)?;
        let (nonce, ciphertext) = bytes.split_at_checked(12).ok_or(RecoveryError::Crypto)?;
        let nonce: [u8; 12] = nonce.try_into().map_err(|_| RecoveryError::Crypto)?;
        let cipher = Aes256Gcm::new_from_slice(self.encryption.as_ref())
            .map_err(|_| RecoveryError::Crypto)?;
        cipher
            .decrypt(
                &Nonce::from(nonce),
                Payload {
                    msg: ciphertext,
                    aad: context.as_bytes(),
                },
            )
            .map(Zeroizing::new)
            .map_err(|_| RecoveryError::Crypto)
    }
}

/// Lifecycle notices supported by the atomic recovery outbox.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub enum RecoveryNoticeKind {
    Welcome,
    PasswordReset,
    PasswordChanged,
}

/// Decrypted only by an authorized outbox worker; its Debug omits personal data.
pub struct RecoveryNotice {
    pub(super) recipient: Zeroizing<String>,
    pub(super) token: Option<SecretToken>,
    pub(super) kind: RecoveryNoticeKind,
    pub(super) expires_at: i64,
    pub(super) locale: Option<RecoveryLocale>,
}

impl std::fmt::Debug for RecoveryNotice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RecoveryNotice")
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct NoticeWire {
    recipient: String,
    token: Option<String>,
    kind: RecoveryNoticeKind,
    expires_at: i64,
    #[serde(default)]
    locale: Option<RecoveryLocale>,
}

impl RecoveryNotice {
    /// Recorded user preference; absence selects the application fallback.
    pub fn locale(&self) -> Option<RecoveryLocale> {
        self.locale
    }
    pub fn recipient(&self) -> &str {
        &self.recipient
    }
    pub fn token(&self) -> Option<&SecretToken> {
        self.token.as_ref()
    }
    pub fn kind(&self) -> RecoveryNoticeKind {
        self.kind
    }
    pub fn expires_at(&self) -> i64 {
        self.expires_at
    }

    pub(super) fn seal(&self, keys: &RecoverySecrets, id: &str) -> Result<String, RecoveryError> {
        let mut wire = NoticeWire {
            recipient: self.recipient.to_string(),
            token: self.token.as_ref().map(|token| token.expose().to_owned()),
            kind: self.kind,
            expires_at: self.expires_at,
            locale: self.locale,
        };
        let result = serde_json::to_vec(&wire)
            .map(Zeroizing::new)
            .map_err(|_| RecoveryError::Crypto);
        use zeroize::Zeroize;
        wire.recipient.zeroize();
        if let Some(token) = &mut wire.token {
            token.zeroize();
        }
        keys.seal(&format!("rullst.auth.notice.v1:{id}"), &result?)
    }

    pub(super) fn open(
        keys: &RecoverySecrets,
        id: &str,
        ciphertext: &str,
    ) -> Result<Self, RecoveryError> {
        let plaintext = keys.open(&format!("rullst.auth.notice.v1:{id}"), ciphertext)?;
        let wire: NoticeWire =
            serde_json::from_slice(&plaintext).map_err(|_| RecoveryError::Crypto)?;
        Ok(Self {
            recipient: Zeroizing::new(wire.recipient),
            token: wire.token.map(|token| SecretToken(Zeroizing::new(token))),
            kind: wire.kind,
            expires_at: wire.expires_at,
            locale: wire.locale,
        })
    }
}

/// Explicitly recorded account language, never inferred from an email address.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub enum RecoveryLocale {
    En,
    PtBr,
    Es,
}
impl RecoveryLocale {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::PtBr => "pt-BR",
            Self::Es => "es",
        }
    }
    pub(super) fn parse(value: &str) -> Result<Option<Self>, RecoveryError> {
        match value {
            "" => Ok(None),
            "en" => Ok(Some(Self::En)),
            "pt-BR" => Ok(Some(Self::PtBr)),
            "es" => Ok(Some(Self::Es)),
            _ => Err(RecoveryError::Storage),
        }
    }
}
