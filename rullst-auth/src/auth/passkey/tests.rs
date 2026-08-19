#![allow(clippy::unwrap_used)]

use super::cbor::{CborKey, CborValue, parse_cbor};
use super::*;
use base64::Engine as _;
use ring::signature::KeyPair;
use sha2::Digest;

#[test]
fn test_passkey_config_builder() {
    let config = PasskeyConfig::new("RP", "rp.com", "https://rp.com")
        .with_rp_name("New RP")
        .with_rp_id("new.rp.com")
        .with_rp_origin("https://new.rp.com");

    assert_eq!(config.rp_name, "New RP");
    assert_eq!(config.rp_id, "new.rp.com");
    assert_eq!(config.rp_origin, "https://new.rp.com");
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
    let client_data_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(client_data_json);

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
    let client_data_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(client_data_json);

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
    let bytes = vec![0x44, 0x01, 0x02, 0x03, 0x04];
    let (val, rest) = parse_cbor(&bytes).unwrap();
    assert!(rest.is_empty());
    if let CborValue::ByteString(b) = val {
        assert_eq!(b, vec![1, 2, 3, 4]);
    } else {
        panic!();
    }
}

#[test]
fn test_parse_cbor_text_string() {
    let bytes = vec![0x65, b'h', b'e', b'l', b'l', b'o'];
    let (val, rest) = parse_cbor(&bytes).unwrap();
    assert!(rest.is_empty());
    if let CborValue::TextString(s) = val {
        assert_eq!(s, "hello");
    } else {
        panic!();
    }
}

#[test]
fn test_parse_cbor_array() {
    let bytes = vec![0x82, 0x01, 0x02];
    let (val, rest) = parse_cbor(&bytes).unwrap();
    assert!(rest.is_empty());
    if let CborValue::Array(arr) = val {
        assert_eq!(arr.len(), 2);
    } else {
        panic!();
    }
}

#[test]
fn test_parse_cbor_map() {
    let bytes = vec![0xA1, 0x61, b'a', 0x01];
    let (val, rest) = parse_cbor(&bytes).unwrap();
    assert!(rest.is_empty());
    if let CborValue::Map(map) = val {
        assert_eq!(map.len(), 1);
        let k = CborKey::TextString("a".to_string());
        if let Some(CborValue::Integer(v)) = map.get(&k) {
            assert_eq!(*v, 1);
        } else {
            panic!();
        }
    } else {
        panic!();
    }
}

#[test]
fn test_parse_cbor_errors() {
    assert!(parse_cbor(&[]).is_err());
    assert!(parse_cbor(&[0x18]).is_err());
    assert!(parse_cbor(&[0x19, 0x01]).is_err());
    assert!(parse_cbor(&[0x1A, 0x01, 0x02, 0x03]).is_err());
    assert!(parse_cbor(&[0x1B, 0x01]).is_err());
    assert!(parse_cbor(&[0x1C]).is_err());
    assert!(parse_cbor(&[0x45, 0x01]).is_err());
    assert!(parse_cbor(&[0x65, 0x01]).is_err());
    assert!(parse_cbor(&[0xE0]).is_err());
    assert!(parse_cbor(&[0xA1, 0x80, 0x01]).is_err());
}

#[test]
fn test_parse_cbor_u32_and_u64() {
    let bytes_u32 = vec![0x1A, 0x00, 0x01, 0x00, 0x00];
    let (val, _) = parse_cbor(&bytes_u32).unwrap();
    if let CborValue::Integer(i) = val {
        assert_eq!(i, 65536);
    } else {
        panic!();
    }

    let bytes_u64 = vec![0x1B, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00];
    let (val, _) = parse_cbor(&bytes_u64).unwrap();
    if let CborValue::Integer(i) = val {
        assert_eq!(i, 4294967296);
    } else {
        panic!();
    }
}

#[test]
fn test_finish_register_and_authenticate_complete_flow() {
    let config = PasskeyConfig::new("Test App", "localhost", "http://localhost");
    let auth = PasskeyAuth::new(&config).unwrap();

    let rng = ring::rand::SystemRandom::new();
    let pkcs8_doc = ring::signature::EcdsaKeyPair::generate_pkcs8(
        &ring::signature::ECDSA_P256_SHA256_ASN1_SIGNING,
        &rng,
    )
    .unwrap();
    let key_pair = ring::signature::EcdsaKeyPair::from_pkcs8(
        &ring::signature::ECDSA_P256_SHA256_ASN1_SIGNING,
        pkcs8_doc.as_ref(),
        &rng,
    )
    .unwrap();

    let pub_key_bytes = key_pair.public_key().as_ref();
    let x_coord = &pub_key_bytes[1..33];
    let y_coord = &pub_key_bytes[33..65];

    let mut cose_map = std::collections::HashMap::new();
    cose_map.insert(
        CborKey::Integer(-2),
        CborValue::ByteString(x_coord.to_vec()),
    );
    cose_map.insert(
        CborKey::Integer(-3),
        CborValue::ByteString(y_coord.to_vec()),
    );
    let cose_val = CborValue::Map(cose_map);

    fn encode_cbor(val: &CborValue) -> Vec<u8> {
        match val {
            CborValue::Integer(i) => {
                if *i >= 0 && *i <= 23 {
                    vec![*i as u8]
                } else if *i < 0 && *i >= -24 {
                    vec![0x20 | (-1i64 - *i) as u8]
                } else {
                    vec![]
                }
            }
            CborValue::ByteString(b) => {
                let mut res = Vec::new();
                if b.len() <= 23 {
                    res.push(0x40 | (b.len() as u8));
                } else if b.len() <= 255 {
                    res.push(0x58);
                    res.push(b.len() as u8);
                } else {
                    res.push(0x59);
                    res.extend_from_slice(&(b.len() as u16).to_be_bytes());
                }
                res.extend_from_slice(b);
                res
            }
            CborValue::TextString(s) => {
                let mut res = Vec::new();
                if s.len() <= 23 {
                    res.push(0x60 | (s.len() as u8));
                } else if s.len() <= 255 {
                    res.push(0x78);
                    res.push(s.len() as u8);
                } else {
                    res.push(0x79);
                    res.extend_from_slice(&(s.len() as u16).to_be_bytes());
                }
                res.extend_from_slice(s.as_bytes());
                res
            }
            CborValue::Array(a) => {
                let mut res = Vec::new();
                if a.len() <= 23 {
                    res.push(0x80 | (a.len() as u8));
                } else {
                    res.push(0x98);
                    res.push(a.len() as u8);
                }
                for item in a {
                    res.extend_from_slice(&encode_cbor(item));
                }
                res
            }
            CborValue::Map(m) => {
                let mut res = Vec::new();
                if m.len() <= 23 {
                    res.push(0xA0 | (m.len() as u8));
                } else {
                    res.push(0xB8);
                    res.push(m.len() as u8);
                }
                for (k, v) in m {
                    match k {
                        CborKey::Integer(i) => {
                            res.extend_from_slice(&encode_cbor(&CborValue::Integer(*i)))
                        }
                        CborKey::TextString(s) => {
                            res.extend_from_slice(&encode_cbor(&CborValue::TextString(s.clone())))
                        }
                    }
                    res.extend_from_slice(&encode_cbor(v));
                }
                res
            }
        }
    }

    let cose_bytes = encode_cbor(&cose_val);

    let mut auth_data = Vec::new();
    let mut rp_hasher = sha2::Sha256::new();
    rp_hasher.update(b"localhost");
    let rp_id_hash = rp_hasher.finalize();
    auth_data.extend_from_slice(&rp_id_hash);
    auth_data.push(0x41); // UP + AT flags
    auth_data.extend_from_slice(&[0, 0, 0, 1]); // sign count
    auth_data.extend_from_slice(&[0u8; 16]); // AAGUID
    let cred_id = vec![10, 20, 30, 40];
    auth_data.extend_from_slice(&(cred_id.len() as u16).to_be_bytes());
    auth_data.extend_from_slice(&cred_id);
    auth_data.extend_from_slice(&cose_bytes);

    let mut attestation_map = std::collections::HashMap::new();
    attestation_map.insert(
        CborKey::TextString("authData".to_string()),
        CborValue::ByteString(auth_data.clone()),
    );
    let attestation_val = CborValue::Map(attestation_map);
    let attestation_bytes = encode_cbor(&attestation_val);

    let reg_challenge = "test_reg_challenge";
    let client_data_json = serde_json::json!({
        "challenge": reg_challenge,
        "origin": "http://localhost",
        "type": "webauthn.create"
    })
    .to_string();

    let reg_cred = RegisterPublicKeyCredential {
        id: "cred_id".into(),
        raw_id: "raw_cred_id".into(),
        r#type: "public-key".into(),
        response: AuthenticatorAttestationResponse {
            attestation_object: base64::engine::general_purpose::URL_SAFE_NO_PAD
                .encode(attestation_bytes),
            client_data_json: base64::engine::general_purpose::URL_SAFE_NO_PAD
                .encode(client_data_json),
        },
    };

    let passkey = auth.finish_register(&reg_cred, reg_challenge).unwrap();
    assert_eq!(passkey.credential_id, cred_id);
    assert_eq!(passkey.public_key, pub_key_bytes);

    let auth_challenge = "test_auth_challenge";
    let auth_client_data_json = serde_json::json!({
        "challenge": auth_challenge,
        "origin": "http://localhost",
        "type": "webauthn.get"
    })
    .to_string();
    let auth_client_data_bytes = auth_client_data_json.as_bytes();

    let mut auth_auth_data = Vec::new();
    auth_auth_data.extend_from_slice(&rp_id_hash);
    auth_auth_data.push(0x01); // UP flag
    auth_auth_data.extend_from_slice(&[0, 0, 0, 5]); // sign count 5

    let mut hasher = sha2::Sha256::new();
    hasher.update(auth_client_data_bytes);
    let client_hash = hasher.finalize();

    let mut to_sign = Vec::new();
    to_sign.extend_from_slice(&auth_auth_data);
    to_sign.extend_from_slice(&client_hash);

    let sig = key_pair.sign(&rng, &to_sign).unwrap();

    let auth_cred = PublicKeyCredential {
        id: "cred_id".into(),
        raw_id: "raw_cred_id".into(),
        r#type: "public-key".into(),
        response: AuthenticatorAssertionResponse {
            authenticator_data: base64::engine::general_purpose::URL_SAFE_NO_PAD
                .encode(&auth_auth_data),
            signature: base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(sig.as_ref()),
            client_data_json: base64::engine::general_purpose::URL_SAFE_NO_PAD
                .encode(auth_client_data_json),
        },
    };

    let updated_passkey = auth
        .finish_authenticate(&auth_cred, auth_challenge, passkey)
        .unwrap();
    assert_eq!(updated_passkey.sign_count, 5);
}
