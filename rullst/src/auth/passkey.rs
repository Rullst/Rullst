use base64::Engine as _;
use rand::distr::{Alphanumeric, SampleString};
#[allow(dead_code)]
use ring::signature;
use sha2::Digest;

/// Configuration for the WebAuthn/Passkey authentication manager.
/// Adheres to backward-compatibility guidelines via `#[non_exhaustive]`
/// and the fluent builder pattern.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct PasskeyConfig {
    pub rp_name: String,
    pub rp_id: String,
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
    #[allow(dead_code)]
    rp_origin: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct CreationChallengeResponse {
    #[serde(rename = "publicKey")]
    pub public_key: PublicKeyCredentialCreationOptions,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct PublicKeyCredentialCreationOptions {
    pub challenge: String,
    pub rp: RelyingPartyInfo,
    pub user: UserInfo,
    #[serde(rename = "pubKeyCredParams")]
    pub pub_key_cred_params: Vec<PubKeyCredParam>,
    pub timeout: u32,
    #[serde(rename = "authenticatorSelection")]
    pub authenticator_selection: AuthenticatorSelection,
    pub attestation: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct RelyingPartyInfo {
    pub name: String,
    pub id: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct UserInfo {
    pub id: String,
    pub name: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct PubKeyCredParam {
    pub r#type: String,
    pub alg: i32,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct AuthenticatorSelection {
    #[serde(rename = "userVerification")]
    pub user_verification: String,
    #[serde(rename = "residentKey")]
    pub resident_key: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct RequestChallengeResponse {
    #[serde(rename = "publicKey")]
    pub public_key: PublicKeyCredentialRequestOptions,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct PublicKeyCredentialRequestOptions {
    pub challenge: String,
    pub timeout: u32,
    #[serde(rename = "rpId")]
    pub rp_id: String,
    #[serde(rename = "allowCredentials")]
    pub allow_credentials: Vec<AllowCredential>,
    #[serde(rename = "userVerification")]
    pub user_verification: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct AllowCredential {
    pub r#type: String,
    pub id: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct RegisterPublicKeyCredential {
    pub id: String,
    #[serde(rename = "rawId")]
    pub raw_id: String,
    pub r#type: String,
    pub response: AuthenticatorAttestationResponse,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct AuthenticatorAttestationResponse {
    #[serde(rename = "attestationObject")]
    pub attestation_object: String,
    #[serde(rename = "clientDataJSON")]
    pub client_data_json: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct PublicKeyCredential {
    pub id: String,
    #[serde(rename = "rawId")]
    pub raw_id: String,
    pub r#type: String,
    pub response: AuthenticatorAssertionResponse,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct AuthenticatorAssertionResponse {
    #[serde(rename = "authenticatorData")]
    pub authenticator_data: String,
    #[serde(rename = "clientDataJSON")]
    pub client_data_json: String,
    pub signature: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Passkey {
    pub credential_id: Vec<u8>,
    pub public_key: Vec<u8>,
    pub sign_count: u32,
}

// Custom lightweight CBOR parser for WebAuthn payload decoding
#[allow(dead_code)]
#[derive(Debug, Clone)]
enum CborValue {
    Integer(i64),
    ByteString(Vec<u8>),
    TextString(String),
    Array(Vec<CborValue>),
    Map(std::collections::HashMap<CborKey, CborValue>),
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum CborKey {
    Integer(i64),
    TextString(String),
}

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

        let auth_data_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(&credential.response.authenticator_data)
            .map_err(|e| format!("Failed to decode authenticatorData: {}", e))?;

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
