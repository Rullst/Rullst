use super::{
    PasskeyAuth,
    cbor::{CborKey, CborValue},
    types::{
        AuthenticatorAssertionResponse, AuthenticatorAttestationResponse, Passkey,
        PublicKeyCredential, RegisterPublicKeyCredential,
    },
};
use base64::Engine as _;
use ring::signature::KeyPair;
use sha2::Digest;
use std::collections::HashMap;

fn encode_cbor_integer(integer: i64) -> Vec<u8> {
    let (major, value) = if integer >= 0 {
        (0_u8, integer as u64)
    } else {
        (1_u8, (-1_i128 - integer as i128) as u64)
    };
    let prefix = major << 5;
    match value {
        0..=23 => vec![prefix | value as u8],
        24..=255 => vec![prefix | 24, value as u8],
        256..=65_535 => {
            let mut encoded = vec![prefix | 25];
            encoded.extend_from_slice(&(value as u16).to_be_bytes());
            encoded
        }
        65_536..=4_294_967_295 => {
            let mut encoded = vec![prefix | 26];
            encoded.extend_from_slice(&(value as u32).to_be_bytes());
            encoded
        }
        _ => {
            let mut encoded = vec![prefix | 27];
            encoded.extend_from_slice(&value.to_be_bytes());
            encoded
        }
    }
}

pub(super) fn encode_cbor(value: &CborValue) -> Vec<u8> {
    match value {
        CborValue::Integer(integer) => encode_cbor_integer(*integer),
        CborValue::ByteString(bytes) => {
            let mut encoded = if bytes.len() <= 23 {
                vec![0x40 | bytes.len() as u8]
            } else if bytes.len() <= 255 {
                vec![0x58, bytes.len() as u8]
            } else {
                let mut prefix = vec![0x59];
                prefix.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
                prefix
            };
            encoded.extend_from_slice(bytes);
            encoded
        }
        CborValue::TextString(text) => {
            let mut encoded = if text.len() <= 23 {
                vec![0x60 | text.len() as u8]
            } else if text.len() <= 255 {
                vec![0x78, text.len() as u8]
            } else {
                let mut prefix = vec![0x79];
                prefix.extend_from_slice(&(text.len() as u16).to_be_bytes());
                prefix
            };
            encoded.extend_from_slice(text.as_bytes());
            encoded
        }
        CborValue::Array(items) => {
            let mut encoded = vec![0x80 | items.len() as u8];
            for item in items {
                encoded.extend_from_slice(&encode_cbor(item));
            }
            encoded
        }
        CborValue::Map(entries) => {
            let mut encoded = vec![0xA0 | entries.len() as u8];
            for (key, value) in entries {
                encoded.extend_from_slice(&encode_cbor(&match key {
                    CborKey::Integer(integer) => CborValue::Integer(*integer),
                    CborKey::TextString(text) => CborValue::TextString(text.clone()),
                }));
                encoded.extend_from_slice(&encode_cbor(value));
            }
            encoded
        }
    }
}

pub(super) struct RegistrationOptions {
    pub(super) flags: u8,
    pub(super) kty: i64,
    pub(super) algorithm: i64,
    pub(super) curve: i64,
    pub(super) format: String,
    pub(super) client_type: String,
    pub(super) origin: String,
    pub(super) cross_origin: bool,
    pub(super) raw_id: Option<Vec<u8>>,
    pub(super) coordinate_length: usize,
    pub(super) invalid_curve_point: bool,
    pub(super) include_x: bool,
    pub(super) include_y: bool,
}

impl Default for RegistrationOptions {
    fn default() -> Self {
        Self {
            flags: 0x45,
            kty: 2,
            algorithm: -7,
            curve: 1,
            format: "none".to_owned(),
            client_type: "webauthn.create".to_owned(),
            origin: "http://localhost".to_owned(),
            cross_origin: false,
            raw_id: None,
            coordinate_length: 32,
            invalid_curve_point: false,
            include_x: true,
            include_y: true,
        }
    }
}

pub(super) struct RegistrationFixture {
    pub(super) credential: RegisterPublicKeyCredential,
    pub(super) challenge: String,
    pub(super) credential_id: Vec<u8>,
    pub(super) public_key: Vec<u8>,
    pub(super) key_pair: ring::signature::EcdsaKeyPair,
}

pub(super) fn registration_fixture(
    auth: &PasskeyAuth,
    rp_id: &str,
    options: RegistrationOptions,
) -> RegistrationFixture {
    let rng = ring::rand::SystemRandom::new();
    let key_document = ring::signature::EcdsaKeyPair::generate_pkcs8(
        &ring::signature::ECDSA_P256_SHA256_ASN1_SIGNING,
        &rng,
    )
    .expect("test key generation should succeed");
    let key_pair = ring::signature::EcdsaKeyPair::from_pkcs8(
        &ring::signature::ECDSA_P256_SHA256_ASN1_SIGNING,
        key_document.as_ref(),
        &rng,
    )
    .expect("test key parsing should succeed");
    let public_key = key_pair.public_key().as_ref().to_vec();

    let mut cose = HashMap::new();
    cose.insert(CborKey::Integer(1), CborValue::Integer(options.kty));
    cose.insert(CborKey::Integer(3), CborValue::Integer(options.algorithm));
    cose.insert(CborKey::Integer(-1), CborValue::Integer(options.curve));
    let coordinate_length = options.coordinate_length.min(32);
    let mut x = public_key[1..1 + coordinate_length].to_vec();
    let mut y = public_key[33..33 + coordinate_length].to_vec();
    if options.invalid_curve_point {
        x.fill(0);
        y.fill(0);
    }
    if options.include_x {
        cose.insert(CborKey::Integer(-2), CborValue::ByteString(x));
    }
    if options.include_y {
        cose.insert(CborKey::Integer(-3), CborValue::ByteString(y));
    }

    let credential_id = vec![10, 20, 30, 40];
    let mut auth_data = sha2::Sha256::digest(rp_id.as_bytes()).to_vec();
    auth_data.push(options.flags);
    auth_data.extend_from_slice(&1_u32.to_be_bytes());
    auth_data.extend_from_slice(&[0_u8; 16]);
    auth_data.extend_from_slice(&(credential_id.len() as u16).to_be_bytes());
    auth_data.extend_from_slice(&credential_id);
    auth_data.extend_from_slice(&encode_cbor(&CborValue::Map(cose)));

    let mut attestation = HashMap::new();
    attestation.insert(
        CborKey::TextString("fmt".to_owned()),
        CborValue::TextString(options.format),
    );
    attestation.insert(
        CborKey::TextString("attStmt".to_owned()),
        CborValue::Map(HashMap::new()),
    );
    attestation.insert(
        CborKey::TextString("authData".to_owned()),
        CborValue::ByteString(auth_data),
    );
    let (_, challenge) = auth
        .start_register(1, "alice", "Alice")
        .expect("registration challenge should be issued");
    let client_data = serde_json::json!({
        "challenge": challenge.clone(),
        "origin": options.origin,
        "type": options.client_type,
        "crossOrigin": options.cross_origin,
    });
    let id = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&credential_id);
    let raw_id = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(options.raw_id.as_deref().unwrap_or(&credential_id));
    let credential = RegisterPublicKeyCredential {
        id,
        raw_id,
        r#type: "public-key".to_owned(),
        response: AuthenticatorAttestationResponse {
            attestation_object: base64::engine::general_purpose::URL_SAFE_NO_PAD
                .encode(encode_cbor(&CborValue::Map(attestation))),
            client_data_json: base64::engine::general_purpose::URL_SAFE_NO_PAD
                .encode(client_data.to_string()),
        },
    };
    RegistrationFixture {
        credential,
        challenge,
        credential_id,
        public_key,
        key_pair,
    }
}

pub(super) fn assertion_fixture(
    auth: &PasskeyAuth,
    passkey: &Passkey,
    key_pair: &ring::signature::EcdsaKeyPair,
    rp_id: &str,
    flags: u8,
    sign_count: u32,
) -> (PublicKeyCredential, String) {
    let (_, challenge) = auth
        .start_authenticate(std::slice::from_ref(passkey))
        .expect("authentication challenge should be issued");
    let client_data = serde_json::json!({
        "challenge": challenge.clone(),
        "origin": "http://localhost",
        "type": "webauthn.get",
        "crossOrigin": false,
    })
    .to_string();
    let mut auth_data = sha2::Sha256::digest(rp_id.as_bytes()).to_vec();
    auth_data.push(flags);
    auth_data.extend_from_slice(&sign_count.to_be_bytes());
    let client_hash = sha2::Sha256::digest(client_data.as_bytes());
    let mut signed = auth_data.clone();
    signed.extend_from_slice(&client_hash);
    let signature = key_pair
        .sign(&ring::rand::SystemRandom::new(), &signed)
        .expect("test signature should succeed");
    let id = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&passkey.credential_id);
    let credential = PublicKeyCredential {
        id: id.clone(),
        raw_id: id,
        r#type: "public-key".to_owned(),
        response: AuthenticatorAssertionResponse {
            authenticator_data: base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(auth_data),
            signature: base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(signature.as_ref()),
            client_data_json: base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(client_data),
        },
    };
    (credential, challenge)
}
