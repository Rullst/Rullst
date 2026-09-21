use super::*;
use sqlx::{Any, Transaction};

impl ApiTokenService {
    /// Issues only the host-approved subset of the immutable scope allowlist.
    /// The host must intersect live account permissions and enforce tenant/MFA
    /// policy before this call. The random bearer is returned once, never stored.
    pub async fn issue(
        &self,
        account: &AuthenticatedRecoveryAccount,
        scopes: ApiScopes,
        label: SessionLabel,
        lifetime_seconds: u32,
        clock: &impl AuthClock,
    ) -> Result<IssuedApiToken, RecoveryError> {
        bounded(async {
            self.lifetime(lifetime_seconds)?;
            if !self.config.scopes.includes(&scopes) {
                return Err(RecoveryError::InvalidInput);
            }
            let (mut tx, current) = self.begin(clock).await?;
            self.owner(&mut tx, account).await?;
            self.purge(&mut tx, current).await?;
            let global: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM rullst_api_tokens WHERE namespace = $1")
                    .bind(&self.config.namespace)
                    .fetch_one(&mut *tx)
                    .await?;
            let owned: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM rullst_api_tokens WHERE namespace = $1
                 AND subject = $2 AND account_epoch = $3",
            )
            .bind(&self.config.namespace)
            .bind(&account.subject)
            .bind(account.version)
            .fetch_one(&mut *tx)
            .await?;
            if global >= self.config.capacity as i64 || owned >= 20 {
                return Err(RecoveryError::Limited);
            }
            let id = ApiTokenId::new(SecretToken::generate()?.expose())?;
            let bearer = types::bearer(&id)?;
            let metadata = ApiTokenMetadata {
                id,
                label,
                scopes,
                revision: 1,
                created: current,
                issued: current,
                expires: current + i64::from(lifetime_seconds),
            };
            sqlx::query(
                "INSERT INTO rullst_api_tokens
                 (namespace,id,subject,token_digest,account_epoch,scopes,label,revision,
                  created_at,issued_at,expires_at)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,1,$8,$8,$9)",
            )
            .bind(&self.config.namespace)
            .bind(metadata.id.as_str())
            .bind(&account.subject)
            .bind(self.digest(&bearer))
            .bind(account.version)
            .bind(metadata.scopes.encode())
            .bind(metadata.label.as_str())
            .bind(current)
            .bind(metadata.expires)
            .execute(&mut *tx)
            .await?;
            let finished = self
                .finish(&mut tx, clock, current, metadata.expires)
                .await?;
            tx.commit().await?;
            self.after_commit(clock, finished, metadata.expires)?;
            Ok(IssuedApiToken { bearer, metadata })
        })
        .await
    }

    /// Renews an active credential with a new secret and the same ID/scopes.
    /// Exact revision CAS prevents stale concurrent rotation. A revoked/expired
    /// credential cannot be rotated; issue a separately approved new token.
    pub async fn rotate(
        &self,
        account: &AuthenticatedRecoveryAccount,
        id: &ApiTokenId,
        expected_revision: u64,
        lifetime_seconds: u32,
        clock: &impl AuthClock,
    ) -> Result<IssuedApiToken, RecoveryError> {
        bounded(async {
            self.lifetime(lifetime_seconds)?;
            let expected = i64::try_from(expected_revision)
                .ok()
                .filter(|value| *value > 0)
                .ok_or(RecoveryError::InvalidInput)?;

            let (mut tx, current) = self.begin(clock).await?;
            self.owner(&mut tx, account).await?;
            let mut metadata = self.owned(&mut tx, account, id, current).await?;
            if metadata.revision != expected {
                return Err(RecoveryError::InvalidAction);
            }
            metadata.revision = metadata
                .revision
                .checked_add(1)
                .ok_or(RecoveryError::Limited)?;
            let old_expiry = metadata.expires;
            metadata.issued = current;
            metadata.expires = current + i64::from(lifetime_seconds);
            let bearer = types::bearer(&metadata.id)?;
            let changed = sqlx::query(
                "UPDATE rullst_api_tokens SET token_digest = $1,revision = $2,issued_at = \
                $3,expires_at = $4 WHERE namespace = $5 AND id = $6 AND subject = $7 AND \
                account_epoch = $8 AND revision = $9",
            )
            .bind(self.digest(&bearer))
            .bind(metadata.revision)
            .bind(current)
            .bind(metadata.expires)
            .bind(&self.config.namespace)
            .bind(id.as_str())
            .bind(&account.subject)
            .bind(account.version)
            .bind(expected)
            .execute(&mut *tx)
            .await?
            .rows_affected();

            if changed != 1 {
                return Err(RecoveryError::InvalidAction);
            }
            let finished = self
                .finish(&mut tx, clock, current, old_expiry.min(metadata.expires))
                .await?;
            tx.commit().await?;

            self.after_commit(clock, finished, old_expiry.min(metadata.expires))?;
            Ok(IssuedApiToken { bearer, metadata })
        })
        .await
    }

    /// Lists only the authenticated account's at-most-20 active credentials.
    /// Metadata contains no bearer or digest; use no-store HTTP responses.
    pub async fn inventory(
        &self,
        account: &AuthenticatedRecoveryAccount,
        clock: &impl AuthClock,
    ) -> Result<Vec<ApiTokenMetadata>, RecoveryError> {
        bounded(async {
            let (mut tx, current) = self.begin(clock).await?;
            self.owner(&mut tx, account).await?;
            let rows = sqlx::query(
                "SELECT id,scopes,label,revision,created_at,issued_at,expires_at FROM \
                rullst_api_tokens WHERE namespace = $1 AND subject = $2 AND account_epoch = $3 AND \
                expires_at > $4 ORDER BY created_at,id LIMIT 21",
            )
            .bind(&self.config.namespace)
            .bind(&account.subject)
            .bind(account.version)
            .bind(current)
            .fetch_all(&mut *tx)
            .await?;

            if rows.len() > 20 {
                return Err(RecoveryError::Configuration);
            }
            let tokens = rows
                .iter()
                .map(|row| self.row(row))
                .collect::<Result<Vec<_>, _>>()?;
            if tokens.iter().any(|token| token.issued > current) {
                return Err(RecoveryError::Configuration);
            }
            let finished = self.finish(&mut tx, clock, current, i64::MAX).await?;
            tx.commit().await?;
            let after = self.after_commit(clock, finished, i64::MAX)?;
            Ok(tokens
                .into_iter()
                .filter(|token| token.expires > after)
                .collect())
        })
        .await
    }

    /// Idempotent owner-scoped revocation. Returns true only when this account's
    /// token was removed; absent and foreign-account IDs return false without
    /// revealing ownership. Neither can affect another account's credential.
    pub async fn revoke(
        &self,
        account: &AuthenticatedRecoveryAccount,
        id: &ApiTokenId,
        clock: &impl AuthClock,
    ) -> Result<bool, RecoveryError> {
        bounded(async {
            let (mut tx, current) = self.begin(clock).await?;
            self.owner(&mut tx, account).await?;
            let changed = sqlx::query(
                "DELETE FROM rullst_api_tokens WHERE namespace = $1 AND subject = $2 AND id = $3",
            )
            .bind(&self.config.namespace)
            .bind(&account.subject)
            .bind(id.as_str())
            .execute(&mut *tx)
            .await?
            .rows_affected();
            self.finish(&mut tx, clock, current, i64::MAX).await?;
            tx.commit().await?;
            Ok(changed == 1)
        })
        .await
    }

    /// Revokes all API credentials owned by this account in this namespace.
    /// Browser sessions and other namespaces are not modified by this operation.
    pub async fn revoke_all(
        &self,
        account: &AuthenticatedRecoveryAccount,
        clock: &impl AuthClock,
    ) -> Result<u64, RecoveryError> {
        bounded(async {
            let (mut tx, current) = self.begin(clock).await?;
            self.owner(&mut tx, account).await?;
            let count =
                sqlx::query("DELETE FROM rullst_api_tokens WHERE namespace = $1 AND subject = $2")
                    .bind(&self.config.namespace)
                    .bind(&account.subject)
                    .execute(&mut *tx)
                    .await?
                    .rows_affected();
            self.finish(&mut tx, clock, current, i64::MAX).await?;
            tx.commit().await?;
            Ok(count)
        })
        .await
    }

    fn lifetime(&self, lifetime: u32) -> Result<(), RecoveryError> {
        if lifetime == 0 || lifetime > self.config.lifetime {
            Err(RecoveryError::InvalidInput)
        } else {
            Ok(())
        }
    }
    pub(super) fn after_commit(
        &self,
        clock: &impl AuthClock,
        finished: i64,
        expiry: i64,
    ) -> Result<i64, RecoveryError> {
        let current = now(clock)?;
        if current < finished || current >= expiry {
            Err(RecoveryError::InvalidAction)
        } else {
            Ok(current)
        }
    }
    async fn owned(
        &self,
        tx: &mut Transaction<'_, Any>,
        account: &AuthenticatedRecoveryAccount,
        id: &ApiTokenId,
        current: i64,
    ) -> Result<ApiTokenMetadata, RecoveryError> {
        let row=sqlx::query("SELECT id,scopes,label,revision,created_at,issued_at,expires_at FROM \
                rullst_api_tokens WHERE namespace = $1 AND subject = $2 AND account_epoch = $3 AND id \
                = $4 AND expires_at > $5")
            .bind(&self.config.namespace).bind(&account.subject).bind(account.version).bind(id.as_str()).bind(current).fetch_optional(&mut **tx).await?.ok_or(RecoveryError::InvalidAction)?;

        let metadata = self.row(&row)?;
        if metadata.issued > current {
            return Err(RecoveryError::InvalidAction);
        }
        Ok(metadata)
    }
}
