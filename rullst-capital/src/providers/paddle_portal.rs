use super::PaddleProvider;
use crate::paddle_checkout::{digest, id};
use crate::{CapitalError, PaddleCustomerRequest, PaddlePortalSession, ProviderFailure};
use reqwest::{Method, Url};
use serde_json::Value;
use zeroize::Zeroizing;

const OPERATION: &str = "create customer portal session";
const MAX_PORTAL_URL_BYTES: usize = 8192;

impl PaddleProvider {
    /// Creates fresh customer-wide portal access after checking the active
    /// customer's original owner and provisioning-attempt metadata.
    ///
    /// Authenticate/authorize the caller first. Load both arguments from trusted
    /// tenant/account/environment-scoped state, never browser input or an email
    /// lookup. The request must describe the original customer provisioning
    /// attempt, not a checkout attempt. Provider metadata must remain host-owned;
    /// the customer read and session creation are separate provider operations.
    ///
    /// Requires `customer.read` and `customer_portal_session.write`. This method
    /// makes one creation attempt, without automatic retry or caching. It does
    /// not accept a return URL or subscription deep links. Empty/`mock_*` keys
    /// return a deterministic, explicitly mock `example.invalid` URL.
    pub async fn create_bound_customer_portal(
        &self,
        provisioning: &PaddleCustomerRequest,
        customer_id: impl Into<String>,
    ) -> Result<PaddlePortalSession, CapitalError> {
        let customer_id = customer_id.into();
        if !id(&customer_id, "ctm_") {
            return Err(CapitalError::ConfigurationError(
                "Paddle portal access requires a bound customer ID".into(),
            ));
        }
        self.retrieve_bound_customer(provisioning, &customer_id)
            .await?;
        if self.offline() {
            let suffix = hex::encode(digest(
                b"rullst.paddle.mock-portal.v1",
                &[&customer_id, provisioning.owner_reference()],
            ));
            let session_id = format!("cpls_{}", &suffix[..26]);
            return Ok(PaddlePortalSession {
                url: Zeroizing::new(format!(
                    "https://example.invalid/rullst/paddle/mock-portal/{session_id}"
                )),
                id: session_id,
                customer: customer_id,
                sandbox: None,
            });
        }
        let response = self
            .billing_json(
                Method::POST,
                &format!("/customers/{customer_id}/portal-sessions"),
                None,
                OPERATION,
            )
            .await?;
        parse_session(&response["data"], &customer_id, self.sandbox)
    }
}

fn parse_session(
    data: &Value,
    customer: &str,
    sandbox: bool,
) -> Result<PaddlePortalSession, CapitalError> {
    let session_id = data["id"]
        .as_str()
        .filter(|value| id(value, "cpls_"))
        .ok_or_else(mismatch)?;
    if data["customer_id"].as_str() != Some(customer) {
        return Err(mismatch());
    }
    let url = data["urls"]["general"]["overview"]
        .as_str()
        .ok_or_else(mismatch)?;
    validate_url(url, sandbox)?;
    Ok(PaddlePortalSession {
        id: session_id.into(),
        customer: customer.into(),
        url: Zeroizing::new(url.into()),
        sandbox: Some(sandbox),
    })
}

// Accept only the documented overview-link profile. Never follow the URL on
// the server or decode/trust its token as local identity/expiry evidence.
fn validate_url(raw: &str, sandbox: bool) -> Result<(), CapitalError> {
    if raw.len() > MAX_PORTAL_URL_BYTES
        || raw.chars().any(|c| c.is_whitespace() || c.is_control())
        || raw.contains('\\')
    {
        return Err(mismatch());
    }
    let url = Url::parse(raw).map_err(|_| mismatch())?;
    let host = if sandbox {
        "sandbox-customer-portal.paddle.com"
    } else {
        "customer-portal.paddle.com"
    };
    if url.scheme() != "https"
        || url.host_str() != Some(host)
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
        || url.fragment().is_some()
        || !url.path().strip_prefix('/').is_some_and(|p| id(p, "cpl_"))
    {
        return Err(mismatch());
    }
    let mut action = false;
    let mut token = false;
    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "action" if !action && value == "overview" => action = true,
            "token"
                if !token
                    && !value.is_empty()
                    && !value.chars().any(|c| c.is_whitespace() || c.is_control()) =>
            {
                token = true;
            }
            _ => return Err(mismatch()),
        }
    }
    if !action || !token {
        return Err(mismatch());
    }
    Ok(())
}

fn mismatch() -> CapitalError {
    ProviderFailure::contract_mismatch("paddle", OPERATION).into()
}

#[cfg(test)]
#[path = "paddle_portal_tests.rs"]
mod tests;
