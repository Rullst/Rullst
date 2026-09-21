use super::super::{RecoveryDeliveryFailure, RecoveryLocale};
use super::*;
use serde::{Deserialize, Serialize};
use sqlx::{Any, Row, Transaction};
use zeroize::Zeroizing;

pub(super) struct Notice {
    pub recipient: Zeroizing<String>,
    pub token: SecretToken,
    pub expires_at: i64,
    pub locale: Option<RecoveryLocale>,
}

/// Fenced, short-lived delivery capability. Debug redacts the account, link,
/// browser, lease and recipient. Worker access is a trusted server capability.
pub struct EmailLoginDelivery {
    id: String,
    lease: SecretToken,
    namespace: String,
    landing: url::Url,
    notice: Notice,
}
impl std::fmt::Debug for EmailLoginDelivery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EmailLoginDelivery([REDACTED])")
    }
}
impl EmailLoginDelivery {
    /// Stable across retries; pass to the mail transport's idempotency mechanism.
    pub fn delivery_id(&self) -> &str {
        &self.id
    }
    pub fn recipient(&self) -> &str {
        &self.notice.recipient
    }
    pub fn locale(&self) -> Option<RecoveryLocale> {
        self.notice.locale
    }
    pub fn expires_at(&self) -> i64 {
        self.notice.expires_at
    }
    /// Sensitive link for the intended email only; never log it or put it in
    /// telemetry. It contains no browser-binding secret or redirect parameter.
    pub fn expose_link(&self) -> Zeroizing<String> {
        let mut url = self.landing.clone();
        url.query_pairs_mut()
            .append_pair("token", self.notice.token.expose());
        Zeroizing::new(url.into())
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireNotice {
    recipient: String,
    token: String,
    expires_at: i64,
    locale: Option<RecoveryLocale>,
}
impl Drop for WireNotice {
    fn drop(&mut self) {
        use zeroize::Zeroize;
        self.recipient.zeroize();
        self.token.zeroize();
    }
}

impl EmailLoginService {
    fn aad(&self, id: &str) -> String {
        format!("rullst.auth.email-login.v1:{}:{id}", self.config.namespace)
    }

    pub(super) async fn enqueue_login(
        &self,
        tx: &mut Transaction<'_, Any>,
        subject: &str,
        notice: &Notice,
        current: i64,
    ) -> Result<(), RecoveryError> {
        self.purge(&mut *tx, current).await?;
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM rullst_email_login_outbox WHERE namespace = $1",
        )
        .bind(&self.config.namespace)
        .fetch_one(&mut **tx)
        .await?;
        if count >= self.config.capacity as i64 {
            return Err(RecoveryError::Limited);
        }
        let id = SecretToken::generate()?;
        let wire = WireNotice {
            recipient: notice.recipient.to_string(),
            token: notice.token.expose().to_owned(),
            expires_at: notice.expires_at,
            locale: notice.locale,
        };
        let plaintext =
            Zeroizing::new(serde_json::to_vec(&wire).map_err(|_| RecoveryError::Crypto)?);
        let ciphertext = self.store.keys.seal(&self.aad(id.expose()), &plaintext)?;
        sqlx::query("INSERT INTO rullst_email_login_outbox (id,namespace,subject,ciphertext,expires_at,status,attempts,due_at,lease,lease_until) VALUES ($1,$2,$3,$4,$5,'pending',0,$6,'',0)")
            .bind(id.expose()).bind(&self.config.namespace).bind(subject).bind(ciphertext).bind(notice.expires_at).bind(current).execute(&mut **tx).await?;
        Ok(())
    }

    async fn purge(
        &self,
        tx: &mut Transaction<'_, Any>,
        current: i64,
    ) -> Result<(), RecoveryError> {
        sqlx::query("DELETE FROM rullst_email_login_outbox WHERE namespace = $1 AND (expires_at <= $2 OR (attempts >= 6 AND status = 'leased' AND lease_until <= $2))")
            .bind(&self.config.namespace).bind(current).execute(&mut **tx).await?;
        sqlx::query(
            "DELETE FROM rullst_email_login_tokens WHERE namespace = $1 AND expires_at <= $2",
        )
        .bind(&self.config.namespace)
        .bind(current)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    /// Claims one currently valid notice for at most 60 seconds, six attempts.
    /// Expired tokens are removed; account revocation, opt-out or suppression is
    /// rechecked. At-least-once delivery can duplicate mail after a worker crash;
    /// single-use redemption and browser binding still apply to every duplicate.
    pub async fn claim_notice(
        &self,
        clock: &impl EmailLoginClock,
    ) -> Result<Option<EmailLoginDelivery>, RecoveryError> {
        bounded(async {
            let (mut tx,current) = self.begin(clock).await?;
            self.purge(&mut tx, current).await?;
            let row = sqlx::query("SELECT o.id,o.ciphertext,o.expires_at,t.token_digest FROM rullst_email_login_outbox o JOIN rullst_email_login_tokens t ON t.namespace = o.namespace AND t.subject = o.subject JOIN rullst_email_login_accounts p ON p.namespace = t.namespace AND p.subject = t.subject AND p.revision = t.policy_revision JOIN rullst_recovery_accounts a ON a.subject = t.subject AND a.session_version = t.account_epoch WHERE o.namespace = $1 AND p.enabled = 1 AND a.suppressed = 0 AND o.attempts < 6 AND o.due_at <= $2 AND t.expires_at > $2 AND (o.status = 'pending' OR (o.status = 'leased' AND o.lease_until <= $2)) ORDER BY o.due_at,o.id LIMIT 1")
                .bind(&self.config.namespace).bind(current).fetch_optional(&mut *tx).await?;
            let Some(row) = row else { tx.commit().await?; return Ok(None); };
            let id: String = row.try_get("id")?;
            let plaintext = self.store.keys.open(&self.aad(&id), &row.try_get::<String,_>("ciphertext")?)?;
            let wire: WireNotice = serde_json::from_slice(&plaintext).map_err(|_| RecoveryError::Crypto)?;
            let token = SecretToken::from_encoded(&wire.token)?;
            let digest = self.digest("email-login-token-v1", token.expose());
            let expected: String = row.try_get("token_digest")?;
            use subtle::ConstantTimeEq;
            if !bool::from(digest.as_bytes().ct_eq(expected.as_bytes())) || wire.expires_at != row.try_get::<i64,_>("expires_at")?
                || super::super::normalized_email(&wire.recipient).as_deref() != Ok(wire.recipient.as_str()) {
                return Err(RecoveryError::Crypto);
            }
            let finished = self.finish_time(&mut tx, clock, current, wire.expires_at).await?;
            let lease = SecretToken::generate()?;
            sqlx::query("UPDATE rullst_email_login_outbox SET status = 'leased',attempts = attempts + 1,lease = $1,lease_until = $2 WHERE namespace = $3 AND id = $4")
                .bind(lease.expose()).bind((finished + 60).min(wire.expires_at)).bind(&self.config.namespace).bind(&id).execute(&mut *tx).await?;
            tx.commit().await?;
            let after = now(clock)?;
            if after < finished || after >= wire.expires_at { return Err(RecoveryError::InvalidAction); }
            Ok(Some(EmailLoginDelivery { id, lease, namespace: self.config.namespace.clone(), landing: self.config.landing.clone(), notice: Notice { recipient: Zeroizing::new(wire.recipient.clone()), token, expires_at: wire.expires_at, locale: wire.locale } }))
        }).await
    }

    /// Completes only a live lease. Delivered ciphertext is deleted immediately.
    /// A provider may accept mail after revocation; its link then fails redemption.
    pub async fn complete_notice(
        &self,
        claim: &EmailLoginDelivery,
        clock: &impl EmailLoginClock,
    ) -> Result<(), RecoveryError> {
        self.finish_notice(claim, None, clock).await
    }

    pub async fn fail_notice(
        &self,
        claim: &EmailLoginDelivery,
        failure: RecoveryDeliveryFailure,
        clock: &impl EmailLoginClock,
    ) -> Result<(), RecoveryError> {
        self.finish_notice(claim, Some(failure), clock).await
    }

    async fn finish_notice(
        &self,
        claim: &EmailLoginDelivery,
        failure: Option<RecoveryDeliveryFailure>,
        clock: &impl EmailLoginClock,
    ) -> Result<(), RecoveryError> {
        bounded(async {
            if claim.namespace != self.config.namespace { return Err(RecoveryError::InvalidAction); }
            let (mut tx,current) = self.begin(clock).await?;
            let row = sqlx::query("SELECT subject,attempts,lease_until FROM rullst_email_login_outbox WHERE namespace = $1 AND id = $2 AND lease = $3 AND status = 'leased' AND lease_until > $4 AND expires_at > $4")
                .bind(&self.config.namespace).bind(&claim.id).bind(claim.lease.expose()).bind(current).fetch_optional(&mut *tx).await?.ok_or(RecoveryError::InvalidAction)?;
            let attempts: i64 = row.try_get("attempts")?;
            let subject: String = row.try_get("subject")?;
            let expiry: i64 = row.try_get("lease_until")?;
            if !(1..=6).contains(&attempts) { return Err(RecoveryError::Configuration); }
            if matches!(failure, Some(RecoveryDeliveryFailure::Transient)) && attempts < 6 {
                sqlx::query("UPDATE rullst_email_login_outbox SET status = 'pending',due_at = $1,lease = '',lease_until = 0 WHERE namespace = $2 AND id = $3")
                    .bind(current + 15 * (1_i64 << attempts)).bind(&self.config.namespace).bind(&claim.id).execute(&mut *tx).await?;
            } else {
                sqlx::query("DELETE FROM rullst_email_login_outbox WHERE namespace = $1 AND id = $2")
                    .bind(&self.config.namespace).bind(&claim.id).execute(&mut *tx).await?;
            }
            if matches!(failure, Some(RecoveryDeliveryFailure::PermanentBounce | RecoveryDeliveryFailure::Complaint)) {
                sqlx::query("UPDATE rullst_recovery_accounts SET suppressed = 1 WHERE subject = $1").bind(&subject).execute(&mut *tx).await?;
                self.cancel_pending(&mut tx, &subject).await?;
            }
            self.finish_time(&mut tx, clock, current, expiry).await?;
            tx.commit().await?;
            Ok(())
        }).await
    }

    /// Trusted retention worker; runs even when no requests or deliveries occur.
    /// No recipient history is retained after expiry. Clock rollback fails closed.
    pub async fn purge_expired(&self, clock: &impl EmailLoginClock) -> Result<(), RecoveryError> {
        bounded(async {
            let (mut tx, current) = self.begin(clock).await?;
            self.purge(&mut tx, current).await?;
            tx.commit().await?;
            Ok(())
        })
        .await
    }
}
