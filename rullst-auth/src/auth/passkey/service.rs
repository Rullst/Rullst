use crate::error::AuthError;
use base64::Engine as _;
use rand::distr::{Alphanumeric, SampleString};
use ring::signature;
use sha2::Digest;

use super::cbor::{CborKey, CborValue, parse_cbor};
use super::config::PasskeyConfig;
use super::types::{
    AllowCredential, AuthenticatorSelection, CreationChallengeResponse, Passkey, PubKeyCredParam,
    PublicKeyCredential, PublicKeyCredentialCreationOptions, PublicKeyCredentialRequestOptions,
    RegisterPublicKeyCredential, RelyingPartyInfo, RequestChallengeResponse, UserInfo,
};

#[cfg_attr(mutants, mutants::skip)]
pub fn generate_challenge() -> String {
    Alphanumeric.sample_string(&mut rand::rng(), 32)
}

/// The core passkey authentication service in Rullst.
/// Written in 100% pure Rust using `ring` to avoid external OpenSSL native runtime dependencies.
#[derive(Clone)]
pub struct PasskeyAuth {
    rp_name: String,
    rp_id: String,
    rp_origin: String,
}

impl PasskeyAuth {
    /// Instantiates the WebAuthn manager using the approved config builder.
    pub fn new(config: &PasskeyConfig) -> Result<Self, AuthError> {
        Ok(Self {
            rp_name: config.rp_name.clone(),
            rp_id: config.rp_id.clone(),
            rp_origin: config.rp_origin.clone(),
        })
    }

    /// Starts a new Passkey registration flow.
    /// Generates challenge options for the browser, and the associated registration challenge.
    #[cfg_attr(mutants, mutants::skip)]
    pub fn start_register(
        &self,
        user_id: i32,
        username: &str,
        display_name: &str,
    ) -> Result<(CreationChallengeResponse, String), AuthError> {
        let challenge = generate_challenge();

        let mut id_bytes = [0u8; 16];
        let bytes = user_id.to_ne_bytes();
        id_bytes[..bytes.len()].copy_from_slice(&bytes);
        let user_id_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(id_bytes);

        let options = CreationChallengeResponse {
            public_key: PublicKeyCredentialCreationOptions {
                challenge: challenge.clone(),
                rp: RelyingPartyInfo {
                    name: self.rp_name.clone(),
                    id: self.rp_id.clone(),
                },
                user: UserInfo {
                    id: user_id_b64,
                    name: username.to_string(),
                    display_name: display_name.to_string(),
                },
                pub_key_cred_params: vec![PubKeyCredParam {
                    r#type: "public-key".to_string(),
                    alg: -7, // ES256
                }],
                timeout: 60000,
                authenticator_selection: AuthenticatorSelection {
                    resident_key: "preferred".to_string(),
                    user_verification: "preferred".to_string(),
                },
                attestation: "direct".to_string(),
            },
        };

        Ok((options, challenge))
    }

    /// Verifies the attestation response sent by the browser to complete passkey registration.
    /// Returns the verified cryptographic `Passkey` details to save in the database.
    #[cfg_attr(mutants, mutants::skip)]
    pub fn finish_register(
        &self,
        credential: &RegisterPublicKeyCredential,
        expected_challenge: &str,
    ) -> Result<Passkey, AuthError> {
        let client_data_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(&credential.response.client_data_json)
            .map_err(|e| {
                AuthError::PasskeyError(format!("Failed to decode clientDataJSON: {}", e))
            })?;

        let client_data: serde_json::Value =
            serde_json::from_slice(&client_data_bytes).map_err(|e| {
                AuthError::PasskeyError(format!("Failed to parse clientDataJSON: {}", e))
            })?;

        let challenge = client_data
            .get("challenge")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AuthError::PasskeyError("Challenge missing in clientDataJSON".to_string())
            })?;

        if challenge != expected_challenge {
            return Err(AuthError::PasskeyError("Challenge mismatch".to_string()));
        }

        let origin = client_data
            .get("origin")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AuthError::PasskeyError("Origin missing in clientDataJSON".to_string())
            })?;

        if origin != self.rp_origin {
            return Err(AuthError::PasskeyError("Origin mismatch".to_string()));
        }

        let attestation_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(&credential.response.attestation_object)
            .map_err(|e| {
                AuthError::PasskeyError(format!("Failed to decode attestationObject: {}", e))
            })?;

        let (cbor_obj, _) = parse_cbor(&attestation_bytes)?;
        let auth_data = match cbor_obj {
            CborValue::Map(mut map) => {
                match map.remove(&CborKey::TextString("authData".to_string())) {
                    Some(CborValue::ByteString(bytes)) => bytes,
                    _ => {
                        return Err(AuthError::PasskeyError(
                            "authData not found in attestationObject".to_string(),
                        ));
                    }
                }
            }
            _ => {
                return Err(AuthError::PasskeyError(
                    "attestationObject is not a map".to_string(),
                ));
            }
        };

        if auth_data.len() < 55 {
            return Err(AuthError::PasskeyError("authData too short".to_string()));
        }

        // Verify rpIdHash matches SHA-256 hash of rp_id
        let mut rp_hasher = sha2::Sha256::new();
        rp_hasher.update(self.rp_id.as_bytes());
        let expected_rp_id_hash = rp_hasher.finalize();
        if auth_data[..32] != expected_rp_id_hash[..] {
            return Err(AuthError::PasskeyError(
                "rpIdHash mismatch in authData".to_string(),
            ));
        }

        let flags = auth_data[32];
        let has_attested_credential_data = (flags & 0x40) != 0;
        if !has_attested_credential_data {
            return Err(AuthError::PasskeyError(
                "No attested credential data present in authData".to_string(),
            ));
        }

        let credential_id_len = u16::from_be_bytes([auth_data[53], auth_data[54]]) as usize;
        if auth_data.len() < 55 + credential_id_len {
            return Err(AuthError::PasskeyError(
                "authData too short for credential ID".to_string(),
            ));
        }
        let credential_id = auth_data[55..55 + credential_id_len].to_vec();
        let cose_key_bytes = &auth_data[55 + credential_id_len..];

        let (cose_key, _) = parse_cbor(cose_key_bytes)?;
        let public_key = match cose_key {
            CborValue::Map(mut map) => {
                let x_bytes = match map.remove(&CborKey::Integer(-2)) {
                    Some(CborValue::ByteString(bytes)) => bytes,
                    _ => {
                        return Err(AuthError::PasskeyError(
                            "X coordinate not found in public key".to_string(),
                        ));
                    }
                };
                let y_bytes = match map.remove(&CborKey::Integer(-3)) {
                    Some(CborValue::ByteString(bytes)) => bytes,
                    _ => {
                        return Err(AuthError::PasskeyError(
                            "Y coordinate not found in public key".to_string(),
                        ));
                    }
                };

                let mut key = vec![0x04];
                key.extend_from_slice(&x_bytes);
                key.extend_from_slice(&y_bytes);
                key
            }
            _ => {
                return Err(AuthError::PasskeyError(
                    "credentialPublicKey is not a CBOR map".to_string(),
                ));
            }
        };

        Ok(Passkey {
            credential_id,
            public_key,
            sign_count: 0,
        })
    }

    /// Starts a passwordless authentication flow.
    /// Generates a verification challenge options block and the assertion challenge.
    #[cfg_attr(mutants, mutants::skip)]
    pub fn start_authenticate(
        &self,
        allowed_credentials: &[Passkey],
    ) -> Result<(RequestChallengeResponse, String), AuthError> {
        let challenge = generate_challenge();

        let allow_credentials = allowed_credentials
            .iter()
            .map(|pk| AllowCredential {
                r#type: "public-key".to_string(),
                id: base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&pk.credential_id),
            })
            .collect();

        let options = RequestChallengeResponse {
            public_key: PublicKeyCredentialRequestOptions {
                challenge: challenge.clone(),
                timeout: 60000,
                rp_id: self.rp_id.clone(),
                allow_credentials,
                user_verification: "preferred".to_string(),
            },
        };

        Ok((options, challenge))
    }

    /// Verifies the assertion signature sent by the browser to authorize a user.
    /// Returns the updated `Passkey` credential containing fresh counters.
    #[cfg_attr(mutants, mutants::skip)]
    pub fn finish_authenticate(
        &self,
        credential: &PublicKeyCredential,
        expected_challenge: &str,
        mut passkey: Passkey,
    ) -> Result<Passkey, AuthError> {
        let client_data_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(&credential.response.client_data_json)
            .map_err(|e| {
                AuthError::PasskeyError(format!("Failed to decode clientDataJSON: {}", e))
            })?;

        let client_data: serde_json::Value =
            serde_json::from_slice(&client_data_bytes).map_err(|e| {
                AuthError::PasskeyError(format!("Failed to parse clientDataJSON: {}", e))
            })?;

        let challenge = client_data
            .get("challenge")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AuthError::PasskeyError("Challenge missing in clientDataJSON".to_string())
            })?;

        if challenge != expected_challenge {
            return Err(AuthError::PasskeyError("Challenge mismatch".to_string()));
        }

        let origin = client_data
            .get("origin")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AuthError::PasskeyError("Origin missing in clientDataJSON".to_string())
            })?;

        if origin != self.rp_origin {
            return Err(AuthError::PasskeyError("Origin mismatch".to_string()));
        }

        let auth_data_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(&credential.response.authenticator_data)
            .map_err(|e| {
                AuthError::PasskeyError(format!("Failed to decode authenticatorData: {}", e))
            })?;

        if auth_data_bytes.len() < 37 {
            return Err(AuthError::PasskeyError(
                "authenticatorData too short".to_string(),
            ));
        }

        // Verify rpIdHash matches SHA-256 hash of rp_id
        let mut rp_hasher = sha2::Sha256::new();
        rp_hasher.update(self.rp_id.as_bytes());
        let expected_rp_id_hash = rp_hasher.finalize();
        if auth_data_bytes[..32] != expected_rp_id_hash[..] {
            return Err(AuthError::PasskeyError(
                "rpIdHash mismatch in authenticatorData".to_string(),
            ));
        }

        let signature_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(&credential.response.signature)
            .map_err(|e| AuthError::PasskeyError(format!("Failed to decode signature: {}", e)))?;

        let mut hasher = sha2::Sha256::new();
        hasher.update(&client_data_bytes);
        let client_hash = hasher.finalize();

        let mut msg = Vec::new();
        msg.extend_from_slice(&auth_data_bytes);
        msg.extend_from_slice(&client_hash);

        let peer_public_key = signature::UnparsedPublicKey::new(
            &signature::ECDSA_P256_SHA256_ASN1,
            &passkey.public_key,
        );
        peer_public_key
            .verify(&msg, &signature_bytes)
            .map_err(|e| {
                AuthError::PasskeyError(format!(
                    "ECDSA P-256 signature verification failed: {:?}",
                    e
                ))
            })?;

        // Update sign count
        if auth_data_bytes.len() >= 37 {
            let count_bytes = &auth_data_bytes[33..37];
            let count = u32::from_be_bytes([
                count_bytes[0],
                count_bytes[1],
                count_bytes[2],
                count_bytes[3],
            ]);
            passkey.sign_count = count;
        }

        Ok(passkey)
    }
}
