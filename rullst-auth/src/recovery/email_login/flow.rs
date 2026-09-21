use super::*;
use sqlx::Row;
use zeroize::Zeroizing;

pub(super) const LINK_LIFETIME: i64 = 900;

impl EmailLoginService {
    /// Uniform acknowledgement for unknown, disabled, suppressed and throttled
    /// accounts. Queueing is atomic and independent from transport availability.
    /// The host must also enforce CSRF and independent ingress/IP abuse limits.
    /// Minimum response delay reduces trivial enumeration; it is not a timing
    /// anonymity guarantee under contention or deployment failures.
    pub async fn request_login(
        &self,
        email: &str,
        browser: &BrowserBinding,
        clock: &impl EmailLoginClock,
    ) -> Result<LoginRequestAccepted, RecoveryError> {
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(250);
        let result = bounded(self.request(email, browser, clock)).await;
        tokio::time::sleep_until(deadline).await;
        result
    }

    async fn request(
        &self,
        email: &str,
        browser: &BrowserBinding,
        clock: &impl EmailLoginClock,
    ) -> Result<LoginRequestAccepted, RecoveryError> {
        let email = if email.len() <= 254 {
            super::super::normalized_email(email)
                .ok()
                .map(Zeroizing::new)
        } else {
            None
        };
        let (mut tx, current) = self.begin(clock).await?;
        if !self.rate(&mut tx, current, false).await? {
            tx.commit().await?;
            return Ok(LoginRequestAccepted);
        }
        let email_key = self
            .store
            .keys
            .digest("email", email.as_deref().map_or("", String::as_str));
        let row = sqlx::query("SELECT a.subject,a.session_version,a.locale,p.revision,p.request_window,p.requests FROM rullst_recovery_accounts a JOIN rullst_email_login_accounts p ON p.subject = a.subject AND p.namespace = $1 WHERE a.email_key = $2 AND a.suppressed = 0 AND p.enabled = 1")
            .bind(&self.config.namespace).bind(email_key).fetch_optional(&mut *tx).await?;
        let (Some(row), Some(email)) = (row, email) else {
            tx.commit().await?;
            return Ok(LoginRequestAccepted);
        };
        let previous: i64 = row.try_get("request_window")?;
        let attempts: i64 = row.try_get("requests")?;
        if previous < 0 || attempts < 0 || current < previous {
            return Err(RecoveryError::Configuration);
        }
        let (window, attempts) = if current - previous >= LINK_LIFETIME {
            (current, 0)
        } else {
            (previous, attempts)
        };
        if attempts >= 3 {
            tx.commit().await?;
            return Ok(LoginRequestAccepted);
        }
        let subject: String = row.try_get("subject")?;
        let epoch: i64 = row.try_get("session_version")?;
        let revision: i64 = row.try_get("revision")?;
        if epoch < 1 || revision < 1 {
            return Err(RecoveryError::Configuration);
        }
        let token = SecretToken::generate()?;
        let expiry = current + LINK_LIFETIME;
        self.cancel_pending(&mut tx, &subject).await?;
        sqlx::query("UPDATE rullst_email_login_accounts SET request_window = $1,requests = $2 WHERE namespace = $3 AND subject = $4")
            .bind(window).bind(attempts + 1).bind(&self.config.namespace).bind(&subject).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO rullst_email_login_tokens (namespace,subject,token_digest,browser_digest,account_epoch,policy_revision,issued_at,expires_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)")
            .bind(&self.config.namespace).bind(&subject).bind(self.digest("email-login-token-v1", token.expose()))
            .bind(self.digest("email-login-browser-v1", browser.expose_cookie())).bind(epoch).bind(revision).bind(current).bind(expiry).execute(&mut *tx).await?;
        let locale = super::super::RecoveryLocale::parse(&row.try_get::<String, _>("locale")?)?;
        self.enqueue_login(
            &mut tx,
            &subject,
            &delivery::Notice {
                recipient: email,
                token,
                expires_at: expiry,
                locale,
            },
            current,
        )
        .await?;
        let finished = self.finish_time(&mut tx, clock, current, expiry).await?;
        tx.commit().await?;
        if now(clock)? < finished {
            return Err(RecoveryError::InvalidAction);
        }
        Ok(LoginRequestAccepted)
    }

    /// Call exclusively from an explicit, CSRF-protected POST in the initiating
    /// browser. GET, HEAD, mail-preview scanners and JavaScript on page load must
    /// never call this operation. The email credential alone cannot authenticate.
    /// Session creation and single-use consumption commit atomically. A timeout
    /// or uncertain commit never returns a session; request a fresh link instead.
    pub async fn redeem(
        &self,
        email_token: &str,
        browser: &BrowserBinding,
        clock: &impl EmailLoginClock,
    ) -> Result<EmailLoginSession, RecoveryError> {
        bounded(self.consume(email_token, browser, clock)).await
    }

    async fn consume(
        &self,
        email_token: &str,
        browser: &BrowserBinding,
        clock: &impl EmailLoginClock,
    ) -> Result<EmailLoginSession, RecoveryError> {
        let (mut tx, current) = self.begin(clock).await?;
        if !self.rate(&mut tx, current, true).await? {
            tx.commit().await?;
            return Err(RecoveryError::InvalidAction);
        }
        let token = SecretToken::from_encoded(email_token);
        let digest = self.digest(
            "email-login-token-v1",
            token.as_ref().map_or("", SecretToken::expose),
        );
        let browser_digest = self.digest("email-login-browser-v1", browser.expose_cookie());
        let row = sqlx::query("SELECT t.subject,t.account_epoch,t.issued_at,t.expires_at FROM rullst_email_login_tokens t JOIN rullst_email_login_accounts p ON p.namespace = t.namespace AND p.subject = t.subject AND p.revision = t.policy_revision JOIN rullst_recovery_accounts a ON a.subject = t.subject AND a.session_version = t.account_epoch WHERE t.namespace = $1 AND t.token_digest = $2 AND t.browser_digest = $3 AND p.enabled = 1 AND a.suppressed = 0 AND t.expires_at > $4 AND t.issued_at <= $4")
            .bind(&self.config.namespace).bind(digest).bind(browser_digest).bind(current).fetch_optional(&mut *tx).await?;
        let Some(row) = row else {
            tx.commit().await?;
            return Err(RecoveryError::InvalidAction);
        };
        let expiry: i64 = row.try_get("expires_at")?;
        let issued: i64 = row.try_get("issued_at")?;
        let epoch: i64 = row.try_get("account_epoch")?;
        if token.is_err()
            || issued < 0
            || expiry.checked_sub(issued) != Some(LINK_LIFETIME)
            || epoch < 1
        {
            return Err(RecoveryError::Configuration);
        }
        let subject: String = row.try_get("subject")?;
        let finished = self.finish_time(&mut tx, clock, current, expiry).await?;
        let account = AuthenticatedRecoveryAccount {
            subject: subject.clone(),
            version: epoch,
            instance: self.store.instance.clone(),
        };
        let token = self
            .store
            .insert_session(&mut tx, &account, finished, 3600, None)
            .await?;
        self.cancel_pending(&mut tx, &subject).await?;
        let finished = self.finish_time(&mut tx, clock, finished, expiry).await?;
        tx.commit().await?;
        let after = now(clock)?;
        if after < finished || after >= expiry {
            return Err(RecoveryError::InvalidAction);
        }
        Ok(EmailLoginSession {
            token,
            subject,
            destination: self.config.destination.clone(),
        })
    }
}
