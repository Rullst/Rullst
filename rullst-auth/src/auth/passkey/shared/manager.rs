use super::{
    CeremonyDurability, CeremonyIntent, CeremonyKind, PasskeyBinding,
    PasskeyCeremonyError as Error, PasskeyCeremonyStore, contracts::MAX_CREDENTIALS,
};
use crate::auth::passkey::{
    CreationChallengeResponse, Passkey, PasskeyAuth, PasskeyConfig, PublicKeyCredential,
    RegisterPublicKeyCredential, RequestChallengeResponse, UserInfo,
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

/// Account-first ES256 ceremonies with a trusted shared store. The host must
/// authorize bindings and persist credential ownership/revocation/counter CAS.
#[derive(Clone)]
pub struct SharedPasskeyAuth<S> {
    auth: PasskeyAuth,
    store: S,
}
impl<S: PasskeyCeremonyStore> SharedPasskeyAuth<S> {
    pub fn new(config: &PasskeyConfig, store: S) -> Result<Self, Error> {
        if store.durability() != CeremonyDurability::SharedDurable
            || config.challenge_ttl_seconds != u64::from(store.config().lifetime_seconds())
            || config.max_pending_challenges != store.config().capacity() as usize
        {
            return Err(Error::Configuration);
        }
        let auth = PasskeyAuth::new(config).map_err(|_| Error::Configuration)?;
        Ok(Self { auth, store })
    }

    pub async fn start_register(
        &self,
        binding: &PasskeyBinding,
        username: impl Into<String>,
        display_name: impl Into<String>,
    ) -> Result<(CreationChallengeResponse, String), Error> {
        let username = username.into();
        let display_name = display_name.into();
        if [&username, &display_name]
            .into_iter()
            .any(|s| s.trim().is_empty() || s.len() > 256 || s.chars().any(char::is_control))
        {
            return Err(Error::InvalidInput);
        }
        let challenge = self
            .issue(binding, CeremonyKind::Registration, Vec::new())
            .await?;
        let options = self.auth.registration_options(
            challenge.clone(),
            UserInfo {
                id: URL_SAFE_NO_PAD.encode(&binding.user_handle),
                name: username,
                display_name,
            },
        );
        Ok((options, challenge))
    }

    pub async fn finish_register(
        &self,
        binding: &PasskeyBinding,
        credential: &RegisterPublicKeyCredential,
        expected_challenge: &str,
    ) -> Result<Passkey, Error> {
        if credential.response.attestation_object.len() > 16_384 {
            return Err(Error::InvalidInput);
        }
        self.bound_response(
            &credential.id,
            &credential.raw_id,
            &credential.r#type,
            &credential.response.client_data_json,
        )?;
        let id = PasskeyAuth::decode_credential_id(
            &credential.id,
            &credential.raw_id,
            &credential.r#type,
        )
        .map_err(|_| Error::Rejected)?;
        let (_, data) = self
            .auth
            .parse_client_data(&credential.response.client_data_json, "webauthn.create")
            .map_err(|_| Error::Rejected)?;
        check_challenge(expected_challenge, &data.challenge)?;
        let consumed = self
            .store
            .consume(
                challenge_digest(expected_challenge),
                self.binding(binding),
                CeremonyKind::Registration,
            )
            .await?;
        self.check_consumed(
            &consumed,
            binding,
            expected_challenge,
            CeremonyKind::Registration,
        )?;
        let passkey = self
            .auth
            .verify_registration(credential, &id)
            .map_err(|_| Error::Rejected)?;
        self.store.confirm(&consumed).await?;
        Ok(passkey)
    }

    /// An empty allowlist is rejected: discoverable-account selection is not supported.
    pub async fn start_authenticate(
        &self,
        binding: &PasskeyBinding,
        allowed: &[Passkey],
    ) -> Result<(RequestChallengeResponse, String), Error> {
        if allowed.is_empty() || allowed.len() > MAX_CREDENTIALS {
            return Err(Error::InvalidInput);
        }
        let mut fingerprints = Vec::with_capacity(allowed.len());
        for (index, passkey) in allowed.iter().enumerate() {
            validate_passkey(passkey)?;
            if allowed[..index]
                .iter()
                .any(|p| p.credential_id == passkey.credential_id)
            {
                return Err(Error::InvalidInput);
            }
            fingerprints.push(credential_digest(passkey));
        }
        let challenge = self
            .issue(binding, CeremonyKind::Authentication, fingerprints)
            .await?;
        Ok((
            self.auth.authentication_options(challenge.clone(), allowed),
            challenge,
        ))
    }

    /// Forward the authenticator's optional `userHandle`; use None only if absent.
    /// A successful return still requires authoritative credential-counter CAS.
    pub async fn finish_authenticate(
        &self,
        binding: &PasskeyBinding,
        credential: &PublicKeyCredential,
        expected_challenge: &str,
        passkey: Passkey,
        user_handle: Option<&str>,
    ) -> Result<Passkey, Error> {
        self.bound_response(
            &credential.id,
            &credential.raw_id,
            &credential.r#type,
            &credential.response.client_data_json,
        )?;
        if credential.response.authenticator_data.len() > 512
            || credential.response.signature.len() > 256
        {
            return Err(Error::InvalidInput);
        }
        validate_passkey(&passkey)?;
        if let Some(handle) = user_handle {
            if handle.len() > 86 {
                return Err(Error::InvalidInput);
            }
            let bytes = URL_SAFE_NO_PAD
                .decode(handle)
                .map_err(|_| Error::Rejected)?;
            if bytes.ct_eq(&binding.user_handle).unwrap_u8() != 1 {
                return Err(Error::Rejected);
            }
        }
        let id = PasskeyAuth::decode_credential_id(
            &credential.id,
            &credential.raw_id,
            &credential.r#type,
        )
        .map_err(|_| Error::Rejected)?;
        if id.ct_eq(&passkey.credential_id).unwrap_u8() != 1 {
            return Err(Error::Rejected);
        }
        let (bytes, data) = self
            .auth
            .parse_client_data(&credential.response.client_data_json, "webauthn.get")
            .map_err(|_| Error::Rejected)?;
        check_challenge(expected_challenge, &data.challenge)?;
        let consumed = self
            .store
            .consume(
                challenge_digest(expected_challenge),
                self.binding(binding),
                CeremonyKind::Authentication,
            )
            .await?;
        self.check_consumed(
            &consumed,
            binding,
            expected_challenge,
            CeremonyKind::Authentication,
        )?;
        let fingerprint = credential_digest(&passkey);
        if !consumed
            .intent()
            .credentials()
            .iter()
            .any(|allowed| allowed.ct_eq(&fingerprint).unwrap_u8() == 1)
        {
            return Err(Error::Rejected);
        }
        let verified = self
            .auth
            .verify_assertion(credential, &bytes, passkey)
            .map_err(|_| Error::Rejected)?;
        self.store.confirm(&consumed).await?;
        Ok(verified)
    }

    async fn issue(
        &self,
        binding: &PasskeyBinding,
        kind: CeremonyKind,
        credentials: Vec<[u8; 32]>,
    ) -> Result<String, Error> {
        for _ in 0..4 {
            let mut nonce = [0_u8; 32];
            ring::rand::SecureRandom::fill(&ring::rand::SystemRandom::new(), &mut nonce)
                .map_err(|_| Error::Unavailable)?;
            let challenge = URL_SAFE_NO_PAD.encode(nonce);
            let intent = CeremonyIntent::new(
                challenge_digest(&challenge),
                self.binding(binding),
                kind,
                credentials.clone(),
            )?;
            match self.store.issue(&intent).await {
                Ok(()) => return Ok(challenge),
                Err(Error::Conflict) => continue,
                Err(error) => return Err(error),
            }
        }
        Err(Error::Conflict)
    }

    fn binding(&self, binding: &PasskeyBinding) -> [u8; 32] {
        framed_digest(
            b"rullst.passkey-binding.v1",
            &[
                binding.tenant.as_bytes(),
                binding.subject.as_bytes(),
                binding.session.as_bytes(),
                &binding.user_handle,
                &self.auth.ceremony_fingerprint(),
                self.store.config().epoch().as_bytes(),
            ],
        )
    }
    fn bound_response(
        &self,
        id: &str,
        raw_id: &str,
        kind: &str,
        client_data: &str,
    ) -> Result<(), Error> {
        if id.len() > 1364
            || raw_id.len() > 1364
            || kind != "public-key"
            || client_data.len() > 8192
        {
            return Err(Error::InvalidInput);
        }
        Ok(())
    }
    fn check_consumed(
        &self,
        consumed: &super::ConsumedCeremony,
        binding: &PasskeyBinding,
        challenge: &str,
        kind: CeremonyKind,
    ) -> Result<(), Error> {
        let intent = consumed.intent();
        if intent.kind() != kind
            || intent
                .challenge()
                .ct_eq(&challenge_digest(challenge))
                .unwrap_u8()
                != 1
            || intent.binding().ct_eq(&self.binding(binding)).unwrap_u8() != 1
        {
            return Err(Error::Corrupt);
        }
        Ok(())
    }
}

fn check_challenge(expected: &str, received: &str) -> Result<(), Error> {
    if expected.len() != 43
        || URL_SAFE_NO_PAD
            .decode(expected)
            .map_or(true, |v| v.len() != 32)
        || expected.as_bytes().ct_eq(received.as_bytes()).unwrap_u8() != 1
    {
        return Err(Error::Rejected);
    }
    Ok(())
}
fn challenge_digest(challenge: &str) -> [u8; 32] {
    framed_digest(b"rullst.passkey-challenge.v1", &[challenge.as_bytes()])
}
fn credential_digest(passkey: &Passkey) -> [u8; 32] {
    framed_digest(
        b"rullst.passkey-credential.v1",
        &[
            &passkey.credential_id,
            &passkey.public_key,
            &passkey.sign_count.to_be_bytes(),
        ],
    )
}
fn validate_passkey(passkey: &Passkey) -> Result<(), Error> {
    if passkey.credential_id.is_empty()
        || passkey.credential_id.len() > 1023
        || passkey.public_key.len() != 65
        || passkey.public_key.first() != Some(&4)
        || p256::PublicKey::from_sec1_bytes(&passkey.public_key).is_err()
    {
        return Err(Error::InvalidInput);
    }
    Ok(())
}
pub(super) fn framed_digest(domain: &[u8], values: &[&[u8]]) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(domain);
    for value in values {
        hash.update((value.len() as u64).to_be_bytes());
        hash.update(value);
    }
    hash.finalize().into()
}
