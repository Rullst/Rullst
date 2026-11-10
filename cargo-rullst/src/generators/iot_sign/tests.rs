#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::key::{self, PublicKeyPaths};
use super::sign::{self, SignRequest};
use super::*;
use ed25519_dalek::SigningKey;
use rullst_iot::{OtaError, OtaManager, OtaManifest, OtaStatus};
use std::fs;

const SEED: [u8; 32] = [
    0x9d, 0x61, 0xb1, 0x9d, 0xef, 0xfd, 0x5a, 0x60, 0xba, 0x84, 0x4a, 0xf4, 0x92, 0xec, 0x2c, 0xc4,
    0x44, 0x49, 0xc5, 0x69, 0x7b, 0x32, 0x69, 0x19, 0x70, 0x3b, 0xac, 0x03, 0x1c, 0xae, 0x7f, 0x60,
];

fn image(dir: &Path, len: usize) -> PathBuf {
    let path = dir.join("firmware.bin");
    let bytes: Vec<u8> = (0..len).map(|index| (index * 31 % 251) as u8).collect();
    fs::write(&path, bytes).unwrap();
    path
}

fn request<'a>(image: &'a Path, version: &'a str, counter: u64) -> SignRequest<'a> {
    SignRequest {
        image,
        target: "microbit-v2",
        version,
        counter,
    }
}

/// Verifies like a device: parse the bytes, begin, stream, finish.
fn device_accepts(
    public: [u8; 32],
    counter: u64,
    manifest: &[u8],
    signature: &[u8],
    firmware: &[u8],
) -> Result<(), OtaError> {
    let manifest = OtaManifest::from_signing_bytes(manifest)?;
    let mut ota = OtaManager::new_with_trusted_key("microbit-v2", "1.0.0", counter, public)?;
    let mut verifier = ota.begin_update(&manifest, signature)?;
    for chunk in firmware.chunks(4096) {
        verifier.update(chunk)?;
    }
    verifier.finish()?;
    assert_eq!(ota.status(), OtaStatus::Verified);
    Ok(())
}

#[test]
fn key_files_round_trip_and_ignore_comments() {
    let text = key::key_file_text(&SEED);
    assert!(text.starts_with(key::KEY_HEADER));
    assert_eq!(text.lines().count(), 2);
    assert_eq!(*key::parse_key_text(&text).unwrap(), SEED);
    let spaced = format!("\n# a note\n\n  {}  \n# end\n", hex::encode(SEED));
    assert_eq!(*key::parse_key_text(&spaced).unwrap(), SEED);
    assert_eq!(*key::parse_key_text(&hex::encode(SEED)).unwrap(), SEED);
}

#[test]
fn malformed_key_files_are_refused_with_a_reason() {
    let line = hex::encode(SEED);
    for (text, reason) in [
        ("", "it holds no key line"),
        ("# only a comment\n", "it holds no key line"),
        (
            &format!("{line}\n{line}\n")[..],
            "it holds more than one key line",
        ),
        (
            &line[..62],
            "the key line is not 64 hexadecimal digits (a 32-byte Ed25519 seed)",
        ),
        (
            &format!("{line}00")[..],
            "the key line is not 64 hexadecimal digits (a 32-byte Ed25519 seed)",
        ),
        (
            &format!("{}zz", &line[..62])[..],
            "the key line is not hexadecimal",
        ),
    ] {
        assert_eq!(key::parse_key_text(text).err(), Some(reason), "{text:?}");
    }
}

#[test]
fn placeholder_seeds_are_refused_and_real_ones_accepted() {
    for byte in [0u8, 0x42, 0xff] {
        assert!(matches!(
            key::signing_key(&[byte; 32]),
            Err(IotSignError::PlaceholderKey)
        ));
    }
    let mut almost = [7u8; 32];
    almost[31] = 8;
    assert!(key::signing_key(&almost).is_ok());
    let key = key::signing_key(&SEED).unwrap();
    assert_eq!(key.to_bytes(), SEED);
}

#[test]
fn generated_seeds_are_random_and_usable() {
    let first = key::generate_seed().unwrap();
    let second = key::generate_seed().unwrap();
    assert_ne!(*first, *second);
    assert!(key::signing_key(&first).is_ok());
}

#[test]
fn public_key_files_sit_beside_the_secret_and_render_for_devices() {
    let paths = PublicKeyPaths::beside(Path::new("keys/ota.key"));
    assert_eq!(paths.hex, Path::new("keys/ota.key.pub"));
    assert_eq!(paths.raw, Path::new("keys/ota.key.pub.bin"));
    assert_eq!(paths.rust, Path::new("keys/ota.key.pub.rs"));
    assert_eq!(
        paths.all(),
        [&paths.hex, &paths.raw, &paths.rust].map(PathBuf::as_path)
    );

    let public = SigningKey::from_bytes(&SEED).verifying_key().to_bytes();
    assert_eq!(key::public_key_hex(&public), hex::encode(public));
    let rust = key::public_key_rust(&public);
    let file = syn::parse_file(&rust).unwrap();
    assert_eq!(file.items.len(), 1);
    let bytes: Vec<u8> = rust
        .split("0x")
        .skip(1)
        .map(|byte| u8::from_str_radix(&byte[..2], 16).unwrap())
        .collect();
    assert_eq!(bytes, public);
    assert!(rust.contains("pub const OTA_PUBLIC_KEY: [u8; 32] = ["));
}

#[test]
fn secret_files_are_private_new_files_unless_forced() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ota.key");
    key::write_secret(&path, b"first", false).unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"first");
    let refused = key::write_secret(&path, b"second", false).unwrap_err();
    assert_eq!(refused.kind(), std::io::ErrorKind::AlreadyExists);
    assert_eq!(fs::read(&path).unwrap(), b"first");
    key::write_secret(&path, b"second", true).unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"second");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        // A forced replacement makes a shared key file private again.
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        key::write_secret(&path, b"third", true).unwrap();
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(fs::read(&path).unwrap(), b"third");
        let fresh = dir.path().join("fresh.key");
        key::write_secret(&fresh, b"x", true).unwrap();
        assert_eq!(
            fs::metadata(&fresh).unwrap().permissions().mode() & 0o777,
            0o600
        );
        let link = dir.path().join("link.key");
        std::os::unix::fs::symlink(&path, &link).unwrap();
        assert!(key::write_secret(&link, b"x", true).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"third");
    }
}

#[test]
fn key_files_are_read_back_and_bad_ones_refused() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ota.key");
    fs::write(&path, key::key_file_text(&SEED).as_bytes()).unwrap();
    assert_eq!(key::read_signing_key(&path).unwrap().to_bytes(), SEED);

    assert!(matches!(
        key::read_signing_key(dir.path()),
        Err(IotSignError::InvalidKeyFile {
            reason: "it is not a regular file",
            ..
        })
    ));
    // A key file of exactly the size limit is read; one byte more is not.
    let mut padded = key::key_file_text(&SEED).to_string();
    padded.push('#');
    while padded.len() < 4095 {
        padded.push('x');
    }
    padded.push('\n');
    assert_eq!(padded.len(), 4096);
    fs::write(&path, &padded).unwrap();
    assert_eq!(key::read_signing_key(&path).unwrap().to_bytes(), SEED);
    padded.insert(padded.len() - 1, 'x');
    fs::write(&path, &padded).unwrap();
    assert!(matches!(
        key::read_signing_key(&path),
        Err(IotSignError::InvalidKeyFile {
            reason: "it is larger than a key file",
            ..
        })
    ));
    fs::write(&path, vec![b'#'; 5000]).unwrap();
    assert!(matches!(
        key::read_signing_key(&path),
        Err(IotSignError::InvalidKeyFile {
            reason: "it is larger than a key file",
            ..
        })
    ));
    fs::write(&path, "not a key\n").unwrap();
    assert!(matches!(
        key::read_signing_key(&path),
        Err(IotSignError::InvalidKeyFile { .. })
    ));
    fs::write(&path, key::key_file_text(&[3; 32]).as_bytes()).unwrap();
    assert!(matches!(
        key::read_signing_key(&path),
        Err(IotSignError::PlaceholderKey)
    ));
    assert!(matches!(
        key::read_signing_key(&dir.path().join("missing.key")),
        Err(IotSignError::Io { .. })
    ));
}

#[test]
fn signed_updates_are_what_a_device_accepts() {
    let dir = tempfile::tempdir().unwrap();
    let image = image(dir.path(), 70_001);
    let firmware = fs::read(&image).unwrap();
    let key = SigningKey::from_bytes(&SEED);
    let signed = sign::sign_image(&key, &request(&image, "1.2.0", 5)).unwrap();

    assert_eq!(
        signed.manifest,
        OtaManifest::from_firmware("microbit-v2", "1.2.0", 5, &firmware).unwrap()
    );
    assert_eq!(
        signed.manifest_bytes,
        signed.manifest.signing_bytes().unwrap()
    );
    assert_eq!(signed.public_key, key.verifying_key().to_bytes());
    let accepted = device_accepts(
        signed.public_key,
        4,
        &signed.manifest_bytes,
        &signed.signature,
        &firmware,
    );
    assert_eq!(accepted, Ok(()));

    // Tampered image, manifest, signature or an older device counter fail.
    let mut changed = firmware.clone();
    changed[65_536] ^= 1;
    assert_eq!(
        device_accepts(
            signed.public_key,
            4,
            &signed.manifest_bytes,
            &signed.signature,
            &changed
        ),
        Err(OtaError::FirmwareHashMismatch)
    );
    let mut manifest = signed.manifest_bytes.clone();
    let last = manifest.len() - 1;
    manifest[last] ^= 1;
    assert_eq!(
        device_accepts(
            signed.public_key,
            4,
            &manifest,
            &signed.signature,
            &firmware
        ),
        Err(OtaError::SignatureInvalid)
    );
    let mut signature = signed.signature;
    signature[0] ^= 1;
    assert_eq!(
        device_accepts(
            signed.public_key,
            4,
            &signed.manifest_bytes,
            &signature,
            &firmware
        ),
        Err(OtaError::SignatureInvalid)
    );
    assert_eq!(
        device_accepts(
            signed.public_key,
            5,
            &signed.manifest_bytes,
            &signed.signature,
            &firmware
        ),
        Err(OtaError::RollbackDetected {
            current: 5,
            proposed: 5
        })
    );

    let summary = sign_summary(&signed, Path::new("m"), Path::new("m.sig"));
    let json = serde_json::to_value(&summary).unwrap();
    assert_eq!(json["schema"], SIGN_SCHEMA);
    assert_eq!(json["rollback_counter"], 5);
    assert_eq!(json["firmware_len"], 70_001);
    assert_eq!(json["manifest"]["bytes"], signed.manifest_bytes.len());
    assert_eq!(json["signature"]["hex"], hex::encode(signed.signature));
    assert_eq!(json["verified"], true);
}

#[test]
fn bad_sign_requests_are_refused() {
    let dir = tempfile::tempdir().unwrap();
    let key = SigningKey::from_bytes(&SEED);
    let image = image(dir.path(), 10);
    for version in ["", " 1.0", "1.0 ", "1.0\n", "1\u{7}"] {
        assert!(
            matches!(
                sign::sign_image(&key, &request(&image, version, 1)),
                Err(IotSignError::InvalidVersion(_))
            ),
            "{version:?}"
        );
    }
    let empty = dir.path().join("empty.bin");
    fs::write(&empty, b"").unwrap();
    assert!(matches!(
        sign::sign_image(&key, &request(&empty, "1", 1)),
        Err(IotSignError::EmptyImage(_))
    ));
    assert!(matches!(
        sign::sign_image(&key, &request(dir.path(), "1", 1)),
        Err(IotSignError::NotAFile(_))
    ));
    assert!(matches!(
        sign::sign_image(&key, &request(&dir.path().join("missing.bin"), "1", 1)),
        Err(IotSignError::Io { .. })
    ));
    let no_target = SignRequest {
        image: &image,
        target: "",
        version: "1",
        counter: 1,
    };
    assert!(matches!(
        sign::sign_image(&key, &no_target),
        Err(IotSignError::Ota(OtaError::EmptyTarget))
    ));
}

fn run_keygen_in(dir: &Path, extra: &[&str]) -> Result<(), IotSignError> {
    let out = dir.join("ota.key");
    let mut args = vec!["iot:keygen", "--out", out.to_str().unwrap()];
    args.extend_from_slice(extra);
    run_keygen(&keygen_command().try_get_matches_from(args).unwrap())
}

#[test]
fn keygen_writes_a_key_pair_and_never_overwrites_without_force() {
    let dir = tempfile::tempdir().unwrap();
    run_keygen_in(dir.path(), &["--json"]).unwrap();
    let secret = dir.path().join("ota.key");
    let signing = key::read_signing_key(&secret).unwrap();
    let public = signing.verifying_key().to_bytes();
    let paths = PublicKeyPaths::beside(&secret);
    assert_eq!(fs::read(&paths.raw).unwrap(), public);
    assert_eq!(
        fs::read_to_string(&paths.hex).unwrap(),
        format!("{}\n", hex::encode(public))
    );
    assert_eq!(
        fs::read_to_string(&paths.rust).unwrap(),
        key::public_key_rust(&public)
    );

    let before = fs::read(&secret).unwrap();
    let refused = run_keygen_in(dir.path(), &[]).unwrap_err();
    assert!(
        refused
            .to_string()
            .contains("refusing to overwrite existing key files")
    );
    assert_eq!(fs::read(&secret).unwrap(), before);
    // A public file alone is enough to refuse.
    fs::remove_file(&secret).unwrap();
    assert!(run_keygen_in(dir.path(), &[]).is_err());
    assert!(!secret.exists());

    run_keygen_in(dir.path(), &["--force"]).unwrap();
    assert_ne!(fs::read(&secret).unwrap(), before);
    let replaced = key::read_signing_key(&secret)
        .unwrap()
        .verifying_key()
        .to_bytes();
    assert_eq!(fs::read(&paths.raw).unwrap(), replaced);
}

#[test]
fn sign_writes_the_manifest_and_signature_a_device_accepts() {
    let dir = tempfile::tempdir().unwrap();
    run_keygen_in(dir.path(), &[]).unwrap();
    let secret = dir.path().join("ota.key");
    let image = image(dir.path(), 4_097);
    let sign = |extra: &[&str]| {
        let mut args = vec![
            "iot:sign",
            "--key",
            secret.to_str().unwrap(),
            "--image",
            image.to_str().unwrap(),
            "--target",
            "microbit-v2",
            "--version",
            "2.0.0",
            "--counter",
            "9",
        ];
        args.extend_from_slice(extra);
        run_sign(&sign_command().try_get_matches_from(args).unwrap())
    };
    sign(&["--json"]).unwrap();
    let manifest = fs::read(with_suffix(&image, ".manifest")).unwrap();
    let signature = fs::read(with_suffix(&image, ".manifest.sig")).unwrap();
    let public: [u8; 32] = fs::read(dir.path().join("ota.key.pub.bin"))
        .unwrap()
        .try_into()
        .unwrap();
    let firmware = fs::read(&image).unwrap();
    assert_eq!(
        device_accepts(public, 8, &manifest, &signature, &firmware),
        Ok(())
    );
    let parsed = OtaManifest::from_signing_bytes(&manifest).unwrap();
    assert_eq!((parsed.version(), parsed.rollback_counter()), ("2.0.0", 9));

    let refused = sign(&[]).unwrap_err();
    assert!(
        refused
            .to_string()
            .contains("refusing to overwrite existing signed-update files")
    );
    let out = dir.path().join("custom.ota");
    sign(&["--out", out.to_str().unwrap()]).unwrap();
    assert_eq!(fs::read(&out).unwrap(), manifest);
    assert_eq!(fs::read(with_suffix(&out, ".sig")).unwrap().len(), 64);
    sign(&["--force"]).unwrap();

    // Counter 0 and a missing flag never reach signing.
    let zero = sign_command().try_get_matches_from([
        "iot:sign",
        "--key",
        "k",
        "--image",
        "i",
        "--target",
        "t",
        "--version",
        "1",
        "--counter",
        "0",
    ]);
    assert!(zero.is_err());
    assert!(
        sign_command()
            .try_get_matches_from(["iot:sign", "--key", "k", "--image", "i", "--target", "t"])
            .is_err()
    );
}

#[test]
fn the_commands_are_valid_clap_definitions() {
    keygen_command().debug_assert();
    sign_command().debug_assert();
    assert_eq!(
        with_suffix(Path::new("a/b.bin"), ".manifest"),
        Path::new("a/b.bin.manifest")
    );
}

#[test]
fn group_or_other_permission_bits_count_as_shared() {
    for mode in [0o600, 0o400, 0o700] {
        assert!(!key::is_shared(mode), "{mode:o}");
    }
    for mode in [0o640, 0o604, 0o644, 0o660, 0o606, 0o610, 0o601] {
        assert!(key::is_shared(mode), "{mode:o}");
    }
}
