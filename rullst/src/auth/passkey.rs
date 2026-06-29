use base64::Engine as _;
use rand::distr::{Alphanumeric, SampleString};
use ring::signature;
use sha2::Digest;

/// Configuration for the WebAuthn/Passkey authentication manager.
/// Adheres to backward-compatibility guidelines via `#[non_exhaustive]`
/// and the fluent builder pattern.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct PasskeyConfig {
    /// Human-readable name of the Relying Party displayed to the user during registration (e.g. `"My App"`).
    pub rp_name: String,
    /// The effective domain of the Relying Party used to scope the credential (e.g. `"example.com"`).
    /// Must match the origin's registrable domain suffix.
    pub rp_id: String,
    /// The full origin URL of the Relying Party (e.g. `"https://example.com"`).
    /// Used to verify the `clientDataJSON.origin` during assertion.
    pub rp_origin: String,
}

impl PasskeyConfig {
    /// Creates a new `PasskeyConfig` with mandatory fields.
    pub fn new(
        rp_name: impl Into<String>,
        rp_id: impl Into<String>,
        rp_origin: impl Into<String>,
    ) -> Self {
        Self {
            rp_name: rp_name.into(),
            rp_id: rp_id.into(),
            rp_origin: rp_origin.into(),
        }
    }

    /// Builder helper to set or override the Relying Party name.
    pub fn with_rp_name(mut self, rp_name: impl Into<String>) -> Self {
        self.rp_name = rp_name.into();
        self
    }

    /// Builder helper to set or override the Relying Party ID.
    pub fn with_rp_id(mut self, rp_id: impl Into<String>) -> Self {
        self.rp_id = rp_id.into();
        self
    }

    /// Builder helper to set or override the Relying Party Origin.
    pub fn with_rp_origin(mut self, rp_origin: impl Into<String>) -> Self {
        self.rp_origin = rp_origin.into();
        self
    }
}

/// The core passkey authentication service in Rullst.
/// Written in 100% pure Rust using `ring` to avoid external OpenSSL native runtime dependencies.
#[derive(Clone)]
pub struct PasskeyAuth {
    rp_name: String,
    rp_id: String,
    rp_origin: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
/// The top-level response sent to the browser to begin a WebAuthn credential registration ceremony.
/// Wraps [`PublicKeyCredentialCreationOptions`] under the `publicKey` JSON key as required by the W3C spec.
pub struct CreationChallengeResponse {
    #[serde(rename = "publicKey")]
    /// The full set of options passed to `navigator.credentials.create()` on the client.
    pub public_key: PublicKeyCredentialCreationOptions,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
/// Options passed to `navigator.credentials.create()` to register a new public-key credential.
/// Serialized under the `publicKey` field of the JSON challenge response.
pub struct PublicKeyCredentialCreationOptions {
    /// Base64url-encoded random challenge used to prevent replay attacks.
    pub challenge: String,
    /// Information about the Relying Party (name and ID).
    pub rp: RelyingPartyInfo,
    /// Information about the user account being registered.
    pub user: UserInfo,
    #[serde(rename = "pubKeyCredParams")]
    /// Ordered list of supported credential types and cryptographic algorithms.
    pub pub_key_cred_params: Vec<PubKeyCredParam>,
    /// Maximum time (in milliseconds) that the browser should wait for the user to respond.
    pub timeout: u32,
    #[serde(rename = "authenticatorSelection")]
    /// Constraints on the authenticator used for credential creation.
    pub authenticator_selection: AuthenticatorSelection,
    /// Attestation conveyance preference (`"none"`, `"indirect"`, or `"direct"`).
    pub attestation: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
/// Identifies the Relying Party to the authenticator.
pub struct RelyingPartyInfo {
    /// Human-readable Relying Party name shown to the user (e.g. `"My App"`).
    pub name: String,
    /// Effective domain that scopes the credential (e.g. `"example.com"`).
    pub id: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
/// Information about the user account being registered. Used by the authenticator to personalize UX.
pub struct UserInfo {
    /// Base64url-encoded unique user handle (opaque identifier, must not contain PII).
    pub id: String,
    /// Username used for account disambiguation (may be shown to the user).
    pub name: String,
    #[serde(rename = "displayName")]
    /// Human-readable display name for the account (e.g. full name).
    pub display_name: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
/// Describes a supported public-key credential type and its cryptographic algorithm.
pub struct PubKeyCredParam {
    /// The credential type — always `"public-key"` per the WebAuthn spec.
    pub r#type: String,
    /// COSE algorithm identifier (e.g. `-7` for ES256, `-257` for RS256).
    pub alg: i32,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
/// Constraints that restrict which authenticators may be used during credential creation.
pub struct AuthenticatorSelection {
    #[serde(rename = "userVerification")]
    /// User verification requirement: `"required"`, `"preferred"`, or `"discouraged"`.
    pub user_verification: String,
    #[serde(rename = "residentKey")]
    /// Resident (discoverable) key requirement: `"required"`, `"preferred"`, or `"discouraged"`.
    pub resident_key: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
/// The top-level response sent to the browser to begin a WebAuthn authentication assertion ceremony.
/// Wraps [`PublicKeyCredentialRequestOptions`] under the `publicKey` JSON key.
pub struct RequestChallengeResponse {
    #[serde(rename = "publicKey")]
    /// The full set of options passed to `navigator.credentials.get()` on the client.
    pub public_key: PublicKeyCredentialRequestOptions,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
/// Options passed to `navigator.credentials.get()` during a WebAuthn authentication ceremony.
pub struct PublicKeyCredentialRequestOptions {
    /// Base64url-encoded random challenge to prevent replay attacks.
    pub challenge: String,
    /// Maximum time (in milliseconds) for the browser to wait for user interaction.
    pub timeout: u32,
    #[serde(rename = "rpId")]
    /// The Relying Party ID that scopes valid credentials for this request.
    pub rp_id: String,
    #[serde(rename = "allowCredentials")]
    /// List of credentials that are acceptable for this authentication ceremony.
    pub allow_credentials: Vec<AllowCredential>,
    #[serde(rename = "userVerification")]
    /// User verification requirement: `"required"`, `"preferred"`, or `"discouraged"`.
    pub user_verification: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
/// A specific credential that is allowed for the current authentication ceremony.
pub struct AllowCredential {
    /// The credential type — always `"public-key"`.
    pub r#type: String,
    /// Base64url-encoded credential ID previously returned during registration.
    pub id: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
/// The credential object submitted by the browser during passkey **registration**.
/// Mirrors the `PublicKeyCredential` interface returned by `navigator.credentials.create()`.
pub struct RegisterPublicKeyCredential {
    /// Base64url-encoded credential ID assigned by the authenticator.
    pub id: String,
    #[serde(rename = "rawId")]
    /// The raw binary credential ID, also base64url-encoded.
    pub raw_id: String,
    /// The credential type — always `"public-key"`.
    pub r#type: String,
    /// The authenticator's attestation response containing the public key and attestation object.
    pub response: AuthenticatorAttestationResponse,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
/// The authenticator's response to a registration request.
/// Contains the CBOR-encoded attestation object and the client data JSON.
pub struct AuthenticatorAttestationResponse {
    #[serde(rename = "attestationObject")]
    /// Base64url-encoded CBOR attestation object containing authData, fmt, and attStmt.
    pub attestation_object: String,
    #[serde(rename = "clientDataJSON")]
    /// Base64url-encoded JSON string describing the client context (origin, challenge, type).
    pub client_data_json: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
/// The credential object submitted by the browser during passkey **authentication**.
/// Mirrors the `PublicKeyCredential` interface returned by `navigator.credentials.get()`.
pub struct PublicKeyCredential {
    /// Base64url-encoded credential ID that matches the registered credential.
    pub id: String,
    #[serde(rename = "rawId")]
    /// The raw binary credential ID, also base64url-encoded.
    pub raw_id: String,
    /// The credential type — always `"public-key"`.
    pub r#type: String,
    /// The authenticator's assertion response containing the signed data.
    pub response: AuthenticatorAssertionResponse,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
/// The authenticator's response to an authentication assertion request.
/// Contains the signed authenticator data and the client data JSON.
pub struct AuthenticatorAssertionResponse {
    #[serde(rename = "authenticatorData")]
    /// Base64url-encoded authenticator data (rpIdHash, flags, signCount).
    pub authenticator_data: String,
    #[serde(rename = "clientDataJSON")]
    /// Base64url-encoded client data JSON (origin, challenge, type).
    pub client_data_json: String,
    /// Base64url-encoded ECDSA or RSA signature over (authenticatorData || hash(clientDataJSON)).
    pub signature: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
/// A stored passkey credential associated with a user account.
/// Persisted in the database after successful WebAuthn registration.
pub struct Passkey {
    /// The credential ID returned by the authenticator, used to match credentials during authentication.
    pub credential_id: Vec<u8>,
    /// The raw DER-encoded COSE public key extracted from the attestation object.
    pub public_key: Vec<u8>,
    /// Monotonically increasing signature counter used to detect authenticator cloning.
    pub sign_count: u32,
}

// Custom lightweight CBOR parser for WebAuthn payload decoding
#[derive(Debug, Clone)]
#[allow(dead_code)] // Array variant retained for spec completeness; may be used by future attestation formats
enum CborValue {
    Integer(i64),
    ByteString(Vec<u8>),
    TextString(String),
    Array(Vec<CborValue>),
    Map(std::collections::HashMap<CborKey, CborValue>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum CborKey {
    Integer(i64),
    TextString(String),
}

#[cfg_attr(mutants, mutants::skip)]
fn parse_cbor(bytes: &[u8]) -> Result<(CborValue, &[u8]), String> {
    if bytes.is_empty() {
        return Err("Unexpected EOF".to_string());
    }
    let head = bytes[0];
    let major = head >> 5;
    let info = head & 0x1F;
    let rest = &bytes[1..];

    let (val, rest) = match info {
        0..=23 => (info as u64, rest),
        24 => {
            if rest.is_empty() {
                return Err("Unexpected EOF".to_string());
            }
            (rest[0] as u64, &rest[1..])
        }
        25 => {
            if rest.len() < 2 {
                return Err("Unexpected EOF".to_string());
            }
            (u16::from_be_bytes([rest[0], rest[1]]) as u64, &rest[2..])
        }
        26 => {
            if rest.len() < 4 {
                return Err("Unexpected EOF".to_string());
            }
            (
                u32::from_be_bytes([rest[0], rest[1], rest[2], rest[3]]) as u64,
                &rest[4..],
            )
        }
        27 => {
            if rest.len() < 8 {
                return Err("Unexpected EOF".to_string());
            }
            (
                u64::from_be_bytes([
                    rest[0], rest[1], rest[2], rest[3], rest[4], rest[5], rest[6], rest[7],
                ]),
                &rest[8..],
            )
        }
        _ => return Err(format!("Unsupported CBOR info: {}", info)),
    };

    match major {
        0 => Ok((CborValue::Integer(val as i64), rest)),
        1 => Ok((CborValue::Integer(-(val as i64) - 1), rest)),
        2 => {
            if rest.len() < val as usize {
                return Err("Unexpected EOF in byte string".to_string());
            }
            Ok((
                CborValue::ByteString(rest[..val as usize].to_vec()),
                &rest[val as usize..],
            ))
        }
        3 => {
            if rest.len() < val as usize {
                return Err("Unexpected EOF in text string".to_string());
            }
            let s = String::from_utf8(rest[..val as usize].to_vec())
                .map_err(|e| format!("Invalid UTF-8: {}", e))?;
            Ok((CborValue::TextString(s), &rest[val as usize..]))
        }
        4 => {
            let mut items = Vec::new();
            let mut current = rest;
            for _ in 0..val {
                let (item, next) = parse_cbor(current)?;
                items.push(item);
                current = next;
            }
            Ok((CborValue::Array(items), current))
        }
        5 => {
            let mut map = std::collections::HashMap::new();
            let mut current = rest;
            for _ in 0..val {
                let (key_val, next) = parse_cbor(current)?;
                let (val_val, next2) = parse_cbor(next)?;
                let key = match key_val {
                    CborValue::Integer(i) => CborKey::Integer(i),
                    CborValue::TextString(s) => CborKey::TextString(s),
                    _ => return Err("Invalid CBOR map key".to_string()),
                };
                map.insert(key, val_val);
                current = next2;
            }
            Ok((CborValue::Map(map), current))
        }
        _ => Err(format!("Unsupported CBOR major type: {}", major)),
    }
}

fn generate_challenge() -> String {
    Alphanumeric.sample_string(&mut rand::rng(), 32)
}

impl PasskeyAuth {
    /// Instantiates the WebAuthn manager using the approved config builder.
    pub fn new(config: &PasskeyConfig) -> Result<Self, String> {
        Ok(Self {
            rp_name: config.rp_name.clone(),
            rp_id: config.rp_id.clone(),
            rp_origin: config.rp_origin.clone(),
        })
    }

    /// Starts a new Passkey registration flow.
    /// Generates challenge options for the browser, and the associated registration challenge.
    pub fn start_register(
        &self,
        user_id: i32,
        username: &str,
        display_name: &str,
    ) -> Result<(CreationChallengeResponse, String), String> {
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
                    alg: -7, // ES256 (ECDSA P-256 w/ SHA-256)
                }],
                timeout: 60000,
                authenticator_selection: AuthenticatorSelection {
                    user_verification: "preferred".to_string(),
                    resident_key: "preferred".to_string(),
                },
                attestation: "none".to_string(),
            },
        };

        Ok((options, challenge))
    }

    /// Verifies the attestation response sent by the browser to complete passkey registration.
    /// Returns the verified cryptographic `Passkey` details to save in the database.
    pub fn finish_register(
        &self,
        credential: &RegisterPublicKeyCredential,
        expected_challenge: &str,
    ) -> Result<Passkey, String> {
        let client_data_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(&credential.response.client_data_json)
            .map_err(|e| format!("Failed to decode clientDataJSON: {}", e))?;

        let client_data: serde_json::Value = serde_json::from_slice(&client_data_bytes)
            .map_err(|e| format!("Failed to parse clientDataJSON: {}", e))?;

        let challenge = client_data
            .get("challenge")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Challenge missing in clientDataJSON".to_string())?;

        if challenge != expected_challenge {
            return Err("Challenge mismatch".to_string());
        }

        let origin = client_data
            .get("origin")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Origin missing in clientDataJSON".to_string())?;

        if origin != self.rp_origin {
            return Err("Origin mismatch".to_string());
        }

        let attestation_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(&credential.response.attestation_object)
            .map_err(|e| format!("Failed to decode attestationObject: {}", e))?;

        let (cbor_obj, _) = parse_cbor(&attestation_bytes)?;
        let auth_data = match cbor_obj {
            CborValue::Map(mut map) => {
                match map.remove(&CborKey::TextString("authData".to_string())) {
                    Some(CborValue::ByteString(bytes)) => bytes,
                    _ => return Err("authData not found in attestationObject".to_string()),
                }
            }
            _ => return Err("attestationObject is not a map".to_string()),
        };

        if auth_data.len() < 55 {
            return Err("authData too short".to_string());
        }

        // Verify rpIdHash matches SHA-256 hash of rp_id
        let mut rp_hasher = sha2::Sha256::new();
        rp_hasher.update(self.rp_id.as_bytes());
        let expected_rp_id_hash = rp_hasher.finalize();
        if auth_data[..32] != expected_rp_id_hash[..] {
            return Err("rpIdHash mismatch in authData".to_string());
        }

        let flags = auth_data[32];
        let has_attested_credential_data = (flags & 0x40) != 0;
        if !has_attested_credential_data {
            return Err("No attested credential data present in authData".to_string());
        }

        let credential_id_len = u16::from_be_bytes([auth_data[53], auth_data[54]]) as usize;
        if auth_data.len() < 55 + credential_id_len {
            return Err("authData too short for credential ID".to_string());
        }
        let credential_id = auth_data[55..55 + credential_id_len].to_vec();
        let cose_key_bytes = &auth_data[55 + credential_id_len..];

        let (cose_key, _) = parse_cbor(cose_key_bytes)?;
        let public_key = match cose_key {
            CborValue::Map(mut map) => {
                let x_bytes = match map.remove(&CborKey::Integer(-2)) {
                    Some(CborValue::ByteString(bytes)) => bytes,
                    _ => return Err("X coordinate not found in public key".to_string()),
                };
                let y_bytes = match map.remove(&CborKey::Integer(-3)) {
                    Some(CborValue::ByteString(bytes)) => bytes,
                    _ => return Err("Y coordinate not found in public key".to_string()),
                };

                let mut key = vec![0x04];
                key.extend_from_slice(&x_bytes);
                key.extend_from_slice(&y_bytes);
                key
            }
            _ => return Err("credentialPublicKey is not a CBOR map".to_string()),
        };

        Ok(Passkey {
            credential_id,
            public_key,
            sign_count: 0,
        })
    }

    /// Starts a passwordless authentication flow.
    /// Generates a verification challenge options block and the assertion challenge.
    pub fn start_authenticate(
        &self,
        allowed_credentials: &[Passkey],
    ) -> Result<(RequestChallengeResponse, String), String> {
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
    pub fn finish_authenticate(
        &self,
        credential: &PublicKeyCredential,
        expected_challenge: &str,
        mut passkey: Passkey,
    ) -> Result<Passkey, String> {
        let client_data_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(&credential.response.client_data_json)
            .map_err(|e| format!("Failed to decode clientDataJSON: {}", e))?;

        let client_data: serde_json::Value = serde_json::from_slice(&client_data_bytes)
            .map_err(|e| format!("Failed to parse clientDataJSON: {}", e))?;

        let challenge = client_data
            .get("challenge")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Challenge missing in clientDataJSON".to_string())?;

        if challenge != expected_challenge {
            return Err("Challenge mismatch".to_string());
        }

        let origin = client_data
            .get("origin")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Origin missing in clientDataJSON".to_string())?;

        if origin != self.rp_origin {
            return Err("Origin mismatch".to_string());
        }

        let auth_data_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(&credential.response.authenticator_data)
            .map_err(|e| format!("Failed to decode authenticatorData: {}", e))?;

        if auth_data_bytes.len() < 37 {
            return Err("authenticatorData too short".to_string());
        }

        // Verify rpIdHash matches SHA-256 hash of rp_id
        let mut rp_hasher = sha2::Sha256::new();
        rp_hasher.update(self.rp_id.as_bytes());
        let expected_rp_id_hash = rp_hasher.finalize();
        if auth_data_bytes[..32] != expected_rp_id_hash[..] {
            return Err("rpIdHash mismatch in authenticatorData".to_string());
        }

        let signature_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(&credential.response.signature)
            .map_err(|e| format!("Failed to decode signature: {}", e))?;

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
            .map_err(|e| format!("ECDSA P-256 signature verification failed: {:?}", e))?;

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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_passkey_config_builder() {
        let config = PasskeyConfig::new("RP", "rp.com", "https://rp.com")
            .with_rp_name("New RP")
            .with_rp_id("new.rp.com");

        assert_eq!(config.rp_name, "New RP");
        assert_eq!(config.rp_id, "new.rp.com");
        assert_eq!(config.rp_origin, "https://rp.com");
    }

    #[test]
    fn test_passkey_auth_start_register() {
        let config = PasskeyConfig::new("App", "app.com", "https://app.com");
        let auth = PasskeyAuth::new(&config).unwrap();

        let (response, challenge) = auth.start_register(1, "alice", "Alice Smith").unwrap();

        assert_eq!(response.public_key.user.name, "alice");
        assert_eq!(response.public_key.user.display_name, "Alice Smith");
        assert_eq!(response.public_key.rp.id, "app.com");
        assert!(!challenge.is_empty());
    }

    #[test]
    fn test_passkey_auth_start_authenticate() {
        let config = PasskeyConfig::new("App", "app.com", "https://app.com");
        let auth = PasskeyAuth::new(&config).unwrap();

        let passkey = Passkey {
            credential_id: vec![1, 2, 3],
            public_key: vec![4, 5, 6],
            sign_count: 0,
        };

        let (response, challenge) = auth.start_authenticate(&[passkey]).unwrap();

        assert_eq!(response.public_key.rp_id, "app.com");
        assert_eq!(response.public_key.allow_credentials.len(), 1);
        assert!(!challenge.is_empty());
    }

    #[test]
    fn test_parse_cbor_integer() {
        // info < 24
        let (val, _) = parse_cbor(&[0x05]).unwrap();
        if let CborValue::Integer(i) = val {
            assert_eq!(i, 5);
        } else {
            panic!();
        }
        // info 24 (1 byte)
        let (val, _) = parse_cbor(&[0x18, 0x1A]).unwrap();
        if let CborValue::Integer(i) = val {
            assert_eq!(i, 26);
        } else {
            panic!();
        }
        // info 25 (2 bytes)
        let (val, _) = parse_cbor(&[0x19, 0x01, 0x00]).unwrap();
        if let CborValue::Integer(i) = val {
            assert_eq!(i, 256);
        } else {
            panic!();
        }
        // Negative integer
        let (val, _) = parse_cbor(&[0x25]).unwrap();
        if let CborValue::Integer(i) = val {
            assert_eq!(i, -6);
        } else {
            panic!();
        }
    }

    #[test]
    fn test_finish_register_mismatches() {
        let config = PasskeyConfig::new("App", "app.com", "https://app.com");
        let auth = PasskeyAuth::new(&config).unwrap();

        let client_data_json = serde_json::json!({
            "challenge": "correct_challenge",
            "origin": "https://app.com",
            "type": "webauthn.create"
        })
        .to_string();
        let client_data_b64 =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(client_data_json);

        let mut cred = RegisterPublicKeyCredential {
            id: "id".into(),
            raw_id: "raw".into(),
            r#type: "public-key".into(),
            response: AuthenticatorAttestationResponse {
                attestation_object: "dummy".into(),
                client_data_json: client_data_b64.clone(),
            },
        };

        // Challenge mismatch
        let res = auth.finish_register(&cred, "wrong_challenge");
        assert_eq!(res.unwrap_err(), "Challenge mismatch");

        // Origin mismatch
        let bad_origin_json = serde_json::json!({
            "challenge": "correct_challenge",
            "origin": "https://wrong.com",
            "type": "webauthn.create"
        })
        .to_string();
        cred.response.client_data_json =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bad_origin_json);

        let res2 = auth.finish_register(&cred, "correct_challenge");
        assert_eq!(res2.unwrap_err(), "Origin mismatch");
    }

    #[test]
    fn test_finish_authenticate_mismatches() {
        let config = PasskeyConfig::new("App", "app.com", "https://app.com");
        let auth = PasskeyAuth::new(&config).unwrap();

        let client_data_json = serde_json::json!({
            "challenge": "correct_challenge",
            "origin": "https://app.com",
            "type": "webauthn.get"
        })
        .to_string();
        let client_data_b64 =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(client_data_json);

        let mut cred = PublicKeyCredential {
            id: "id".into(),
            raw_id: "raw".into(),
            r#type: "public-key".into(),
            response: AuthenticatorAssertionResponse {
                authenticator_data: "dummy".into(),
                signature: "dummy".into(),
                client_data_json: client_data_b64.clone(),
            },
        };

        let pk = Passkey {
            credential_id: vec![],
            public_key: vec![],
            sign_count: 0,
        };

        // Challenge mismatch
        let res = auth.finish_authenticate(&cred, "wrong_challenge", pk.clone());
        assert_eq!(res.unwrap_err(), "Challenge mismatch");

        // Origin mismatch
        let bad_origin_json = serde_json::json!({
            "challenge": "correct_challenge",
            "origin": "https://wrong.com",
            "type": "webauthn.get"
        })
        .to_string();
        cred.response.client_data_json =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bad_origin_json);

        let res2 = auth.finish_authenticate(&cred, "correct_challenge", pk);
        assert_eq!(res2.unwrap_err(), "Origin mismatch");
    }
    #[test]
    fn test_parse_cbor_byte_string() {
        let (val, rest) = parse_cbor(&[0x44, 0x01, 0x02, 0x03, 0x04]).unwrap();
        if let CborValue::ByteString(b) = val {
            assert_eq!(b, vec![1, 2, 3, 4]);
        } else {
            panic!();
        }
        assert!(rest.is_empty());

        // Unexpected EOF
        let res = parse_cbor(&[0x44, 0x01, 0x02]);
        assert!(res.is_err());
    }

    #[test]
    fn test_parse_cbor_text_string() {
        let (val, rest) = parse_cbor(&[0x63, b'a', b'b', b'c']).unwrap();
        if let CborValue::TextString(s) = val {
            assert_eq!(s, "abc");
        } else {
            panic!();
        }
        assert!(rest.is_empty());

        // Unexpected EOF
        let res = parse_cbor(&[0x63, b'a']);
        assert!(res.is_err());
    }

    #[test]
    fn test_parse_cbor_array() {
        // [1, 2] -> 0x82, 0x01, 0x02
        let (val, rest) = parse_cbor(&[0x82, 0x01, 0x02]).unwrap();
        if let CborValue::Array(arr) = val {
            assert_eq!(arr.len(), 2);
        } else {
            panic!();
        }
        assert!(rest.is_empty());
    }

    #[test]
    fn test_parse_cbor_map() {
        // {"a": 1} -> 0xA1, 0x61, b'a', 0x01
        let (val, rest) = parse_cbor(&[0xA1, 0x61, b'a', 0x01]).unwrap();
        if let CborValue::Map(map) = val {
            assert_eq!(map.len(), 1);
        } else {
            panic!();
        }
        assert!(rest.is_empty());
    }

    #[test]
    fn test_parse_cbor_invalid_types() {
        // Empty bytes
        let res = parse_cbor(&[]);
        assert!(res.is_err());

        // Info 24 with EOF
        let res = parse_cbor(&[0x18]);
        assert!(res.is_err());

        // Info 25 with 1 byte (EOF)
        let res = parse_cbor(&[0x19, 0x01]);
        assert!(res.is_err());

        // Info 26 with 3 bytes (EOF)
        let res = parse_cbor(&[0x1A, 0x01, 0x02, 0x03]);
        assert!(res.is_err());

        // Info 27 with 7 bytes (EOF)
        let res = parse_cbor(&[0x1B, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07]);
        assert!(res.is_err());

        // Unsupported info
        let res = parse_cbor(&[0x1C]);
        assert!(res.is_err());

        // Unsupported major type (e.g., 6) -> 0xC0
        let res = parse_cbor(&[0xC0]);
        assert!(res.is_err());
    }

    #[test]
    fn test_finish_register_edge_cases() {
        let config = PasskeyConfig::new("App", "app.com", "https://app.com");
        let auth = PasskeyAuth::new(&config).unwrap();

        let client_data_json = serde_json::json!({
            "challenge": "correct_challenge",
            "origin": "https://app.com",
            "type": "webauthn.create"
        })
        .to_string();
        let client_data_b64 =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(client_data_json);

        let mut cred = RegisterPublicKeyCredential {
            id: "id".into(),
            raw_id: "raw".into(),
            r#type: "public-key".into(),
            response: AuthenticatorAttestationResponse {
                attestation_object: "dummy".into(),
                client_data_json: client_data_b64.clone(),
            },
        };

        // 1. Invalid base64 in attestationObject
        cred.response.attestation_object = "invalid base64!!!".into();
        assert!(
            auth.finish_register(&cred, "correct_challenge")
                .unwrap_err()
                .contains("Failed to decode attestationObject")
        );

        // 2. attestationObject not a map
        // CBOR Array instead of Map: [1, 2] -> 0x82, 0x01, 0x02
        cred.response.attestation_object =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([0x82, 0x01, 0x02]);
        assert_eq!(
            auth.finish_register(&cred, "correct_challenge")
                .unwrap_err(),
            "attestationObject is not a map"
        );

        // 3. authData not found in map
        // Map with one string "test" -> "test"
        cred.response.attestation_object =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([
                0xA1, 0x64, b't', b'e', b's', b't', 0x64, b't', b'e', b's', b't',
            ]);
        assert_eq!(
            auth.finish_register(&cred, "correct_challenge")
                .unwrap_err(),
            "authData not found in attestationObject"
        );

        // 4. authData too short
        let mut map_with_short_authdata = vec![0xA1, 0x68];
        map_with_short_authdata.extend_from_slice(b"authData");
        map_with_short_authdata.extend_from_slice(&[0x44, 1, 2, 3, 4]); // Byte string of length 4
        cred.response.attestation_object =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&map_with_short_authdata);
        assert_eq!(
            auth.finish_register(&cred, "correct_challenge")
                .unwrap_err(),
            "authData too short"
        );

        // 5. rpIdHash mismatch
        let mut auth_data_bytes = vec![0u8; 60];
        // Fill first 32 bytes with garbage (not the hash of "app.com")
        auth_data_bytes[0] = 0xFF;

        let mut map_with_bad_hash = vec![0xA1, 0x68];
        map_with_bad_hash.extend_from_slice(b"authData");
        map_with_bad_hash.push(0x58);
        map_with_bad_hash.push(auth_data_bytes.len() as u8);
        map_with_bad_hash.extend_from_slice(&auth_data_bytes);

        cred.response.attestation_object =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&map_with_bad_hash);
        assert_eq!(
            auth.finish_register(&cred, "correct_challenge")
                .unwrap_err(),
            "rpIdHash mismatch in authData"
        );

        // 6. No attested credential data
        let mut rp_hasher = sha2::Sha256::new();
        rp_hasher.update(b"app.com");
        let expected_rp_id_hash = rp_hasher.finalize();
        auth_data_bytes[..32].copy_from_slice(&expected_rp_id_hash);
        auth_data_bytes[32] = 0x00; // flags without AT (0x40)

        let mut map_with_no_at = vec![0xA1, 0x68];
        map_with_no_at.extend_from_slice(b"authData");
        map_with_no_at.push(0x58);
        map_with_no_at.push(auth_data_bytes.len() as u8);
        map_with_no_at.extend_from_slice(&auth_data_bytes);
        cred.response.attestation_object =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&map_with_no_at);
        assert_eq!(
            auth.finish_register(&cred, "correct_challenge")
                .unwrap_err(),
            "No attested credential data present in authData"
        );

        // 7. authData too short for credential ID
        auth_data_bytes[32] = 0x40; // flags with AT (0x40)
        auth_data_bytes[53] = 0x00;
        auth_data_bytes[54] = 0xFF; // Expects 255 bytes of credential ID but we only have 60 total

        let mut map_with_short_id = vec![0xA1, 0x68];
        map_with_short_id.extend_from_slice(b"authData");
        map_with_short_id.push(0x58);
        map_with_short_id.push(auth_data_bytes.len() as u8);
        map_with_short_id.extend_from_slice(&auth_data_bytes);
        cred.response.attestation_object =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&map_with_short_id);
        assert_eq!(
            auth.finish_register(&cred, "correct_challenge")
                .unwrap_err(),
            "authData too short for credential ID"
        );
    }

    #[test]
    fn test_finish_authenticate_edge_cases() {
        let config = PasskeyConfig::new("App", "app.com", "https://app.com");
        let auth = PasskeyAuth::new(&config).unwrap();

        let client_data_json = serde_json::json!({
            "challenge": "correct_challenge",
            "origin": "https://app.com",
            "type": "webauthn.get"
        })
        .to_string();
        let client_data_b64 =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(client_data_json);

        let passkey = Passkey {
            credential_id: vec![],
            public_key: vec![],
            sign_count: 0,
        };

        let mut cred = PublicKeyCredential {
            id: "id".into(),
            raw_id: "raw".into(),
            r#type: "public-key".into(),
            response: AuthenticatorAssertionResponse {
                authenticator_data: "dummy".into(),
                signature: "dummy".into(),
                client_data_json: client_data_b64.clone(),
            },
        };

        // 1. Invalid base64 auth data
        cred.response.authenticator_data = "invalid base 64!!!".into();
        assert!(
            auth.finish_authenticate(&cred, "correct_challenge", passkey.clone())
                .unwrap_err()
                .contains("Failed to decode authenticatorData")
        );

        // 2. Auth data too short
        cred.response.authenticator_data =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([0x01, 0x02]);
        assert_eq!(
            auth.finish_authenticate(&cred, "correct_challenge", passkey.clone())
                .unwrap_err(),
            "authenticatorData too short"
        );

        // 3. rpIdHash mismatch
        let mut auth_data_bytes = vec![0u8; 37];
        cred.response.authenticator_data =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&auth_data_bytes);
        assert_eq!(
            auth.finish_authenticate(&cred, "correct_challenge", passkey.clone())
                .unwrap_err(),
            "rpIdHash mismatch in authenticatorData"
        );

        // 4. Invalid base64 signature
        let mut rp_hasher = sha2::Sha256::new();
        rp_hasher.update(b"app.com");
        auth_data_bytes[..32].copy_from_slice(&rp_hasher.finalize());
        cred.response.authenticator_data =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&auth_data_bytes);
        cred.response.signature = "invalid sig!!!".into();
        assert!(
            auth.finish_authenticate(&cred, "correct_challenge", passkey.clone())
                .unwrap_err()
                .contains("Failed to decode signature")
        );

        // 5. ECDSA verification failed (bad signature)
        cred.response.signature =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([0x00; 64]);
        assert!(
            auth.finish_authenticate(&cred, "correct_challenge", passkey.clone())
                .unwrap_err()
                .contains("ECDSA P-256 signature verification failed")
        );
    }
}
