//! Wycheproof ECDSA P-256 with SHA-256 vectors (ASN.1 DER signatures) through
//! both ES256 checks of `es256_differential_tests`.
//!
//! `rullst-auth/tests/fixtures/wycheproof/ecdsa_secp256r1_sha256_test.json` is
//! `testvectors_v1/ecdsa_secp256r1_sha256_test.json` of
//! <https://github.com/C2SP/wycheproof> at commit
//! `65ca494727a52944c204441c860b039e6786b99b`, unmodified; it is licensed under
//! the Apache License 2.0 (`LICENSE` beside it).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::es256_differential_tests::{point_decision, previous_verify, signature_decision, unhex};
use sha2::Digest;
use std::path::Path;

/// SHA-256 of the upstream file, so an edited fixture fails instead of passing.
const FIXTURE_SHA256: &str = "182db4f3e230f6f9fa9f800d2a614dede30284b8e8438bbfe1171905402e9332";

/// Valid vectors that aws-lc-rs rejects on aarch64: tcId 483 ("s = 2^128")
/// and tcId 484 ("s = n - 2^128"), both `result: valid` with the
/// `ValidSignature` flag. Measured with aws-lc-rs 1.18.0 and aws-lc-sys 0.44.0
/// on aarch64-unknown-linux-gnu and aarch64-apple-darwin; x86_64 Linux and
/// macOS accept them, as `ring` does everywhere. A fail-safe false rejection:
/// a valid signature does not verify, and no invalid vector is accepted.
/// Signatures built with a chosen s showed the class on both targets: every
/// s = 2^k and s = n - 2^k with 64 <= k <= 255 is rejected, while 2^k ± 1,
/// n - 2^k - 1 and 3,000 random s verify, so an honest signer's uniform s
/// meets it with probability about 2^-247. On aarch64 the test requires the
/// rejection, so a fixed aws-lc fails it, and these identifiers can then be
/// removed.
const AARCH64_FALSE_REJECTIONS: [u64; 2] = [483, 484];

/// A listed aarch64 false rejection: the vector is valid, `ring` accepts it
/// and aws-lc-rs still rejects it. Returns the expected (valid) result.
fn known_false_rejection(id: u64, expected: bool, key: &[u8], message: &[u8], sig: &[u8]) -> bool {
    assert!(
        expected,
        "tcId {id} is listed as a false rejection but is not valid"
    );
    assert!(
        previous_verify(key, message, sig),
        "tcId {id}: ring rejects it too"
    );
    assert!(
        !super::es256::verify(key, message, sig),
        "tcId {id} now verifies on aarch64: aws-lc fixed the false rejection; remove it from AARCH64_FALSE_REJECTIONS"
    );
    expected
}

#[test]
fn wycheproof_ecdsa_p256_sha256_der_vectors_give_their_expected_results() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("wycheproof")
        .join("ecdsa_secp256r1_sha256_test.json");
    let bytes = std::fs::read(&path).unwrap();
    let digest: String = sha2::Sha256::digest(&bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    assert_eq!(digest, FIXTURE_SHA256, "{} was modified", path.display());
    let vectors: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(vectors["algorithm"], "ECDSA");
    let mut results = [0_u64; 2];
    let mut listed = Vec::new();
    for group in vectors["testGroups"].as_array().unwrap() {
        assert_eq!(group["type"], "EcdsaVerify");
        assert_eq!(group["sha"], "SHA-256");
        assert_eq!(group["publicKey"]["curve"], "secp256r1");
        let public_key = unhex(group["publicKey"]["uncompressed"].as_str().unwrap());
        assert!(point_decision(&public_key), "group key rejected");
        for test in group["tests"].as_array().unwrap() {
            let id = test["tcId"].as_u64().unwrap();
            let expected = match test["result"].as_str().unwrap() {
                "valid" => true,
                "invalid" => false,
                other => panic!("tcId {id}: result {other:?} is not handled"),
            };
            let message = unhex(test["msg"].as_str().unwrap());
            let signature = unhex(test["sig"].as_str().unwrap());
            let false_rejection = AARCH64_FALSE_REJECTIONS.contains(&id);
            if false_rejection {
                listed.push(id);
            }
            // Elsewhere, and for every other vector, both verifiers must agree.
            let decision = if false_rejection && cfg!(target_arch = "aarch64") {
                known_false_rejection(id, expected, &public_key, &message, &signature)
            } else {
                signature_decision(&public_key, &message, &signature)
            };
            assert_eq!(decision, expected, "tcId {id}: {}", test["comment"]);
            results[usize::from(expected)] += 1;
        }
    }
    assert_eq!(listed, AARCH64_FALSE_REJECTIONS, "the listed vectors ran");
    // [invalid, valid]: every vector of the file ran.
    assert_eq!(results, [310, 174]);
    assert_eq!(results[0] + results[1], vectors["numberOfTests"]);
}
