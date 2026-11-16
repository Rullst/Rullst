use super::*;
use serde_json::{Value, json};

fn only(jwk: Value) -> JwkSet {
    serde_json::from_value(json!({ "keys": [jwk] })).expect("JWK Set")
}

#[test]
fn a_sole_curve_key_verifies_only_its_algorithm() {
    let p256 = only(json!({ "kty": "EC", "crv": "P-256", "x": "AAAA", "y": "AAAA" }));
    let p384 = only(json!({ "kty": "EC", "crv": "P-384", "x": "AAAA", "y": "AAAA" }));
    let ed25519 = only(json!({ "kty": "OKP", "crv": "Ed25519", "x": "AAAA" }));
    let okp_p256 = only(json!({ "kty": "OKP", "crv": "P-256", "x": "AAAA" }));
    for (set, alg, usable) in [
        (&p256, Algorithm::ES256, true),
        (&p256, Algorithm::ES384, false),
        (&p384, Algorithm::ES384, true),
        (&p384, Algorithm::ES256, false),
        (&ed25519, Algorithm::EdDSA, true),
        (&ed25519, Algorithm::ES256, false),
        (&okp_p256, Algorithm::EdDSA, false),
    ] {
        assert_eq!(
            sole_verification_key(set, alg).is_ok(),
            usable,
            "{alg:?} with {:?}",
            set.keys[0].algorithm
        );
    }
}
