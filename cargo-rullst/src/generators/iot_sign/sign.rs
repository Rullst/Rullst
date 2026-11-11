//! Builds and signs a rullst-iot OTA manifest with the crate's own encoding,
//! then checks the result with the device's own verifier.

use super::IotSignError;
use ed25519_dalek::{Signer, SigningKey};
use rullst_iot::{OtaManager, OtaManifest};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// Read size for hashing and verifying the image.
const CHUNK: usize = 65_536;

/// What `iot:sign` signs.
pub(super) struct SignRequest<'a> {
    pub(super) image: &'a Path,
    pub(super) target: &'a str,
    pub(super) version: &'a str,
    pub(super) counter: u64,
}

/// A signed update: the canonical manifest bytes and their signature.
pub(super) struct SignedUpdate {
    pub(super) manifest: OtaManifest,
    pub(super) manifest_bytes: Vec<u8>,
    pub(super) signature: [u8; 64],
    pub(super) public_key: [u8; 32],
}

/// Signs `request.image` with `key`.
///
/// The manifest is `rullst_iot::OtaManifest::new` over the image's length and
/// SHA-256, its bytes are `signing_bytes()`, and the result is accepted only
/// after `OtaManifest::from_signing_bytes` and `OtaManager::begin_update`, fed
/// the image again in chunks, accept it, as a device would.
pub(super) fn sign_image(
    key: &SigningKey,
    request: &SignRequest<'_>,
) -> Result<SignedUpdate, IotSignError> {
    check_version(request.version)?;
    let (firmware_len, firmware_sha256) = hash_image(request.image)?;
    let manifest = OtaManifest::new(
        request.target,
        request.version,
        request.counter,
        firmware_len,
        firmware_sha256,
    )
    .map_err(IotSignError::Ota)?;
    let manifest_bytes = manifest.signing_bytes().map_err(IotSignError::Ota)?;
    let signature = key.sign(&manifest_bytes).to_bytes();
    let public_key = key.verifying_key().to_bytes();
    let signed = SignedUpdate {
        manifest,
        manifest_bytes,
        signature,
        public_key,
    };
    self_check(&signed, request.image)?;
    Ok(signed)
}

/// The version text a person reads on the device: non-empty, without
/// surrounding spaces or control characters.
fn check_version(version: &str) -> Result<(), IotSignError> {
    if version.is_empty() || version.trim() != version || version.chars().any(char::is_control) {
        return Err(IotSignError::InvalidVersion(version.to_string()));
    }
    Ok(())
}

fn open(path: &Path) -> Result<File, IotSignError> {
    let file = File::open(path).map_err(|source| IotSignError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let regular = file
        .metadata()
        .map(|metadata| metadata.is_file())
        .unwrap_or(false);
    if !regular {
        return Err(IotSignError::NotAFile(path.to_path_buf()));
    }
    Ok(file)
}

/// Reads `path` in chunks, passing each to `each`.
fn for_each_chunk(
    path: &Path,
    mut each: impl FnMut(&[u8]) -> Result<(), IotSignError>,
) -> Result<(), IotSignError> {
    let mut file = open(path)?;
    let mut buffer = vec![0u8; CHUNK];
    loop {
        let read = file.read(&mut buffer).map_err(|source| IotSignError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        if read == 0 {
            return Ok(());
        }
        each(&buffer[..read])?;
    }
}

/// The image's length and SHA-256 digest; an empty image is refused.
fn hash_image(path: &Path) -> Result<(u64, [u8; 32]), IotSignError> {
    let mut hasher = Sha256::new();
    let mut len = 0u64;
    for_each_chunk(path, |chunk| {
        hasher.update(chunk);
        len = len.saturating_add(chunk.len() as u64);
        Ok(())
    })?;
    if len == 0 {
        return Err(IotSignError::EmptyImage(path.to_path_buf()));
    }
    Ok((len, hasher.finalize().into()))
}

/// Verifies `signed` exactly as a device does: it parses the written manifest
/// bytes and streams the image through `OtaManager::begin_update`.
pub(super) fn self_check(signed: &SignedUpdate, image: &Path) -> Result<(), IotSignError> {
    let check = |error| IotSignError::SelfCheck(error);
    let parsed = OtaManifest::from_signing_bytes(&signed.manifest_bytes).map_err(check)?;
    if parsed != signed.manifest {
        return Err(IotSignError::SelfCheck(
            rullst_iot::OtaError::MalformedManifest,
        ));
    }
    let mut ota = OtaManager::new_with_trusted_key(
        parsed.target(),
        "0",
        parsed.rollback_counter().saturating_sub(1),
        signed.public_key,
    )
    .map_err(check)?;
    let mut verifier = ota
        .begin_update(&parsed, &signed.signature)
        .map_err(check)?;
    for_each_chunk(image, |chunk| verifier.update(chunk).map_err(check))?;
    verifier.finish().map_err(check)
}
