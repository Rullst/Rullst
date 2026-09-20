//! Account mail bridge: Auth owns the transaction, Mail owns rendering/transport.

use crate::{
    auth::recovery::{
        RecoveryDeliveryFailure, RecoveryError, RecoveryNoticeKind, SqlRecoveryStore,
    },
    mail::{AccountEvent, AccountMail, ActionLink, MailDriver, MailFailureClass, MailLocale},
};

/// Immutable server-owned delivery configuration. Provider/domain tracking must
/// be disabled independently, including settings that override per-message flags.
pub struct AccountMailConfig {
    origin: String,
    reset_endpoint: String,
    application_name: String,
    sender: String,
    locale: MailLocale,
}

impl AccountMailConfig {
    pub fn new(
        origin: impl Into<String>,
        reset_endpoint: impl Into<String>,
        application_name: impl Into<String>,
        sender: impl Into<String>,
        locale: MailLocale,
    ) -> Result<Self, RecoveryError> {
        let origin = origin.into();
        let reset_endpoint = reset_endpoint.into();
        ActionLink::new(&reset_endpoint, &origin).map_err(|_| RecoveryError::Configuration)?;
        if reset_endpoint.contains(['?', '#']) {
            return Err(RecoveryError::Configuration);
        }
        let application_name = application_name.into();
        let sender = sender.into();
        let test = AccountMail::new(
            "validation@example.com",
            &application_name,
            locale,
            AccountEvent::Welcome,
        )
        .map_err(|_| RecoveryError::Configuration)?
        .from(&sender)
        .into_message();
        crate::mail::DeliveryPipeline::prepare(&test).map_err(|_| RecoveryError::Configuration)?;
        Ok(Self {
            origin,
            reset_endpoint,
            application_name,
            sender,
            locale,
        })
    }
}

/// Low-cardinality worker result; no recipient or action URL is exposed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AccountMailOutcome {
    Idle,
    Delivered,
    RetryScheduled,
    Failed,
}

/// Sends one leased encrypted notice with a 30-second timeout. Run under a
/// bounded worker loop, separate from the password-reset request handler.
/// Delivery retries can duplicate mail on transports without native idempotency.
pub async fn deliver_next_account_mail(
    store: &SqlRecoveryStore,
    driver: &impl MailDriver,
    config: &AccountMailConfig,
) -> Result<AccountMailOutcome, RecoveryError> {
    let now = unix_now()?;
    let Some(claim) = store.claim_notice(now).await? else {
        return Ok(AccountMailOutcome::Idle);
    };
    let notice = claim.notice();
    let event = match notice.kind() {
        RecoveryNoticeKind::Welcome => AccountEvent::Welcome,
        RecoveryNoticeKind::PasswordChanged => AccountEvent::PasswordChanged,
        RecoveryNoticeKind::PasswordReset => {
            let token = notice.token().ok_or(RecoveryError::InvalidAction)?;
            let link = ActionLink::new(
                format!("{}?token={}", config.reset_endpoint, token.expose()),
                &config.origin,
            )
            .map_err(|_| RecoveryError::Configuration)?;
            let expires_at = chrono::DateTime::from_timestamp(notice.expires_at(), 0)
                .ok_or(RecoveryError::InvalidAction)?;
            AccountEvent::PasswordResetAt { link, expires_at }
        }
        _ => return Err(RecoveryError::InvalidAction),
    };
    let message = AccountMail::new(
        notice.recipient(),
        &config.application_name,
        notice.locale().map_or(config.locale, |locale| {
            MailLocale::from_preference(locale.as_str())
        }),
        event,
    )
    .map_err(|_| RecoveryError::Configuration)?
    .from(&config.sender)
    .into_message();
    let sent = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        driver.send_with_delivery_id(&message, claim.delivery_id()),
    )
    .await;
    match sent {
        Ok(Ok(())) => {
            store.complete_notice(&claim, unix_now()?).await?;
            Ok(AccountMailOutcome::Delivered)
        }
        result => {
            let transient = match result {
                Err(_) => true,
                Ok(Err(error)) => matches!(
                    error.failure_class(),
                    MailFailureClass::Transient | MailFailureClass::RateLimited
                ),
                Ok(Ok(())) => false,
            };
            store
                .fail_notice(
                    &claim,
                    if transient {
                        RecoveryDeliveryFailure::Transient
                    } else {
                        RecoveryDeliveryFailure::Permanent
                    },
                    unix_now()?,
                )
                .await?;
            Ok(if transient {
                AccountMailOutcome::RetryScheduled
            } else {
                AccountMailOutcome::Failed
            })
        }
    }
}

fn unix_now() -> Result<u64, RecoveryError> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|time| time.as_secs())
        .map_err(|_| RecoveryError::Configuration)
}
