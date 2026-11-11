//! `cargo rullst iot:keygen` and `iot:sign` through the installed binary:
//! the JSON summaries, and a signed update that rullst-iot accepts.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use rullst_iot::{OtaError, OtaManager, OtaManifest};
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn rullst(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rullst"))
        .current_dir(dir)
        .args(args)
        .output()
        .expect("run the rullst binary")
}

fn json(output: &Output) -> serde_json::Value {
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("stdout is one JSON document")
}

fn accepts(
    public: [u8; 32],
    manifest: &[u8],
    signature: &[u8],
    image: &[u8],
) -> Result<(), OtaError> {
    let manifest = OtaManifest::from_signing_bytes(manifest)?;
    let mut ota = OtaManager::new_with_trusted_key("board-a", "1.0.0", 2, public)?;
    let mut verifier = ota.begin_update(&manifest, signature)?;
    for chunk in image.chunks(1000) {
        verifier.update(chunk)?;
    }
    verifier.finish()
}

#[test]
fn keygen_and_sign_produce_an_update_rullst_iot_accepts() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let image: Vec<u8> = (0..50_000u32).map(|i| (i % 253) as u8).collect();
    fs::write(root.join("fw.bin"), &image).unwrap();

    let keygen = json(&rullst(root, &["iot:keygen", "--out", "ota.key", "--json"]));
    assert_eq!(keygen["schema"], "rullst.iot-keygen.v1");
    let public_hex = keygen["public_key"].as_str().unwrap().to_string();
    let secret = fs::read_to_string(root.join("ota.key")).unwrap();
    // The summary names the secret file but never contains the secret.
    let seed_line = secret.lines().last().unwrap();
    assert_eq!(seed_line.len(), 64);
    assert!(!keygen.to_string().contains(seed_line));
    let human = rullst(root, &["iot:keygen", "--out", "x.key"]);
    assert!(human.status.success());
    let other_seed = fs::read_to_string(root.join("x.key")).unwrap();
    let other_seed = other_seed.lines().last().unwrap();
    assert!(!String::from_utf8_lossy(&human.stdout).contains(other_seed));
    assert!(!String::from_utf8_lossy(&human.stderr).contains(other_seed));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(root.join("ota.key"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    let sign = json(&rullst(
        root,
        &[
            "iot:sign",
            "--key",
            "ota.key",
            "--image",
            "fw.bin",
            "--target",
            "board-a",
            "--version",
            "1.1.0",
            "--counter",
            "3",
            "--json",
        ],
    ));
    assert_eq!(sign["schema"], "rullst.iot-sign.v1");
    assert_eq!(sign["public_key"], public_hex.as_str());
    assert_eq!(sign["firmware_len"], 50_000);
    assert_eq!(sign["verified"], true);
    let manifest = fs::read(root.join("fw.bin.manifest")).unwrap();
    let signature = fs::read(root.join("fw.bin.manifest.sig")).unwrap();
    assert_eq!(sign["manifest"]["bytes"], manifest.len());
    let public: [u8; 32] = fs::read(root.join("ota.key.pub.bin"))
        .unwrap()
        .try_into()
        .unwrap();
    assert_eq!(accepts(public, &manifest, &signature, &image), Ok(()));
    let mut tampered = image.clone();
    tampered[49_999] ^= 0x80;
    assert_eq!(
        accepts(public, &manifest, &signature, &tampered),
        Err(OtaError::FirmwareHashMismatch)
    );

    // Refusals: existing key, existing outputs, counter 0, a placeholder key.
    assert!(
        !rullst(root, &["iot:keygen", "--out", "ota.key"])
            .status
            .success()
    );
    assert_eq!(fs::read_to_string(root.join("ota.key")).unwrap(), secret);
    let again = rullst(
        root,
        &[
            "iot:sign",
            "--key",
            "ota.key",
            "--image",
            "fw.bin",
            "--target",
            "board-a",
            "--version",
            "1.1.0",
            "--counter",
            "3",
        ],
    );
    assert!(!again.status.success());
    assert!(String::from_utf8_lossy(&again.stderr).contains("--force"));
    let zero = rullst(
        root,
        &[
            "iot:sign",
            "--key",
            "ota.key",
            "--image",
            "fw.bin",
            "--target",
            "board-a",
            "--version",
            "1.1.0",
            "--counter",
            "0",
            "--out",
            "z.manifest",
        ],
    );
    assert!(!zero.status.success());
    assert!(!root.join("z.manifest").exists());
    fs::write(root.join("weak.key"), format!("{}\n", "00".repeat(32))).unwrap();
    let weak = rullst(
        root,
        &[
            "iot:sign",
            "--key",
            "weak.key",
            "--image",
            "fw.bin",
            "--target",
            "board-a",
            "--version",
            "1.1.0",
            "--counter",
            "3",
            "--out",
            "w.manifest",
        ],
    );
    assert!(!weak.status.success());
    assert!(!root.join("w.manifest").exists());
}
