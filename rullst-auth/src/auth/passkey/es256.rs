//! ES256 (ECDSA over P-256 with SHA-256) through aws-lc-rs: the public-key
//! check of registration and storage, and the assertion signature check.
//!
//! A WebAuthn ES256 credential key is kept as the uncompressed SEC1 point
//! `0x04 || x || y` built from its COSE EC2 coordinates. Only that form is
//! accepted here: aws-lc-rs would also parse a compressed point or a
//! SubjectPublicKeyInfo document, which no stored [`Passkey`](super::Passkey)
//! holds.

use aws_lc_rs::signature::{ECDSA_P256_SHA256_ASN1, ParsedPublicKey, UnparsedPublicKey};

/// First byte of an uncompressed SEC1 point.
const UNCOMPRESSED: u8 = 0x04;
/// The two 32-byte coordinates that follow it on P-256.
const COORDINATE_BYTES: usize = 64;

fn is_uncompressed(point: &[u8]) -> bool {
    matches!(point, [UNCOMPRESSED, coordinates @ ..] if coordinates.len() == COORDINATE_BYTES)
}

/// Whether `point` is an uncompressed SEC1 encoding of a point on P-256:
/// both coordinates below the field prime and on the curve.
#[must_use]
pub(super) fn is_valid_public_key(point: &[u8]) -> bool {
    is_uncompressed(point) && ParsedPublicKey::new(&ECDSA_P256_SHA256_ASN1, point).is_ok()
}

/// Whether `signature`, an ASN.1 DER ECDSA signature as WebAuthn sends it, signs
/// the SHA-256 digest of `message` under `public_key`. High-S signatures are
/// accepted: WebAuthn does not require low-S, and authenticators produce both.
#[must_use]
pub(super) fn verify(public_key: &[u8], message: &[u8], signature: &[u8]) -> bool {
    is_uncompressed(public_key)
        && UnparsedPublicKey::new(&ECDSA_P256_SHA256_ASN1, public_key)
            .verify(message, signature)
            .is_ok()
}
