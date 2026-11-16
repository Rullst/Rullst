//! Wycheproof ECDSA P-256 with SHA-256 vectors (ASN.1 DER signatures) through
//! both ES256 checks of `es256_differential_tests`.
//!
//! `rullst-auth/tests/fixtures/wycheproof/ecdsa_secp256r1_sha256_test.json` is
//! `testvectors_v1/ecdsa_secp256r1_sha256_test.json` of
//! <https://github.com/C2SP/wycheproof> at commit
//! `65ca494727a52944c204441c860b039e6786b99b`, unmodified; it is licensed under
//! the Apache License 2.0 (`LICENSE` beside it).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::es256_differential_tests::{point_decision, signature_decision, unhex};
use sha2::Digest;
use std::path::Path;

/// SHA-256 of the upstream file, so an edited fixture fails instead of passing.
const FIXTURE_SHA256: &str = "182db4f3e230f6f9fa9f800d2a614dede30284b8e8438bbfe1171905402e9332";

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
            assert_eq!(
                signature_decision(&public_key, &message, &signature),
                expected,
                "tcId {id}: {}",
                test["comment"]
            );
            results[usize::from(expected)] += 1;
        }
    }
    // [invalid, valid]: every vector of the file ran.
    assert_eq!(results, [310, 174]);
    assert_eq!(results[0] + results[1], vectors["numberOfTests"]);
}

// TEMPORARY PROBE (fix/v13-alpha6-macos-nightly): dropped before merge.
#[test]
fn es256_probe_lists_every_disagreement() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/wycheproof/ecdsa_secp256r1_sha256_test.json");
    let vectors: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    println!(
        "PROBE target {} {}",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    let mut mismatches = 0;
    for group in vectors["testGroups"].as_array().unwrap() {
        let public_key = unhex(group["publicKey"]["uncompressed"].as_str().unwrap());
        for test in group["tests"].as_array().unwrap() {
            let expected = test["result"] == "valid";
            let message = unhex(test["msg"].as_str().unwrap());
            let signature = unhex(test["sig"].as_str().unwrap());
            let aws_lc = super::es256::verify(&public_key, &message, &signature);
            let ring =
                super::es256_differential_tests::previous_verify(&public_key, &message, &signature);
            if aws_lc != expected || ring != expected {
                mismatches += 1;
                println!(
                    "PROBE tcId {} result {} aws-lc {aws_lc} ring {ring} comment {} flags {}",
                    test["tcId"], test["result"], test["comment"], test["flags"]
                );
            }
        }
    }
    println!("PROBE {mismatches} disagreement(s)");
}
