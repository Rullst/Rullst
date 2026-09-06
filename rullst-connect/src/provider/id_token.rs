//! Shared post-signature OIDC claim policy. Never validates an unverified payload.
use jsonwebtoken::{Algorithm, Validation};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::ConnectError;

pub(crate) fn validation(algorithm: Algorithm, client_id: &str, issuers: &[&str]) -> Validation {
    let mut validation = Validation::new(algorithm);
    // Setting issuer/audience expectations does not require their presence in
    // jsonwebtoken. The OIDC identity and lifetime claims must remain required
    // even when a nonce is expected; nonce is not a JWT registered claim.
    validation.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);
    validation.set_audience(&[client_id]);
    validation.set_issuer(issuers);
    validation
}

pub(crate) fn validate_claims(
    claims: &Value,
    client_id: &str,
    expected_nonce: Option<&str>,
) -> Result<(), ConnectError> {
    let invalid = || {
        ConnectError::Provider("OIDC id_token contains invalid identity or lifetime claims".into())
    };
    let subject = claims["sub"].as_str().ok_or_else(invalid)?;
    if subject.is_empty()
        || subject.len() > 255
        || !subject.is_ascii()
        || subject.bytes().any(|byte| byte.is_ascii_control())
    {
        return Err(invalid());
    }
    let issued_at = claims["iat"].as_u64().ok_or_else(invalid)?;
    let expires_at = claims["exp"].as_u64().ok_or_else(invalid)?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    if issued_at > now.saturating_add(60) || expires_at <= issued_at {
        return Err(invalid());
    }

    // No other audience trust list is configured by these adapters. Do not
    // infer trust in another client merely because this client also appears.
    let trusted_audience = match &claims["aud"] {
        Value::String(audience) => audience == client_id,
        Value::Array(audiences) => {
            !audiences.is_empty()
                && audiences
                    .iter()
                    .all(|audience| audience.as_str() == Some(client_id))
        }
        _ => false,
    };
    if !trusted_audience
        || claims
            .get("azp")
            .is_some_and(|azp| azp.as_str() != Some(client_id))
    {
        return Err(invalid());
    }
    if let Some(expected_nonce) = expected_nonce {
        let nonce = claims["nonce"].as_str().unwrap_or("");
        if expected_nonce.is_empty()
            || nonce.is_empty()
            || !super::verify_nonce(nonce, expected_nonce)
        {
            return Err(ConnectError::Provider(
                "OIDC id_token nonce mismatch".into(),
            ));
        }
    }
    Ok(())
}
