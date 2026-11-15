//! Differential tests of the ES256 checks: aws-lc-rs (`es256`) against the
//! checks used up to 13.0.0-alpha.5, `p256` for the public key at registration
//! and in the stores, and `ring` for the assertion signature. Every caller
//! required 65 bytes starting with `0x04` before either library ran, so the
//! previous checks below include that gate.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::es256;
use super::test_support::{RegistrationOptions, assertion_fixture, registration_fixture};
use super::{PasskeyAuth, PasskeyConfig};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use ring::rand::SystemRandom;
use ring::signature::{self, EcdsaKeyPair, KeyPair};

/// P-256 field prime.
const P: [u8; 32] = hex32("ffffffff00000001000000000000000000000000ffffffffffffffffffffffff");
/// P-256 group order.
const N: [u8; 32] = hex32("ffffffff00000000ffffffffffffffffbce6faada7179e84f3b9cac2fc632551");
/// The point with the smallest x coordinate (5), so `x + p` still fits 32 bytes.
const SMALL_X: ([u8; 32], [u8; 32]) = (
    hex32("0000000000000000000000000000000000000000000000000000000000000005"),
    hex32("459243b9aa581806fe913bce99817ade11ca503c64d9a3c533415c083248fbcc"),
);
/// A point whose y coordinate is 1, so `y + p` still fits 32 bytes.
const SMALL_Y: ([u8; 32], [u8; 32]) = (
    hex32("09e78d4ef60d05f750f6636209092bc43cbdd6b47e11a9de20a9feb2a50bb96c"),
    hex32("0000000000000000000000000000000000000000000000000000000000000001"),
);

const fn hex32(text: &str) -> [u8; 32] {
    let digits = text.as_bytes();
    assert!(digits.len() == 64);
    let mut out = [0; 32];
    let mut index = 0;
    while index < 32 {
        out[index] = (nibble(digits[2 * index]) << 4) | nibble(digits[2 * index + 1]);
        index += 1;
    }
    out
}

const fn nibble(digit: u8) -> u8 {
    match digit {
        b'0'..=b'9' => digit - b'0',
        b'a'..=b'f' => digit - b'a' + 10,
        _ => panic!("lowercase hex digit expected"),
    }
}

pub(super) fn unhex(text: &str) -> Vec<u8> {
    assert!(text.len().is_multiple_of(2), "odd hex length");
    (0..text.len() / 2)
        .map(|index| u8::from_str_radix(&text[2 * index..2 * index + 2], 16).unwrap())
        .collect()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn previous_point_check(point: &[u8]) -> bool {
    point.len() == 65
        && point.first() == Some(&0x04)
        && p256::PublicKey::from_sec1_bytes(point).is_ok()
}

pub(super) fn previous_verify(public_key: &[u8], message: &[u8], signature: &[u8]) -> bool {
    public_key.len() == 65
        && public_key.first() == Some(&0x04)
        && signature::UnparsedPublicKey::new(&signature::ECDSA_P256_SHA256_ASN1, public_key)
            .verify(message, signature)
            .is_ok()
}

/// Both checks reach the same decision on `point`; returns it.
pub(super) fn point_decision(point: &[u8]) -> bool {
    let previous = previous_point_check(point);
    assert_eq!(
        es256::is_valid_public_key(point),
        previous,
        "public key {}",
        hex(point)
    );
    previous
}

/// Both verifiers reach the same decision on the triple; returns it.
pub(super) fn signature_decision(public_key: &[u8], message: &[u8], signature: &[u8]) -> bool {
    let previous = previous_verify(public_key, message, signature);
    assert_eq!(
        es256::verify(public_key, message, signature),
        previous,
        "public key {}, message {}, signature {}",
        hex(public_key),
        hex(message),
        hex(signature)
    );
    previous
}

fn point(x: &[u8], y: &[u8]) -> Vec<u8> {
    [&[0x04][..], x, y].concat()
}

/// `a + b` on 256-bit big-endian values; the sums used here do not overflow.
fn add(a: &[u8; 32], b: &[u8; 32]) -> [u8; 32] {
    let mut out = [0; 32];
    let mut carry = 0_u16;
    for index in (0..32).rev() {
        let sum = u16::from(a[index]) + u16::from(b[index]) + carry;
        out[index] = sum as u8;
        carry = sum >> 8;
    }
    assert_eq!(carry, 0, "256-bit overflow");
    out
}

/// `a - b` on 256-bit big-endian values, `a >= b`.
fn sub(a: &[u8; 32], b: &[u8; 32]) -> [u8; 32] {
    let mut out = [0; 32];
    let mut borrow = 0_i16;
    for index in (0..32).rev() {
        let mut difference = i16::from(a[index]) - i16::from(b[index]) - borrow;
        borrow = i16::from(difference < 0);
        difference += borrow * 256;
        out[index] = difference as u8;
    }
    assert_eq!(borrow, 0, "256-bit underflow");
    out
}

fn key_pair() -> EcdsaKeyPair {
    let rng = SystemRandom::new();
    let document =
        EcdsaKeyPair::generate_pkcs8(&signature::ECDSA_P256_SHA256_ASN1_SIGNING, &rng).unwrap();
    EcdsaKeyPair::from_pkcs8(
        &signature::ECDSA_P256_SHA256_ASN1_SIGNING,
        document.as_ref(),
        &rng,
    )
    .unwrap()
}

fn sign(key_pair: &EcdsaKeyPair, message: &[u8]) -> Vec<u8> {
    key_pair
        .sign(&SystemRandom::new(), message)
        .unwrap()
        .as_ref()
        .to_vec()
}

/// A DER INTEGER with the minimal encoding of the unsigned value.
fn der_integer(value: &[u8]) -> Vec<u8> {
    let start = value
        .iter()
        .position(|byte| *byte != 0)
        .unwrap_or(value.len() - 1);
    let mut body = value[start..].to_vec();
    if body[0] & 0x80 != 0 {
        body.insert(0, 0);
    }
    [vec![0x02, body.len() as u8], body].concat()
}

pub(super) fn der_signature(r: &[u8], s: &[u8]) -> Vec<u8> {
    let body = [der_integer(r), der_integer(s)].concat();
    [vec![0x30, body.len() as u8], body].concat()
}

/// `r` and `s` of a DER signature `ring` produced, as 32-byte values.
pub(super) fn signature_scalars(signature: &[u8]) -> ([u8; 32], [u8; 32]) {
    assert_eq!(signature[0], 0x30);
    assert_eq!(usize::from(signature[1]), signature.len() - 2);
    let mut rest = &signature[2..];
    let mut scalars = [[0_u8; 32]; 2];
    for scalar in &mut scalars {
        assert_eq!(rest[0], 0x02);
        let length = usize::from(rest[1]);
        let value = &rest[2..2 + length];
        let value = value.strip_prefix(&[0]).unwrap_or(value);
        scalar[32 - value.len()..].copy_from_slice(value);
        rest = &rest[2 + length..];
    }
    assert!(rest.is_empty());
    (scalars[0], scalars[1])
}

/// The low-S and high-S forms (`s` and `n - s`) of one signature.
pub(super) fn both_s_forms(signature: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let (r, s) = signature_scalars(signature);
    let negated = sub(&N, &s);
    let (low, high) = if s <= negated {
        (s, negated)
    } else {
        (negated, s)
    };
    (der_signature(&r, &low), der_signature(&r, &high))
}

#[test]
fn valid_signatures_in_both_s_forms_verify_and_wrong_messages_or_keys_do_not() {
    let other = key_pair();
    for round in 0..16_u8 {
        let key_pair = key_pair();
        let public_key = key_pair.public_key().as_ref();
        let message = [b"rullst webauthn message ".as_slice(), &[round]].concat();
        let (low, high) = both_s_forms(&sign(&key_pair, &message));
        assert!(point_decision(public_key));
        for signature in [&low, &high] {
            assert!(signature_decision(public_key, &message, signature));
            let mut wrong_message = message.clone();
            wrong_message[0] ^= 1;
            assert!(!signature_decision(public_key, &wrong_message, signature));
            assert!(!signature_decision(public_key, &message[1..], signature));
            assert!(!signature_decision(
                other.public_key().as_ref(),
                &message,
                signature
            ));
        }
    }
}

#[test]
fn malformed_der_signatures_are_rejected_by_both() {
    let key_pair = key_pair();
    let public_key = key_pair.public_key().as_ref();
    let message = b"signed authenticatorData and clientDataJSON digest";
    let (valid, _) = both_s_forms(&sign(&key_pair, message));
    assert!(signature_decision(public_key, message, &valid));
    let (r, s) = signature_scalars(&valid);
    let zero = [0_u8; 32];
    let one = hex32("0000000000000000000000000000000000000000000000000000000000000001");
    let with = |edit: &dyn Fn(&mut Vec<u8>)| {
        let mut signature = valid.clone();
        edit(&mut signature);
        signature
    };
    let r_der = der_integer(&r);
    let s_der = der_integer(&s);
    let sequence = |body: Vec<u8>| [vec![0x30, body.len() as u8], body].concat();
    let malformed = [
        ("trailing byte", with(&|sig| sig.push(0))),
        (
            "trailing integer",
            with(&|sig| sig.extend_from_slice(&[2, 1, 1])),
        ),
        ("outer length too long", with(&|sig| sig[1] += 1)),
        ("outer length too short", with(&|sig| sig[1] -= 1)),
        ("r length too long", with(&|sig| sig[3] += 1)),
        ("r length too short", with(&|sig| sig[3] -= 1)),
        ("truncated", valid[..valid.len() - 1].to_vec()),
        ("empty", Vec::new()),
        ("header only", vec![0x30, 0x00]),
        ("set instead of sequence", with(&|sig| sig[0] = 0x31)),
        ("bit string instead of integer", with(&|sig| sig[2] = 0x03)),
        (
            "long-form outer length (BER)",
            [vec![0x30, 0x81, valid[1]], valid[2..].to_vec()].concat(),
        ),
        (
            "indefinite outer length (BER)",
            [vec![0x30, 0x80], valid[2..].to_vec(), vec![0, 0]].concat(),
        ),
        (
            "r with a redundant leading zero",
            sequence(
                [
                    vec![0x02, r_der[1] + 1, 0x00],
                    r_der[2..].to_vec(),
                    s_der.clone(),
                ]
                .concat(),
            ),
        ),
        (
            "r negative",
            sequence(
                [
                    vec![0x02, 0x20, r[0] | 0x80],
                    r[1..].to_vec(),
                    s_der.clone(),
                ]
                .concat(),
            ),
        ),
        (
            "s negative",
            sequence(
                [
                    r_der.clone(),
                    vec![0x02, 0x20, s[0] | 0x80],
                    s[1..].to_vec(),
                ]
                .concat(),
            ),
        ),
        ("r zero", der_signature(&zero, &s)),
        ("s zero", der_signature(&r, &zero)),
        ("r and s zero", der_signature(&zero, &zero)),
        ("r equal to n", der_signature(&N, &s)),
        ("s equal to n", der_signature(&r, &N)),
        ("r equal to n + 1", der_signature(&add(&N, &one), &s)),
        ("s equal to n + 1", der_signature(&r, &add(&N, &one))),
        ("s equal to p", der_signature(&r, &P)),
        ("r one", der_signature(&one, &s)),
        ("r and s swapped", der_signature(&s, &r)),
        ("fixed-width r || s", [r, s].concat()),
    ];
    for (name, signature) in &malformed {
        assert!(
            !signature_decision(public_key, message, signature),
            "{name} was accepted"
        );
    }
}

#[test]
fn points_off_the_curve_out_of_range_or_not_uncompressed_are_rejected_by_both() {
    let key_pair = key_pair();
    let valid = key_pair.public_key().as_ref().to_vec();
    let (x, y) = (&valid[1..33], &valid[33..65]);
    let y: [u8; 32] = y.try_into().unwrap();
    let one = hex32("0000000000000000000000000000000000000000000000000000000000000001");
    let max = [0xff; 32];
    // Accepted: a generated key, its negation and the two small-coordinate points.
    for accepted in [
        valid.clone(),
        point(x, &sub(&P, &y)),
        point(&SMALL_X.0, &SMALL_X.1),
        point(&SMALL_X.0, &sub(&P, &SMALL_X.1)),
        point(&SMALL_Y.0, &SMALL_Y.1),
    ] {
        assert!(point_decision(&accepted), "{}", hex(&accepted));
    }
    let rejected = [
        ("y + 1, off the curve", point(x, &add(&y, &one))),
        ("(0, 0)", point(&[0; 32], &[0; 32])),
        ("x = p", point(&P, &y)),
        ("y = p", point(x, &P)),
        (
            "x = 5 + p, the same point unreduced",
            point(&add(&SMALL_X.0, &P), &SMALL_X.1),
        ),
        (
            "y = 1 + p, the same point unreduced",
            point(&SMALL_Y.0, &add(&SMALL_Y.1, &P)),
        ),
        ("all ones", point(&max, &max)),
        ("infinity (SEC1 0x00)", vec![0x00]),
        (
            "infinity padded to 65 bytes",
            [vec![0x00], vec![0; 64]].concat(),
        ),
        ("missing last byte", valid[..64].to_vec()),
        ("trailing byte", [valid.clone(), vec![0]].concat()),
        ("x only", valid[..33].to_vec()),
        ("empty", Vec::new()),
        ("hybrid 0x06", [vec![0x06], valid[1..].to_vec()].concat()),
        ("hybrid 0x07", [vec![0x07], valid[1..].to_vec()].concat()),
        ("wrong tag 0x05", [vec![0x05], valid[1..].to_vec()].concat()),
    ];
    for (name, candidate) in &rejected {
        assert!(!point_decision(candidate), "{name} was accepted");
    }
    // A bit flipped anywhere in a coordinate leaves the curve.
    for bit in (0..512).step_by(7) {
        let mut flipped = valid.clone();
        flipped[1 + bit / 8] ^= 1 << (bit % 8);
        assert!(!point_decision(&flipped), "bit {bit}");
    }
}

#[test]
fn compressed_and_der_keys_that_aws_lc_rs_would_parse_are_rejected() {
    let key_pair = key_pair();
    let valid = key_pair.public_key().as_ref().to_vec();
    let message = b"assertion";
    let signature = sign(&key_pair, message);
    let compressed = [vec![0x02 | (valid[64] & 1)], valid[1..33].to_vec()].concat();
    let spki = [
        unhex("3059301306072a8648ce3d020106082a8648ce3d030107034200"),
        valid.clone(),
    ]
    .concat();
    for encoding in [&compressed, &spki] {
        // The libraries disagree on these encodings without the 65-byte gate:
        // aws-lc-rs parses both, p256 parses only the compressed point.
        assert!(
            aws_lc_rs::signature::ParsedPublicKey::new(
                &aws_lc_rs::signature::ECDSA_P256_SHA256_ASN1,
                encoding
            )
            .is_ok()
        );
        assert!(
            aws_lc_rs::signature::UnparsedPublicKey::new(
                &aws_lc_rs::signature::ECDSA_P256_SHA256_ASN1,
                encoding
            )
            .verify(message, &signature)
            .is_ok()
        );
        assert!(!point_decision(encoding));
        assert!(!signature_decision(encoding, message, &signature));
    }
    assert!(p256::PublicKey::from_sec1_bytes(&compressed).is_ok());
    assert!(p256::PublicKey::from_sec1_bytes(&spki).is_err());
    assert!(signature_decision(&valid, message, &signature));
}

#[test]
fn random_corruptions_reach_the_same_decision() {
    // xorshift64*, seeded: the corruptions are the same on every run.
    let mut state = 0x9e37_79b9_7f4a_7c15_u64;
    let mut next = move || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state.wrapping_mul(0x2545_f491_4f6c_dd1d)
    };
    let key_pair = key_pair();
    let valid_key = key_pair.public_key().as_ref().to_vec();
    let message = b"random corruption round".to_vec();
    let valid_signature = sign(&key_pair, &message);
    for _ in 0..512 {
        let (mut public_key, mut text, mut signature) =
            (valid_key.clone(), message.clone(), valid_signature.clone());
        let choice = next();
        let target = match choice % 3 {
            0 => &mut public_key,
            1 => &mut text,
            _ => &mut signature,
        };
        match (choice >> 8) % 4 {
            0 => {
                let index = (next() as usize) % target.len();
                target[index] ^= 1 << (next() % 8);
            }
            1 => {
                let index = (next() as usize) % target.len();
                target[index] = next() as u8;
            }
            2 => target.push(next() as u8),
            _ => {
                target.pop();
            }
        }
        signature_decision(&public_key, &text, &signature);
        point_decision(&public_key);
    }
}

#[test]
fn assertions_with_high_s_and_low_s_signatures_both_complete() {
    let auth = PasskeyAuth::new(&PasskeyConfig::new(
        "Test App",
        "localhost",
        "http://localhost",
    ))
    .unwrap();
    let fixture = registration_fixture(&auth, "localhost", RegistrationOptions::default());
    let registered = auth
        .finish_register(&fixture.credential, &fixture.challenge)
        .unwrap();
    for (sign_count, high_s) in [(2, false), (3, true)] {
        let (mut assertion, challenge) = assertion_fixture(
            &auth,
            &registered,
            &fixture.key_pair,
            "localhost",
            0x05,
            sign_count,
        );
        let signature = URL_SAFE_NO_PAD
            .decode(&assertion.response.signature)
            .unwrap();
        let (low, high) = both_s_forms(&signature);
        assertion.response.signature = URL_SAFE_NO_PAD.encode(if high_s { high } else { low });
        let updated = auth
            .finish_authenticate(&assertion, &challenge, registered.clone())
            .unwrap();
        assert_eq!(updated.sign_count, sign_count);
    }
}
