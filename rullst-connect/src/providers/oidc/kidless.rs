//! Key selection for ID tokens whose JOSE header carries no `kid`.
//!
//! OIDC Core 10.1 requires `kid` only when the issuer's JWK Set holds more
//! than one key. A token without one is therefore verified only when the set
//! holds exactly one key, and that key must be usable to verify this token:
//! a signature key (`use` absent or `sig`, `key_ops` absent or including
//! `verify`), with any declared `alg` equal to the header `alg`, and of the
//! key type and curve that algorithm requires. Every other set fails closed.

use jsonwebtoken::Algorithm;
use jsonwebtoken::jwk::{
    AlgorithmParameters, EllipticCurve, Jwk, JwkSet, KeyOperations, PublicKeyUse,
};

use crate::error::ConnectError;

/// Returns the only key of `jwks` when it can verify a token signed with `alg`.
pub(super) fn sole_verification_key(jwks: &JwkSet, alg: Algorithm) -> Result<&Jwk, ConnectError> {
    let [jwk] = jwks.keys.as_slice() else {
        return Err(ConnectError::Provider(
            "OIDC id_token has no 'kid' header and the issuer's JWK Set does not hold exactly one key"
                .to_owned(),
        ));
    };
    if !verifies(jwk, alg) {
        return Err(ConnectError::Provider(
            "OIDC id_token has no 'kid' header and the issuer's only JWK cannot verify its algorithm"
                .to_owned(),
        ));
    }
    Ok(jwk)
}

fn verifies(jwk: &Jwk, alg: Algorithm) -> bool {
    let common = &jwk.common;
    let signature_use = matches!(common.public_key_use, None | Some(PublicKeyUse::Signature));
    let verify_operation = common
        .key_operations
        .as_ref()
        .is_none_or(|operations| operations.contains(&KeyOperations::Verify));
    let declared_alg = common.key_algorithm.is_none_or(|declared| {
        declared
            .to_string()
            .parse::<Algorithm>()
            .is_ok_and(|declared| declared == alg)
    });
    signature_use && verify_operation && declared_alg && key_type_fits(&jwk.algorithm, alg)
}

fn key_type_fits(parameters: &AlgorithmParameters, alg: Algorithm) -> bool {
    match (parameters, alg) {
        (AlgorithmParameters::RSA(_), Algorithm::RS256 | Algorithm::RS384 | Algorithm::RS512) => {
            true
        }
        (AlgorithmParameters::EllipticCurve(key), Algorithm::ES256) => {
            key.curve == EllipticCurve::P256
        }
        (AlgorithmParameters::EllipticCurve(key), Algorithm::ES384) => {
            key.curve == EllipticCurve::P384
        }
        (AlgorithmParameters::OctetKeyPair(key), Algorithm::EdDSA) => {
            key.curve == EllipticCurve::Ed25519
        }
        _ => false,
    }
}
