use crate::error::AuthError;
use base64::Engine as _;
use ring::signature;
use serde::Deserialize;
use sha2::Digest;
use std::time::Duration;
use subtle::ConstantTimeEq;
use url::{Host, Url};

use super::cbor::{CborKey, CborValue, parse_cbor};
use super::ceremony::{Ceremony, ChallengeStore};
use super::config::PasskeyConfig;
use super::types::{
    AllowCredential, AuthenticatorSelection, CreationChallengeResponse, Passkey, PubKeyCredParam,
    PublicKeyCredential, PublicKeyCredentialCreationOptions, PublicKeyCredentialRequestOptions,
    RegisterPublicKeyCredential, RelyingPartyInfo, RequestChallengeResponse, UserInfo,
};

const FLAG_USER_PRESENT: u8 = 0x01;
const FLAG_USER_VERIFIED: u8 = 0x04;
const FLAG_BACKUP_ELIGIBLE: u8 = 0x08;
const FLAG_BACKUP_STATE: u8 = 0x10;
const FLAG_ATTESTED_CREDENTIAL_DATA: u8 = 0x40;
const FLAG_EXTENSION_DATA: u8 = 0x80;
const MAX_CREDENTIAL_ID_BYTES: usize = 1_023;

#[derive(Deserialize)]
struct CollectedClientData {
    #[serde(rename = "type")]
    ceremony_type: String,
    challenge: String,
    origin: String,
    #[serde(default, rename = "crossOrigin")]
    cross_origin: bool,
}

fn passkey_error(message: impl Into<String>) -> AuthError {
    AuthError::PasskeyError(message.into())
}

pub use super::ceremony::generate_challenge;

/// WebAuthn manager supporting ES256 credentials and the privacy-preserving `none` attestation
/// format. Challenges are one-time, expire, and are shared by clones of this value.
#[derive(Clone)]
pub struct PasskeyAuth {
    rp_name: String,
    rp_id: String,
    rp_origin: String,
    require_user_verification: bool,
    challenge_ttl: Duration,
    challenges: ChallengeStore,
}

impl PasskeyAuth {
    /// Validates relying-party configuration and creates a WebAuthn manager.
    pub fn new(config: &PasskeyConfig) -> Result<Self, AuthError> {
        if config.rp_name.trim().is_empty() {
            return Err(passkey_error("relying-party name cannot be empty"));
        }
        if config.challenge_ttl_seconds == 0 || config.max_pending_challenges == 0 {
            return Err(passkey_error(
                "challenge TTL and pending-challenge limit must be greater than zero",
            ));
        }

        let parsed_origin = Url::parse(&config.rp_origin)
            .map_err(|error| passkey_error(format!("invalid relying-party origin: {error}")))?;
        if !parsed_origin.username().is_empty()
            || parsed_origin.password().is_some()
            || parsed_origin.query().is_some()
            || parsed_origin.fragment().is_some()
            || parsed_origin.path() != "/"
        {
            return Err(passkey_error(
                "relying-party origin must contain only scheme, host, and optional port",
            ));
        }

        let origin_host = parsed_origin
            .host()
            .ok_or_else(|| passkey_error("relying-party origin must contain a host"))?;
        let local_origin = match origin_host {
            Host::Domain(domain) => domain.eq_ignore_ascii_case("localhost"),
            Host::Ipv4(address) => address.is_loopback(),
            Host::Ipv6(address) => address.is_loopback(),
        };
        if parsed_origin.scheme() != "https" && !(parsed_origin.scheme() == "http" && local_origin)
        {
            return Err(passkey_error(
                "WebAuthn origin must use HTTPS except for an exact loopback host",
            ));
        }

        let parsed_rp_id = Host::parse(config.rp_id.trim())
            .map_err(|error| passkey_error(format!("invalid relying-party ID: {error}")))?;
        let normalized_rp_id = parsed_rp_id.to_string().to_ascii_lowercase();
        let normalized_origin_host = origin_host.to_string().to_ascii_lowercase();
        let rp_matches_origin = normalized_origin_host == normalized_rp_id
            || matches!(parsed_rp_id, Host::Domain(_))
                && normalized_origin_host.ends_with(&format!(".{normalized_rp_id}"));
        if !rp_matches_origin {
            return Err(passkey_error(
                "relying-party ID must equal the origin host or be its domain suffix",
            ));
        }

        Ok(Self {
            rp_name: config.rp_name.clone(),
            rp_id: normalized_rp_id,
            rp_origin: parsed_origin.origin().ascii_serialization(),
            require_user_verification: config.require_user_verification,
            challenge_ttl: Duration::from_secs(config.challenge_ttl_seconds),
            challenges: ChallengeStore::new(
                Duration::from_secs(config.challenge_ttl_seconds),
                config.max_pending_challenges,
            ),
        })
    }

    fn parse_client_data(
        &self,
        encoded: &str,
        expected_type: &str,
    ) -> Result<(Vec<u8>, CollectedClientData), AuthError> {
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(encoded)
            .map_err(|error| passkey_error(format!("failed to decode clientDataJSON: {error}")))?;
        let client_data: CollectedClientData = serde_json::from_slice(&bytes)
            .map_err(|error| passkey_error(format!("failed to parse clientDataJSON: {error}")))?;

        if client_data.ceremony_type != expected_type {
            return Err(passkey_error("clientDataJSON ceremony type mismatch"));
        }
        if client_data.origin != self.rp_origin {
            return Err(passkey_error("Origin mismatch"));
        }
        if client_data.cross_origin {
            return Err(passkey_error(
                "cross-origin WebAuthn ceremony is not allowed",
            ));
        }
        Ok((bytes, client_data))
    }

    fn validate_authenticator_data(&self, auth_data: &[u8]) -> Result<u32, AuthError> {
        if auth_data.len() < 37 {
            return Err(passkey_error("authenticatorData too short"));
        }
        let expected_hash = sha2::Sha256::digest(self.rp_id.as_bytes());
        if auth_data[..32].ct_eq(expected_hash.as_slice()).unwrap_u8() != 1 {
            return Err(passkey_error("rpIdHash mismatch in authenticatorData"));
        }

        let flags = auth_data[32];
        if flags & FLAG_USER_PRESENT == 0 {
            return Err(passkey_error("authenticator did not prove user presence"));
        }
        if self.require_user_verification && flags & FLAG_USER_VERIFIED == 0 {
            return Err(passkey_error(
                "authenticator did not prove user verification",
            ));
        }
        if flags & FLAG_BACKUP_STATE != 0 && flags & FLAG_BACKUP_ELIGIBLE == 0 {
            return Err(passkey_error("invalid authenticator backup flags"));
        }

        Ok(u32::from_be_bytes([
            auth_data[33],
            auth_data[34],
            auth_data[35],
            auth_data[36],
        ]))
    }

    fn decode_credential_id(
        id: &str,
        raw_id: &str,
        credential_type: &str,
    ) -> Result<Vec<u8>, AuthError> {
        if credential_type != "public-key" {
            return Err(passkey_error("credential type must be public-key"));
        }
        let raw = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(raw_id)
            .map_err(|_| passkey_error("raw credential ID is not valid base64url"))?;
        let encoded_id = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(id)
            .map_err(|_| passkey_error("credential ID is not valid base64url"))?;
        if raw.is_empty() || raw.len() > MAX_CREDENTIAL_ID_BYTES {
            return Err(passkey_error("credential ID length is invalid"));
        }
        if raw.as_slice().ct_eq(&encoded_id).unwrap_u8() != 1 {
            return Err(passkey_error("credential ID and raw ID do not match"));
        }
        Ok(raw)
    }

    /// Starts a registration ceremony and stores its challenge as one-time state.
    #[cfg_attr(mutants, mutants::skip)]
    pub fn start_register(
        &self,
        user_id: i32,
        username: &str,
        display_name: &str,
    ) -> Result<(CreationChallengeResponse, String), AuthError> {
        let challenge = self.challenges.issue(Ceremony::Registration, Vec::new())?;
        let mut id_bytes = [0_u8; 16];
        id_bytes[..4].copy_from_slice(&user_id.to_be_bytes());
        let verification = if self.require_user_verification {
            "required"
        } else {
            "preferred"
        };

        let options = CreationChallengeResponse {
            public_key: PublicKeyCredentialCreationOptions {
                challenge: challenge.clone(),
                rp: RelyingPartyInfo {
                    name: self.rp_name.clone(),
                    id: self.rp_id.clone(),
                },
                user: UserInfo {
                    id: base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(id_bytes),
                    name: username.to_owned(),
                    display_name: display_name.to_owned(),
                },
                pub_key_cred_params: vec![PubKeyCredParam {
                    r#type: "public-key".to_owned(),
                    alg: -7,
                }],
                timeout: self.challenge_ttl.as_millis().min(u32::MAX as u128) as u32,
                authenticator_selection: AuthenticatorSelection {
                    resident_key: "preferred".to_owned(),
                    user_verification: verification.to_owned(),
                },
                // Only the standards-defined privacy-preserving none format is accepted below.
                attestation: "none".to_owned(),
            },
        };
        Ok((options, challenge))
    }

    /// Finishes registration for an ES256 credential with `none` attestation.
    #[cfg_attr(mutants, mutants::skip)]
    pub fn finish_register(
        &self,
        credential: &RegisterPublicKeyCredential,
        expected_challenge: &str,
    ) -> Result<Passkey, AuthError> {
        let credential_id =
            Self::decode_credential_id(&credential.id, &credential.raw_id, &credential.r#type)?;
        let (_, client_data) =
            self.parse_client_data(&credential.response.client_data_json, "webauthn.create")?;
        self.challenges.consume(
            expected_challenge,
            &client_data.challenge,
            Ceremony::Registration,
            None,
        )?;

        let attestation = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(&credential.response.attestation_object)
            .map_err(|error| {
                passkey_error(format!("failed to decode attestationObject: {error}"))
            })?;
        let (attestation, trailing) = parse_cbor(&attestation)?;
        if !trailing.is_empty() {
            return Err(passkey_error("trailing data in attestationObject"));
        }
        let CborValue::Map(mut attestation) = attestation else {
            return Err(passkey_error("attestationObject is not a map"));
        };
        let format = attestation.remove(&CborKey::TextString("fmt".to_owned()));
        if !matches!(format, Some(CborValue::TextString(value)) if value == "none") {
            return Err(passkey_error(
                "only the WebAuthn none attestation format is supported",
            ));
        }
        let statement = attestation.remove(&CborKey::TextString("attStmt".to_owned()));
        if !matches!(statement, Some(CborValue::Map(value)) if value.is_empty()) {
            return Err(passkey_error("none attestation must have an empty attStmt"));
        }
        let Some(CborValue::ByteString(auth_data)) =
            attestation.remove(&CborKey::TextString("authData".to_owned()))
        else {
            return Err(passkey_error("authData not found in attestationObject"));
        };
        if !attestation.is_empty() || auth_data.len() < 55 {
            return Err(passkey_error(
                "invalid attestationObject or authData length",
            ));
        }

        let sign_count = self.validate_authenticator_data(&auth_data)?;
        let flags = auth_data[32];
        if flags & FLAG_ATTESTED_CREDENTIAL_DATA == 0 {
            return Err(passkey_error("attested credential data flag is missing"));
        }
        if flags & FLAG_EXTENSION_DATA != 0 {
            return Err(passkey_error(
                "unrequested authenticator extensions are unsupported",
            ));
        }

        let credential_id_len = u16::from_be_bytes([auth_data[53], auth_data[54]]) as usize;
        if credential_id_len == 0
            || credential_id_len > MAX_CREDENTIAL_ID_BYTES
            || auth_data.len() < 55 + credential_id_len
        {
            return Err(passkey_error("invalid attested credential ID length"));
        }
        let attested_id = &auth_data[55..55 + credential_id_len];
        if attested_id.ct_eq(&credential_id).unwrap_u8() != 1 {
            return Err(passkey_error(
                "response credential ID does not match attested credential ID",
            ));
        }

        let (cose_key, trailing) = parse_cbor(&auth_data[55 + credential_id_len..])?;
        if !trailing.is_empty() {
            return Err(passkey_error("trailing data after credential public key"));
        }
        let CborValue::Map(mut cose_key) = cose_key else {
            return Err(passkey_error("credentialPublicKey is not a CBOR map"));
        };
        for (label, expected, name) in [(1, 2, "kty"), (3, -7, "alg"), (-1, 1, "curve")] {
            if !matches!(cose_key.remove(&CborKey::Integer(label)), Some(CborValue::Integer(value)) if value == expected)
            {
                return Err(passkey_error(format!("unsupported or missing COSE {name}")));
            }
        }
        let Some(CborValue::ByteString(x)) = cose_key.remove(&CborKey::Integer(-2)) else {
            return Err(passkey_error("X coordinate missing from public key"));
        };
        let Some(CborValue::ByteString(y)) = cose_key.remove(&CborKey::Integer(-3)) else {
            return Err(passkey_error("Y coordinate missing from public key"));
        };
        if x.len() != 32 || y.len() != 32 {
            return Err(passkey_error(
                "ES256 coordinates must each contain 32 bytes",
            ));
        }
        let mut public_key = Vec::with_capacity(65);
        public_key.push(0x04);
        public_key.extend_from_slice(&x);
        public_key.extend_from_slice(&y);
        p256::PublicKey::from_sec1_bytes(&public_key)
            .map_err(|_| passkey_error("ES256 public key is not a valid P-256 point"))?;

        Ok(Passkey {
            credential_id,
            public_key,
            sign_count,
        })
    }

    /// Starts an authentication ceremony with a one-time challenge and credential allowlist.
    #[cfg_attr(mutants, mutants::skip)]
    pub fn start_authenticate(
        &self,
        allowed_credentials: &[Passkey],
    ) -> Result<(RequestChallengeResponse, String), AuthError> {
        let allowed_ids = allowed_credentials
            .iter()
            .map(|passkey| passkey.credential_id.clone())
            .collect::<Vec<_>>();
        let challenge = self
            .challenges
            .issue(Ceremony::Authentication, allowed_ids)?;
        let verification = if self.require_user_verification {
            "required"
        } else {
            "preferred"
        };
        let allow_credentials = allowed_credentials
            .iter()
            .map(|passkey| AllowCredential {
                r#type: "public-key".to_owned(),
                id: base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&passkey.credential_id),
            })
            .collect();
        let options = RequestChallengeResponse {
            public_key: PublicKeyCredentialRequestOptions {
                challenge: challenge.clone(),
                timeout: self.challenge_ttl.as_millis().min(u32::MAX as u128) as u32,
                rp_id: self.rp_id.clone(),
                allow_credentials,
                user_verification: verification.to_owned(),
            },
        };
        Ok((options, challenge))
    }

    /// Verifies an ES256 assertion and returns the counter that must be persisted atomically.
    #[cfg_attr(mutants, mutants::skip)]
    pub fn finish_authenticate(
        &self,
        credential: &PublicKeyCredential,
        expected_challenge: &str,
        mut passkey: Passkey,
    ) -> Result<Passkey, AuthError> {
        let credential_id =
            Self::decode_credential_id(&credential.id, &credential.raw_id, &credential.r#type)?;
        if credential_id.ct_eq(&passkey.credential_id).unwrap_u8() != 1 {
            return Err(passkey_error(
                "assertion credential does not match the passkey",
            ));
        }
        if passkey.public_key.len() != 65 || passkey.public_key.first() != Some(&0x04) {
            return Err(passkey_error("stored ES256 public key is malformed"));
        }

        let (client_data_bytes, client_data) =
            self.parse_client_data(&credential.response.client_data_json, "webauthn.get")?;
        self.challenges.consume(
            expected_challenge,
            &client_data.challenge,
            Ceremony::Authentication,
            Some(&credential_id),
        )?;

        let auth_data = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(&credential.response.authenticator_data)
            .map_err(|error| {
                passkey_error(format!("failed to decode authenticatorData: {error}"))
            })?;
        let new_sign_count = self.validate_authenticator_data(&auth_data)?;
        if auth_data[32] & (FLAG_ATTESTED_CREDENTIAL_DATA | FLAG_EXTENSION_DATA) != 0
            || auth_data.len() != 37
        {
            return Err(passkey_error(
                "unexpected data in assertion authenticatorData",
            ));
        }

        let signature_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(&credential.response.signature)
            .map_err(|error| passkey_error(format!("failed to decode signature: {error}")))?;
        let client_hash = sha2::Sha256::digest(&client_data_bytes);
        let mut signed_message = Vec::with_capacity(auth_data.len() + client_hash.len());
        signed_message.extend_from_slice(&auth_data);
        signed_message.extend_from_slice(&client_hash);
        signature::UnparsedPublicKey::new(&signature::ECDSA_P256_SHA256_ASN1, &passkey.public_key)
            .verify(&signed_message, &signature_bytes)
            .map_err(|_| passkey_error("ES256 assertion signature verification failed"))?;

        if (passkey.sign_count != 0 || new_sign_count != 0) && new_sign_count <= passkey.sign_count
        {
            return Err(passkey_error(
                "authenticator signature counter did not advance monotonically",
            ));
        }
        passkey.sign_count = new_sign_count;
        Ok(passkey)
    }
}
