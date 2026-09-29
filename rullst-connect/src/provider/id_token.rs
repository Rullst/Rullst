//! Shared post-signature OIDC claim policy. Never validates an unverified payload.
use jsonwebtoken::{Algorithm, Validation};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{ConnectError, ConnectUser};

/// Longest ID token accepted by the audience-bound verification entry points.
const MAX_ID_TOKEN_BYTES: usize = 16 * 1024;
/// Longest caller-supplied nonce accepted by those entry points.
const MAX_NONCE_BYTES: usize = 512;

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

/// Checks caller-supplied input to a `verify_id_token` entry point before any
/// I/O. A nonce is mandatory there: it is the only binding between the token
/// and the sign-in attempt this server started.
pub(crate) fn validate_verification_input(
    id_token: &str,
    expected_nonce: &str,
) -> Result<(), ConnectError> {
    if id_token.is_empty() || id_token.len() > MAX_ID_TOKEN_BYTES {
        return Err(ConnectError::Token(format!(
            "ID token must contain 1 to {MAX_ID_TOKEN_BYTES} bytes"
        )));
    }
    if expected_nonce.is_empty() || expected_nonce.len() > MAX_NONCE_BYTES {
        return Err(ConnectError::Provider(format!(
            "ID-token verification requires an expected nonce of 1 to {MAX_NONCE_BYTES} bytes"
        )));
    }
    Ok(())
}

/// Deterministic identity for `verify_id_token` under mock credentials.
pub(crate) fn mock_verified_user(
    provider: &str,
    id_token: &str,
) -> Result<ConnectUser, ConnectError> {
    if !cfg!(any(test, feature = "mock")) {
        return Err(ConnectError::Offline(
            "mock credentials are network-free but functional mock identities require the 'mock' feature"
                .to_string(),
        ));
    }
    Ok(ConnectUser {
        id: "mock-user".to_string(),
        name: "Rullst Mock User".to_string(),
        email: Some("mock@example.invalid".to_string()),
        avatar_url: None,
        email_verified: Some(true),
        raw_data: serde_json::json!({ "provider": provider, "mock": true }),
        access_token: secrecy::SecretString::from(id_token.to_owned()),
        refresh_token: None,
        expires_in: None,
    })
}
