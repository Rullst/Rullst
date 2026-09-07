//! Durable shared SQLite lifecycle for validated passkey credentials.

use super::{Passkey, PasskeyAuth, PublicKeyCredential};
use crate::AuthError;
use base64::Engine as _;
mod storage;
use sqlx::{Sqlite, SqliteConnection, SqlitePool, Transaction};
use storage::*;
use subtle::ConstantTimeEq as _;

const MAX_TOTAL_CREDENTIALS: usize = 1_000_000;
const MAX_CREDENTIALS_PER_SUBJECT: usize = 128;
const MAX_CREDENTIAL_ID_BYTES: usize = 1_023;
const MAX_SUBJECT_BYTES: usize = 256;
const MAX_LABEL_BYTES: usize = 128;

/// Typed durable passkey lifecycle failures. Credential, subject and path data
/// are deliberately omitted from formatted errors.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum PasskeyStoreError {
    #[error("invalid passkey store configuration: {0}")]
    InvalidConfiguration(&'static str),
    #[error("passkey credential is malformed")]
    InvalidCredential,
    #[error("passkey credential already exists")]
    AlreadyExists,
    #[error("passkey credential was not found")]
    NotFound,
    #[error("passkey credential is revoked")]
    Revoked,
    #[error("passkey signature counter changed concurrently")]
    CounterConflict,
    #[error("passkey credential capacity is exhausted")]
    CapacityExceeded,
    #[error("passkey ceremony was rejected")]
    CeremonyRejected,
    #[error("passkey storage operation failed: {0}")]
    StorageUnavailable(&'static str),
    #[error("passkey storage is corrupt: {0}")]
    CorruptStorage(&'static str),
}

/// Secret-minimized metadata for one registered passkey device.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PasskeyDeviceSummary {
    credential_id: Vec<u8>,
    label: String,
    sign_count: u32,
    created_at: u64,
    last_used_at: Option<u64>,
    revoked_at: Option<u64>,
}

impl PasskeyDeviceSummary {
    #[must_use]
    pub fn credential_id(&self) -> &[u8] {
        &self.credential_id
    }

    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    #[must_use]
    pub const fn sign_count(&self) -> u32 {
        self.sign_count
    }

    #[must_use]
    pub const fn created_at(&self) -> u64 {
        self.created_at
    }

    #[must_use]
    pub const fn last_used_at(&self) -> Option<u64> {
        self.last_used_at
    }

    #[must_use]
    pub const fn revoked_at(&self) -> Option<u64> {
        self.revoked_at
    }

    #[must_use]
    pub const fn is_revoked(&self) -> bool {
        self.revoked_at.is_some()
    }
}

/// File-backed passkey device registry with transactional quotas and counter CAS.
///
/// The store is shareable by processes on one SQLite file. It persists public
/// credentials, device labels, monotonic counters, usage timestamps and
/// revocation state. WebAuthn challenges remain inside [`PasskeyAuth`], so a
/// multi-instance host still needs sticky routing or its own shared ceremony
/// store. The host owns file permissions, encryption, backup and replication.
#[derive(Clone)]
pub struct SqlitePasskeyStore {
    pool: SqlitePool,
    max_total_credentials: usize,
    max_credentials_per_subject: usize,
}

impl SqlitePasskeyStore {
    /// Opens or creates a file-backed device registry with persisted quotas.
    pub async fn connect(
        database_url: impl Into<String>,
        max_total_credentials: usize,
        max_credentials_per_subject: usize,
    ) -> Result<Self, PasskeyStoreError> {
        validate_limits(max_total_credentials, max_credentials_per_subject)?;
        let database_url = database_url.into();
        let pool = connect_pool(&database_url).await?;
        if let Err(error) =
            prepare_schema(&pool, max_total_credentials, max_credentials_per_subject).await
        {
            pool.close().await;
            return Err(error);
        }
        Ok(Self {
            pool,
            max_total_credentials,
            max_credentials_per_subject,
        })
    }

    /// Registers one already validated credential for an authoritative subject.
    pub async fn register(
        &self,
        subject: impl Into<String>,
        label: impl Into<String>,
        passkey: Passkey,
    ) -> Result<PasskeyDeviceSummary, PasskeyStoreError> {
        let subject = subject.into();
        let label = label.into();
        validate_subject(&subject)?;
        validate_label(&label)?;
        validate_passkey(&passkey)?;
        let now = unix_time()?;
        let mut connection = self.begin_write("begin registration").await?;
        let result = self
            .register_in_transaction(&mut connection, &subject, &label, passkey, now)
            .await;
        finish(connection, result, "finish registration").await
    }

    /// Returns bounded device metadata, including revoked entries, in creation order.
    pub async fn devices(
        &self,
        subject: &str,
    ) -> Result<Vec<PasskeyDeviceSummary>, PasskeyStoreError> {
        validate_subject(subject)?;
        let rows: Vec<DeviceRow> = sqlx::query_as("SELECT credential_id, label, sign_count, created_at, last_used_at, revoked_at FROM rullst_auth_passkey_devices WHERE subject = ? ORDER BY created_at, credential_id LIMIT ?")
            .bind(subject)
            .bind(as_i64(self.max_credentials_per_subject, "per-subject limit")?)
            .fetch_all(&self.pool)
            .await
            .map_err(|_| unavailable("list devices"))?;
        rows.into_iter().map(decode_summary).collect()
    }

    /// Loads active public credentials suitable for `start_authenticate`.
    pub async fn active_passkeys(&self, subject: &str) -> Result<Vec<Passkey>, PasskeyStoreError> {
        validate_subject(subject)?;
        let rows: Vec<(Vec<u8>, Vec<u8>, i64)> = sqlx::query_as("SELECT credential_id, public_key, sign_count FROM rullst_auth_passkey_devices WHERE subject = ? AND revoked_at IS NULL ORDER BY created_at, credential_id LIMIT ?")
            .bind(subject)
            .bind(as_i64(self.max_credentials_per_subject, "per-subject limit")?)
            .fetch_all(&self.pool)
            .await
            .map_err(|_| unavailable("load active credentials"))?;
        rows.into_iter()
            .map(|(credential_id, public_key, sign_count)| {
                decode_passkey(credential_id, public_key, sign_count)
            })
            .collect()
    }

    /// Verifies one assertion and atomically advances its stored counter.
    pub async fn finish_authenticate(
        &self,
        auth: &PasskeyAuth,
        subject: &str,
        credential: &PublicKeyCredential,
        expected_challenge: &str,
    ) -> Result<Passkey, PasskeyStoreError> {
        validate_subject(subject)?;
        let credential_id = decode_credential_id(credential)?;
        let previous = self.load_active(subject, &credential_id).await?;
        let updated = auth
            .finish_authenticate(credential, expected_challenge, previous.clone())
            .map_err(|_: AuthError| PasskeyStoreError::CeremonyRejected)?;
        self.advance_counter(subject, &previous, &updated).await?;
        Ok(updated)
    }

    /// Changes a device label without altering credential material.
    pub async fn rename(
        &self,
        subject: &str,
        credential_id: &[u8],
        label: impl Into<String>,
    ) -> Result<(), PasskeyStoreError> {
        validate_subject(subject)?;
        validate_credential_id(credential_id)?;
        let label = label.into();
        validate_label(&label)?;
        let changed = sqlx::query("UPDATE rullst_auth_passkey_devices SET label = ? WHERE subject = ? AND credential_id = ? AND revoked_at IS NULL")
            .bind(label)
            .bind(subject)
            .bind(credential_id)
            .execute(&self.pool)
            .await
            .map_err(|_| unavailable("rename device"))?;
        if changed.rows_affected() != 1 {
            return self.classify_missing(subject, credential_id).await;
        }
        Ok(())
    }

    /// Idempotently marks a credential revoked; revoked credentials cannot authenticate.
    pub async fn revoke(
        &self,
        subject: &str,
        credential_id: &[u8],
    ) -> Result<(), PasskeyStoreError> {
        validate_subject(subject)?;
        validate_credential_id(credential_id)?;
        let now = i64::try_from(unix_time()?).map_err(|_| corrupt("revocation time"))?;
        let changed = sqlx::query("UPDATE rullst_auth_passkey_devices SET revoked_at = COALESCE(revoked_at, ?) WHERE subject = ? AND credential_id = ?")
            .bind(now)
            .bind(subject)
            .bind(credential_id)
            .execute(&self.pool)
            .await
            .map_err(|_| unavailable("revoke device"))?;
        if changed.rows_affected() != 1 {
            return Err(PasskeyStoreError::NotFound);
        }
        Ok(())
    }

    /// Gracefully closes all pooled connections, useful before rotating a file.
    pub async fn close(self) {
        self.pool.close().await;
    }

    async fn register_in_transaction(
        &self,
        connection: &mut SqliteConnection,
        subject: &str,
        label: &str,
        passkey: Passkey,
        now: u64,
    ) -> Result<PasskeyDeviceSummary, PasskeyStoreError> {
        let duplicate: Option<(i64,)> =
            sqlx::query_as("SELECT 1 FROM rullst_auth_passkey_devices WHERE credential_id = ?")
                .bind(&passkey.credential_id)
                .fetch_optional(&mut *connection)
                .await
                .map_err(|_| unavailable("lookup credential"))?;
        if duplicate.is_some() {
            return Err(PasskeyStoreError::AlreadyExists);
        }
        let counts: (i64, i64) = sqlx::query_as("SELECT (SELECT COUNT(*) FROM rullst_auth_passkey_devices), (SELECT COUNT(*) FROM rullst_auth_passkey_devices WHERE subject = ?)")
            .bind(subject)
            .fetch_one(&mut *connection)
            .await
            .map_err(|_| unavailable("count credentials"))?;
        let total = usize::try_from(counts.0).map_err(|_| corrupt("total credential count"))?;
        let per_subject =
            usize::try_from(counts.1).map_err(|_| corrupt("subject credential count"))?;
        if total >= self.max_total_credentials || per_subject >= self.max_credentials_per_subject {
            return Err(PasskeyStoreError::CapacityExceeded);
        }
        let sign_count = i64::from(passkey.sign_count);
        let created_at = i64::try_from(now).map_err(|_| corrupt("creation time"))?;
        sqlx::query("INSERT INTO rullst_auth_passkey_devices (credential_id, subject, label, public_key, sign_count, created_at, last_used_at, revoked_at) VALUES (?, ?, ?, ?, ?, ?, NULL, NULL)")
            .bind(&passkey.credential_id)
            .bind(subject)
            .bind(label)
            .bind(&passkey.public_key)
            .bind(sign_count)
            .bind(created_at)
            .execute(&mut *connection)
            .await
            .map_err(|_| unavailable("persist credential"))?;
        Ok(PasskeyDeviceSummary {
            credential_id: passkey.credential_id,
            label: label.to_string(),
            sign_count: passkey.sign_count,
            created_at: now,
            last_used_at: None,
            revoked_at: None,
        })
    }

    async fn load_active(
        &self,
        subject: &str,
        credential_id: &[u8],
    ) -> Result<Passkey, PasskeyStoreError> {
        let row: Option<(Vec<u8>, i64, Option<i64>)> = sqlx::query_as("SELECT public_key, sign_count, revoked_at FROM rullst_auth_passkey_devices WHERE subject = ? AND credential_id = ?")
            .bind(subject)
            .bind(credential_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|_| unavailable("load credential"))?;
        match row {
            Some((_, _, Some(_))) => Err(PasskeyStoreError::Revoked),
            Some((public_key, sign_count, None)) => {
                decode_passkey(credential_id.to_vec(), public_key, sign_count)
            }
            None => Err(PasskeyStoreError::NotFound),
        }
    }

    async fn advance_counter(
        &self,
        subject: &str,
        previous: &Passkey,
        updated: &Passkey,
    ) -> Result<(), PasskeyStoreError> {
        if previous.credential_id != updated.credential_id
            || previous.public_key != updated.public_key
            || ((previous.sign_count != 0 || updated.sign_count != 0)
                && updated.sign_count <= previous.sign_count)
        {
            return Err(PasskeyStoreError::InvalidCredential);
        }
        let now = i64::try_from(unix_time()?).map_err(|_| corrupt("usage time"))?;
        let changed = sqlx::query("UPDATE rullst_auth_passkey_devices SET sign_count = ?, last_used_at = ? WHERE subject = ? AND credential_id = ? AND public_key = ? AND sign_count = ? AND revoked_at IS NULL")
            .bind(i64::from(updated.sign_count))
            .bind(now)
            .bind(subject)
            .bind(&previous.credential_id)
            .bind(&previous.public_key)
            .bind(i64::from(previous.sign_count))
            .execute(&self.pool)
            .await
            .map_err(|_| unavailable("advance signature counter"))?;
        if changed.rows_affected() != 1 {
            return Err(PasskeyStoreError::CounterConflict);
        }
        Ok(())
    }

    async fn classify_missing(
        &self,
        subject: &str,
        credential_id: &[u8],
    ) -> Result<(), PasskeyStoreError> {
        let row: Option<(Option<i64>,)> = sqlx::query_as("SELECT revoked_at FROM rullst_auth_passkey_devices WHERE subject = ? AND credential_id = ?")
            .bind(subject)
            .bind(credential_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|_| unavailable("classify credential"))?;
        match row {
            Some((Some(_),)) => Err(PasskeyStoreError::Revoked),
            _ => Err(PasskeyStoreError::NotFound),
        }
    }

    async fn begin_write(
        &self,
        operation: &'static str,
    ) -> Result<Transaction<'static, Sqlite>, PasskeyStoreError> {
        self.pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|_| unavailable(operation))
    }
}

type DeviceRow = (Vec<u8>, String, i64, i64, Option<i64>, Option<i64>);

fn decode_summary(row: DeviceRow) -> Result<PasskeyDeviceSummary, PasskeyStoreError> {
    validate_credential_id(&row.0)?;
    validate_label(&row.1)?;
    let sign_count = u32::try_from(row.2).map_err(|_| corrupt("signature counter"))?;
    let created_at = u64::try_from(row.3).map_err(|_| corrupt("creation time"))?;
    let last_used_at = optional_u64(row.4, "last-used time")?;
    let revoked_at = optional_u64(row.5, "revocation time")?;
    if created_at == 0
        || last_used_at.is_some_and(|value| value < created_at)
        || revoked_at.is_some_and(|value| value < created_at)
    {
        return Err(corrupt("device timestamps"));
    }
    Ok(PasskeyDeviceSummary {
        credential_id: row.0,
        label: row.1,
        sign_count,
        created_at,
        last_used_at,
        revoked_at,
    })
}

fn decode_passkey(
    credential_id: Vec<u8>,
    public_key: Vec<u8>,
    sign_count: i64,
) -> Result<Passkey, PasskeyStoreError> {
    let sign_count = u32::try_from(sign_count).map_err(|_| corrupt("signature counter"))?;
    let passkey = Passkey {
        credential_id,
        public_key,
        sign_count,
    };
    validate_passkey(&passkey)?;
    Ok(passkey)
}

fn decode_credential_id(credential: &PublicKeyCredential) -> Result<Vec<u8>, PasskeyStoreError> {
    if credential.r#type != "public-key" {
        return Err(PasskeyStoreError::InvalidCredential);
    }
    let raw = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(&credential.raw_id)
        .map_err(|_| PasskeyStoreError::InvalidCredential)?;
    let id = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(&credential.id)
        .map_err(|_| PasskeyStoreError::InvalidCredential)?;
    validate_credential_id(&raw)?;
    if raw.ct_eq(&id).unwrap_u8() != 1 {
        return Err(PasskeyStoreError::InvalidCredential);
    }
    Ok(raw)
}

fn validate_limits(total: usize, per_subject: usize) -> Result<(), PasskeyStoreError> {
    if !(1..=MAX_TOTAL_CREDENTIALS).contains(&total)
        || !(1..=MAX_CREDENTIALS_PER_SUBJECT).contains(&per_subject)
        || per_subject > total
    {
        return Err(PasskeyStoreError::InvalidConfiguration("credential limits"));
    }
    Ok(())
}

fn validate_subject(subject: &str) -> Result<(), PasskeyStoreError> {
    if subject.is_empty()
        || subject.trim() != subject
        || subject.len() > MAX_SUBJECT_BYTES
        || subject.chars().any(char::is_control)
    {
        return Err(PasskeyStoreError::InvalidConfiguration("subject"));
    }
    Ok(())
}

fn validate_label(label: &str) -> Result<(), PasskeyStoreError> {
    if label.is_empty()
        || label.trim() != label
        || label.len() > MAX_LABEL_BYTES
        || label.chars().any(char::is_control)
    {
        return Err(PasskeyStoreError::InvalidConfiguration("device label"));
    }
    Ok(())
}

fn validate_passkey(passkey: &Passkey) -> Result<(), PasskeyStoreError> {
    validate_credential_id(&passkey.credential_id)?;
    if passkey.public_key.len() != 65 || passkey.public_key.first() != Some(&0x04) {
        return Err(PasskeyStoreError::InvalidCredential);
    }
    p256::PublicKey::from_sec1_bytes(&passkey.public_key)
        .map_err(|_| PasskeyStoreError::InvalidCredential)?;
    Ok(())
}

fn validate_credential_id(credential_id: &[u8]) -> Result<(), PasskeyStoreError> {
    if credential_id.is_empty() || credential_id.len() > MAX_CREDENTIAL_ID_BYTES {
        return Err(PasskeyStoreError::InvalidCredential);
    }
    Ok(())
}

fn optional_u64(
    value: Option<i64>,
    context: &'static str,
) -> Result<Option<u64>, PasskeyStoreError> {
    value
        .map(|value| u64::try_from(value).map_err(|_| corrupt(context)))
        .transpose()
}

#[cfg(test)]
mod tests;
