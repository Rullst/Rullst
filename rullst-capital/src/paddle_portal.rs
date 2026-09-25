//! Short-lived customer portal access. Treat the URL as a bearer credential.

use crate::CapitalError;
use zeroize::Zeroizing;

/// A newly created overview session for one verified Paddle customer.
///
/// The URL grants access to the customer's entire portal, not one subscription.
/// Do not persist, cache, log, or embed it in an iframe. Return it only to the
/// authorized customer with `Cache-Control: no-store` and
/// `Referrer-Policy: no-referrer`. Paddle controls expiry; this receipt does not
/// establish a lifetime, single-use guarantee, or application-side revocation.
///
/// Debug is redacted and serialization/cloning are intentionally not provided.
/// The owned URL is zeroized on drop; this does not erase transport/parser
/// buffers or copies made by the application.
pub struct PaddlePortalSession {
    pub(crate) id: String,
    pub(crate) customer: String,
    pub(crate) url: Zeroizing<String>,
    pub(crate) sandbox: Option<bool>,
}

impl PaddlePortalSession {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn customer_id(&self) -> &str {
        &self.customer
    }

    /// Sensitive bearer URL. Expose only after authenticating and authorizing
    /// the account owner; do not place it in diagnostics or telemetry.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// `Some(true)` selects sandbox, `Some(false)` production; `None` is mock.
    pub fn sandbox(&self) -> Option<bool> {
        self.sandbox
    }

    pub fn is_mock(&self) -> bool {
        self.sandbox.is_none()
    }

    /// Rejects local simulation; this is not proof of provider interoperability.
    pub fn require_real(&self) -> Result<(), CapitalError> {
        if self.is_mock() {
            return Err(CapitalError::ConfigurationError(
                "A mock Paddle portal session cannot grant real customer access".into(),
            ));
        }
        Ok(())
    }
}

impl std::fmt::Debug for PaddlePortalSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PaddlePortalSession")
            .finish_non_exhaustive()
    }
}
