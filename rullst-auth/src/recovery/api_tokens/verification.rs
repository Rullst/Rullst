use super::*;
use sqlx::Row;
use subtle::ConstantTimeEq;

impl ApiTokenService {
    /// Verifies authoritative state and every exact required scope, without a
    /// positive cache. Missing/revoked/expired/wrong-purpose credentials fail.
    /// Hosts must still resolve current tenant membership, domain permissions
    /// and resource ownership. Already-authorized work is not recalled later.
    pub async fn verify(
        &self,
        bearer: &str,
        required: &ApiScopes,
        clock: &impl AuthClock,
    ) -> Result<ApiTokenPrincipal, RecoveryError> {
        bounded(async {
            let id = types::parse_bearer(bearer)?;
            if !self.config.scopes.includes(required) {
                return Err(RecoveryError::InvalidAction);
            }
            let (mut tx, current) = self.begin(clock).await?;
            let row = sqlx::query(
                "SELECT t.id,t.subject,t.token_digest,t.scopes,t.label,t.revision,
                 t.created_at,t.issued_at,t.expires_at FROM rullst_api_tokens t
                 JOIN rullst_recovery_accounts a ON a.subject = t.subject
                 AND a.session_version = t.account_epoch WHERE t.namespace = $1
                 AND t.id = $2 AND t.expires_at > $3 AND t.issued_at <= $3",
            )
            .bind(&self.config.namespace)
            .bind(id.as_str())
            .bind(current)
            .fetch_optional(&mut *tx)
            .await?;
            let Some(row) = row else {
                tx.commit().await?;
                return Err(RecoveryError::InvalidAction);
            };
            let expected: String = row.try_get("token_digest")?;
            let digest = self.digest(bearer);
            let metadata = self.row(&row)?;
            if !bool::from(expected.as_bytes().ct_eq(digest.as_bytes()))
                || !metadata.scopes.includes(required)
            {
                tx.commit().await?;
                return Err(RecoveryError::InvalidAction);
            }
            let subject: String = row.try_get("subject")?;
            if !super::super::valid_subject(&subject) {
                return Err(RecoveryError::Configuration);
            }
            let finished = self
                .finish(&mut tx, clock, current, metadata.expires)
                .await?;
            tx.commit().await?;
            self.after_commit(clock, finished, metadata.expires)?;
            Ok(ApiTokenPrincipal {
                subject,
                namespace: self.config.namespace.clone(),
                metadata,
            })
        })
        .await
    }
}
