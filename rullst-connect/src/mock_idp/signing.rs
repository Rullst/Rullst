use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use ed25519_dalek::{Signer, SigningKey};
use serde::Serialize;

use super::MockIdpUser;

pub(super) const KEY_ID: &str = "rullst-mock-ed25519-v1";

// Public deterministic fixture material. This is intentionally not a secret
// and must never be reused outside the local mock IdP.
const FIXTURE_SIGNING_SEED: [u8; 32] = [0x52; 32];

pub(super) struct MockSigner {
    key: SigningKey,
}

#[derive(Serialize)]
struct IdTokenHeader<'a> {
    alg: &'a str,
    kid: &'a str,
    typ: &'a str,
}

#[derive(Serialize)]
struct IdTokenClaims<'a> {
    iss: &'a str,
    aud: &'a str,
    sub: &'a str,
    name: &'a str,
    email: &'a str,
    email_verified: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    picture: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    nonce: Option<&'a str>,
    iat: u64,
    exp: u64,
}

impl MockSigner {
    pub(super) fn new() -> Self {
        Self {
            key: SigningKey::from_bytes(&FIXTURE_SIGNING_SEED),
        }
    }

    pub(super) fn jwks(&self) -> serde_json::Value {
        serde_json::json!({
            "keys": [{
                "kty": "OKP",
                "use": "sig",
                "crv": "Ed25519",
                "kid": KEY_ID,
                "alg": "EdDSA",
                "x": URL_SAFE_NO_PAD.encode(self.key.verifying_key().to_bytes())
            }]
        })
    }

    pub(super) fn sign_id_token(
        &self,
        issuer: &str,
        audience: &str,
        user: &MockIdpUser,
        nonce: Option<&str>,
        issued_at: u64,
        expires_at: u64,
    ) -> Result<String, crate::ConnectError> {
        let header = IdTokenHeader {
            alg: "EdDSA",
            kid: KEY_ID,
            typ: "JWT",
        };
        let claims = IdTokenClaims {
            iss: issuer,
            aud: audience,
            sub: &user.subject,
            name: &user.name,
            email: &user.email,
            email_verified: true,
            picture: user.picture.as_deref(),
            nonce,
            iat: issued_at,
            exp: expires_at,
        };
        let header = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&header)?);
        let claims = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims)?);
        let signing_input = format!("{header}.{claims}");
        let signature = self.key.sign(signing_input.as_bytes());
        Ok(format!(
            "{signing_input}.{}",
            URL_SAFE_NO_PAD.encode(signature.to_bytes())
        ))
    }
}
