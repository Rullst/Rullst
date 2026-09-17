use super::StripeInboxError;
use crate::StripeSubscriptionEvent;
use crate::providers::stripe_contract::valid_reference;
use ring::digest::{Context, SHA256};

/// Trusted endpoint configuration, never inferred from webhook contact data.
///
/// The host must establish which Stripe account owns its endpoint secret and
/// credentials. An ordinary platform event does not carry that account ID.
#[derive(Clone)]
pub struct StripeInboxScope {
    pub(super) hash: String,
    account: String,
    connected: bool,
    livemode: bool,
}

impl StripeInboxScope {
    /// Accepts ordinary platform-account events without a connected-account field.
    pub fn platform(
        application_namespace: impl Into<String>,
        account: impl Into<String>,
        livemode: bool,
    ) -> Result<Self, StripeInboxError> {
        Self::new(
            application_namespace.into(),
            account.into(),
            livemode,
            false,
        )
    }

    /// Accepts Connect events carrying exactly the configured connected account.
    /// This does not add Connect support to checkout/customer HTTP adapters.
    pub fn connected(
        application_namespace: impl Into<String>,
        account: impl Into<String>,
        livemode: bool,
    ) -> Result<Self, StripeInboxError> {
        Self::new(application_namespace.into(), account.into(), livemode, true)
    }

    fn new(
        namespace: String,
        account: String,
        livemode: bool,
        connected: bool,
    ) -> Result<Self, StripeInboxError> {
        if !valid_reference(&namespace, "", 200) || !valid_reference(&account, "acct_", 200) {
            return Err(StripeInboxError::InvalidConfiguration);
        }
        let mut hash = Context::new(&SHA256);
        hash.update(b"rullst.stripe.inbox-scope.v1\0");
        hash.update(&[u8::from(livemode), u8::from(connected)]);
        for field in [&namespace, &account] {
            hash.update(&(field.len() as u64).to_be_bytes());
            hash.update(field.as_bytes());
        }
        Ok(Self {
            hash: hex::encode(hash.finish()),
            account,
            connected,
            livemode,
        })
    }

    pub(super) fn verify(&self, event: &StripeSubscriptionEvent) -> Result<(), StripeInboxError> {
        event
            .require_real()
            .map_err(|_| StripeInboxError::MockEvent)?;
        let expected = self.connected.then_some(self.account.as_str());
        if event.livemode() != self.livemode || event.connected_account() != expected {
            return Err(StripeInboxError::ScopeMismatch);
        }
        Ok(())
    }
}

impl std::fmt::Debug for StripeInboxScope {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StripeInboxScope")
            .field("connected", &self.connected)
            .field("livemode", &self.livemode)
            .finish_non_exhaustive()
    }
}
