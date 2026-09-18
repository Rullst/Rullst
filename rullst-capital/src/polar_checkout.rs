//! Product-based Polar checkout with an explicit application customer identity.
use crate::CapitalError;
use std::net::IpAddr;

/// One server-selected product and an opaque, stable application customer ID.
/// Persist checkout intent before dispatch. Polar checkout creation is not
/// documented as idempotent: never blindly retry an uncertain creation outcome.
#[derive(Clone)]
pub struct PolarCheckoutRequest {
    pub(crate) product: String,
    pub(crate) owner: String,
    pub(crate) success: String,
    pub(crate) email: Option<String>,
    pub(crate) ip: Option<IpAddr>,
}

impl PolarCheckoutRequest {
    pub fn new(
        product_id: impl Into<String>,
        external_customer_id: impl Into<String>,
        success_url: impl Into<String>,
    ) -> Result<Self, CapitalError> {
        let mut request = Self {
            product: product_id.into(),
            owner: external_customer_id.into(),
            success: success_url.into(),
            email: None,
            ip: None,
        };
        request.product.make_ascii_lowercase();
        if !uuid(&request.product)
            || !crate::providers::stripe_contract::valid_reference(&request.owner, "", 200)
        {
            return Err(CapitalError::ConfigurationError(
                "Polar checkout requires a product UUID and an opaque external customer ID".into(),
            ));
        }
        crate::providers::validate_checkout_url("polar-redirect", &request.success)?;
        Ok(request)
    }

    /// Optional contact data, never an ownership key.
    pub fn with_email(mut self, email: impl Into<String>) -> Result<Self, CapitalError> {
        let validated = crate::StripeCustomerRequest::new("polar", "contact")?.with_email(email)?;
        self.email = validated.email().map(str::to_owned);
        Ok(self)
    }

    /// Use only an IP resolved by the application's trusted proxy boundary.
    /// This adapter never reads Forwarded/X-Forwarded-For headers. Omit the IP
    /// when no trusted socket/proxy identity is available.
    pub fn with_trusted_client_ip(mut self, ip: IpAddr) -> Self {
        self.ip = Some(ip);
        self
    }
    pub fn product_id(&self) -> &str {
        &self.product
    }
    pub fn external_customer_id(&self) -> &str {
        &self.owner
    }
}

impl std::fmt::Debug for PolarCheckoutRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PolarCheckoutRequest")
            .finish_non_exhaustive()
    }
}

/// Persist the ID and identity bindings before redirecting. Never payment evidence.
#[derive(Clone)]
pub struct PolarCheckoutSession {
    pub(crate) id: String,
    pub(crate) url: String,
    pub(crate) expires_at: Option<i64>,
    pub(crate) mock: bool,
}
impl PolarCheckoutSession {
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn url(&self) -> &str {
        &self.url
    }
    pub fn expires_at(&self) -> Option<i64> {
        self.expires_at
    }
    pub fn is_mock(&self) -> bool {
        self.mock
    }
}
impl std::fmt::Debug for PolarCheckoutSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PolarCheckoutSession")
            .field("mock", &self.mock)
            .finish_non_exhaustive()
    }
}

pub(crate) fn uuid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(index, byte)| {
            if matches!(index, 8 | 13 | 18 | 23) {
                byte == b'-'
            } else {
                byte.is_ascii_hexdigit()
            }
        })
}
