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
pub(super) struct CollectedClientData {
    #[serde(rename = "type")]
    ceremony_type: String,
    pub(super) challenge: String,
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
        let options = self.registration_options(
            challenge.clone(),
            UserInfo {
                id: base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(id_bytes),
                name: username.to_owned(),
                display_name: display_name.to_owned(),
            },
        );
        Ok((options, challenge))
    }

    pub(super) fn registration_options(
        &self,
        challenge: String,
        user: UserInfo,
    ) -> CreationChallengeResponse {
        let verification = if self.require_user_verification {
            "required"
        } else {
            "preferred"
        };

        CreationChallengeResponse {
            public_key: PublicKeyCredentialCreationOptions {
                challenge,
                rp: RelyingPartyInfo {
                    name: self.rp_name.clone(),
                    id: self.rp_id.clone(),
                },
                user,
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
        }
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

        self.verify_registration(credential, &credential_id)
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
        let options = self.authentication_options(challenge.clone(), allowed_credentials);
        Ok((options, challenge))
    }

    pub(super) fn authentication_options(
        &self,
        challenge: String,
        allowed_credentials: &[Passkey],
    ) -> RequestChallengeResponse {
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
        RequestChallengeResponse {
            public_key: PublicKeyCredentialRequestOptions {
                challenge,
                timeout: self.challenge_ttl.as_millis().min(u32::MAX as u128) as u32,
                rp_id: self.rp_id.clone(),
                allow_credentials,
                user_verification: verification.to_owned(),
            },
        }
    }

    /// Verifies an ES256 assertion and returns the counter that must be persisted atomically.
    #[cfg_attr(mutants, mutants::skip)]
    pub fn finish_authenticate(
        &self,
        credential: &PublicKeyCredential,
        expected_challenge: &str,
        passkey: Passkey,
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

        self.verify_assertion(credential, &client_data_bytes, passkey)
    }
    pub(super) fn ceremony_fingerprint(&self) -> [u8; 32] {
        let mut hash = sha2::Sha256::new();
        hash.update(b"rullst.passkey-rp.v1");
        for value in [
            self.rp_id.as_bytes(),
            self.rp_origin.as_bytes(),
            &[u8::from(self.require_user_verification)],
        ] {
            hash.update((value.len() as u64).to_be_bytes());
            hash.update(value);
        }
        hash.finalize().into()
    }
}

#[cfg(test)]
#[path = "service_negative_tests.rs"]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod negative_path_tests;
#[path = "verification.rs"]
mod verification;
