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

// TEMPORARY PROBE (fix/v13-alpha6-macos-nightly): dropped before merge.
// Valid signatures with a chosen s: with R = k0·G and r = x(R) mod n, the key
// Q = r⁻¹·(s·R − z·G) makes (r, s) verify for any s.
#[test]
fn es256_probe_structured_and_random_s() {
    use p256::elliptic_curve::PrimeField;
    use p256::elliptic_curve::ops::Reduce;
    use p256::elliptic_curve::point::AffineCoordinates;
    use p256::elliptic_curve::sec1::ToSec1Point;
    use p256::{FieldBytes, ProjectivePoint, Scalar};

    fn integer(bytes: &[u8]) -> Vec<u8> {
        let mut value: Vec<u8> = bytes.iter().copied().skip_while(|b| *b == 0).collect();
        if value.is_empty() {
            value.push(0);
        }
        if value[0] & 0x80 != 0 {
            value.insert(0, 0);
        }
        let mut out = vec![0x02, value.len() as u8];
        out.extend(value);
        out
    }
    fn hashed(data: &[u8]) -> Scalar {
        let digest: [u8; 32] = sha2::Sha256::digest(data).into();
        <Scalar as Reduce<FieldBytes>>::reduce(&FieldBytes::from(digest))
    }
    let message = b"hello, world";
    let z = hashed(message);
    // (aws-lc, ring) disagreements, by family.
    let check = |k0: Scalar, s: Scalar, label: &str, failures: &mut Vec<String>| {
        let point = (ProjectivePoint::GENERATOR * k0).to_affine();
        let r = <Scalar as Reduce<FieldBytes>>::reduce(&point.x());
        let q = (ProjectivePoint::from(point) * s - ProjectivePoint::GENERATOR * z)
            * r.invert().unwrap();
        let public_key = q.to_affine().to_sec1_point(false).as_bytes().to_vec();
        let mut body = integer(&r.to_repr());
        body.extend(integer(&s.to_repr()));
        let mut signature = vec![0x30, body.len() as u8];
        signature.extend(body);
        let aws_lc = super::es256::verify(&public_key, message, &signature);
        let ring =
            super::es256_differential_tests::previous_verify(&public_key, message, &signature);
        if !(aws_lc && ring) {
            failures.push(format!(
                "{label}: aws-lc {aws_lc} ring {ring} s {}",
                s.to_repr()
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
            ));
        }
    };
    let k0 = Scalar::from(0x1234_5678_9abc_def1_u64);
    let mut failures = Vec::new();
    let mut power = Scalar::ONE;
    for k in 0..256 {
        check(k0, power, &format!("2^{k}"), &mut failures);
        check(k0, -power, &format!("n-2^{k}"), &mut failures);
        check(k0, power + Scalar::ONE, &format!("2^{k}+1"), &mut failures);
        check(
            k0,
            -power - Scalar::ONE,
            &format!("n-2^{k}-1"),
            &mut failures,
        );
        check(k0, power - Scalar::ONE, &format!("2^{k}-1"), &mut failures);
        power = power + power;
    }
    let structured = failures.len();
    for index in 0_u32..3000 {
        let k = hashed(&[b"k".as_slice(), &index.to_be_bytes()].concat());
        let s = hashed(&[b"s".as_slice(), &index.to_be_bytes()].concat());
        check(k, s, &format!("random {index}"), &mut failures);
    }
    println!(
        "PROBE2 target {} {}",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    for failure in &failures {
        println!("PROBE2 {failure}");
    }
    println!(
        "PROBE2 {structured} structured of 1280 and {} random of 3000 not verified by both",
        failures.len() - structured
    );
}
