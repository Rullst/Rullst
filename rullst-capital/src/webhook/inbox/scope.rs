use super::StripeInboxError;
use crate::StripeSubscriptionEvent;
use crate::providers::stripe_contract::valid_reference;
use sha2::{Digest, Sha256};

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
        let mut hash = Sha256::new();
        hash.update(b"rullst.stripe.inbox-scope.v1\0");
        hash.update([u8::from(livemode), u8::from(connected)]);
        for field in [&namespace, &account] {
            hash.update((field.len() as u64).to_be_bytes());
            hash.update(field.as_bytes());
        }
        Ok(Self {
            hash: hex::encode(hash.finalize()),
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

#[cfg(test)]
mod tests {
    use super::StripeInboxScope;

    #[test]
    fn scope_hashes_are_sha256_known_answers() {
        // SHA-256 of the v1 tag, [livemode, connected] and each
        // length-prefixed field, computed independently. Stored inbox rows
        // are keyed by this hash, so it must not change.
        let platform = StripeInboxScope::platform("billing", "acct_1KAT", false).unwrap();
        assert_eq!(
            platform.hash,
            "94fb098576f05816a91a3fd2ad6c3683796b36fbafc765b367797e97cc10e30d"
        );
        let connected = StripeInboxScope::connected("billing", "acct_1KAT", true).unwrap();
        assert_eq!(
            connected.hash,
            "a73df6b09469993ed85286063b9c5209693fcfe57b2667575ae56683c3c1d7e4"
        );
    }
}
