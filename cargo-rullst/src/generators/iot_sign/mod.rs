//! `cargo rullst iot:keygen` and `cargo rullst iot:sign` (Experimental).
//!
//! `iot:keygen` writes an Ed25519 key pair for a firmware publisher.
//! `iot:sign` turns a firmware image into the canonical manifest bytes and
//! signature that a device parses with `OtaManifest::from_signing_bytes` and
//! verifies with `OtaManager::begin_update`. The encoding is `rullst-iot`'s
//! own, and every result is verified with `rullst-iot` before it is written.

mod key;
mod sign;
#[cfg(test)]
mod tests;

use crate::generators::output_guard::{reject_existing, write_output};
use clap::{Arg, ArgAction, ArgMatches, Command, value_parser};
use key::PublicKeyPaths;
use serde::Serialize;
use sign::{SignRequest, SignedUpdate};
use std::path::{Path, PathBuf};

pub(crate) const KEYGEN_SCHEMA: &str = "rullst.iot-keygen.v1";
pub(crate) const SIGN_SCHEMA: &str = "rullst.iot-sign.v1";
const FORCE_HINT: &str = "; pass --force to replace them";

/// Why `iot:keygen` or `iot:sign` refused or failed.
#[derive(Debug, thiserror::Error)]
pub(crate) enum IotSignError {
    #[error("{}: {source}", .path.display())]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("{} is not a rullst-iot signing key: {reason}", .path.display())]
    InvalidKeyFile { path: PathBuf, reason: &'static str },
    #[error(
        "the signing key's bytes are all equal, which looks like a placeholder; generate one with `cargo rullst iot:keygen`"
    )]
    PlaceholderKey,
    #[error("the signing key's public key is weak (a small-order point); a device refuses it")]
    WeakKey,
    #[error("{} is not a regular file", .0.display())]
    NotAFile(PathBuf),
    #[error("the firmware image {} is empty", .0.display())]
    EmptyImage(PathBuf),
    #[error(
        "--version {0:?} must be non-empty text without surrounding spaces or control characters"
    )]
    InvalidVersion(String),
    #[error("the manifest is invalid: {0}")]
    Ota(rullst_iot::OtaError),
    #[error("rullst-iot did not accept the signed update ({0}); nothing was written")]
    SelfCheck(rullst_iot::OtaError),
    #[error("the operating system's random number generator failed: {0}")]
    Random(String),
    #[error(transparent)]
    Output(#[from] std::io::Error),
    #[error("could not write the JSON summary: {0}")]
    Json(#[from] serde_json::Error),
}

fn json_arg() -> Arg {
    Arg::new("json")
        .long("json")
        .action(ArgAction::SetTrue)
        .help("Print a versioned JSON summary on standard output")
}

fn path_arg(id: &'static str, help: &'static str) -> Arg {
    Arg::new(id)
        .long(id)
        .value_name("FILE")
        .value_parser(value_parser!(PathBuf))
        .help(help)
}

pub(crate) fn keygen_command() -> Command {
    Command::new("iot:keygen")
        .about("Generate an Ed25519 key pair for signing rullst-iot firmware updates (Experimental)")
        .arg(path_arg(
            "out",
            "Secret key file to create (mode 0600 on Unix); the public key is written beside it as FILE.pub, FILE.pub.bin and FILE.pub.rs",
        )
        .required(true))
        .arg(
            Arg::new("force")
                .long("force")
                .action(ArgAction::SetTrue)
                .help("Replace existing key files"),
        )
        .arg(json_arg().help("Print a versioned JSON summary (rullst.iot-keygen.v1) on standard output"))
}

pub(crate) fn sign_command() -> Command {
    Command::new("iot:sign")
        .about("Sign a firmware image into a rullst-iot OTA manifest and signature (Experimental)")
        .arg(
            path_arg(
                "key",
                "Secret key file written by `cargo rullst iot:keygen`",
            )
            .required(true),
        )
        .arg(path_arg("image", "Firmware image to sign").required(true))
        .arg(
            Arg::new("target")
                .long("target")
                .value_name("TARGET")
                .required(true)
                .help("Device class the image is for, as the device's OtaManager is configured"),
        )
        .arg(
            Arg::new("version")
                .long("version")
                .value_name("VERSION")
                .required(true)
                .help("Firmware version text, for example 1.2.0"),
        )
        .arg(
            Arg::new("counter")
                .long("counter")
                .value_name("N")
                .required(true)
                .value_parser(value_parser!(u64).range(1..))
                .help(
                    "Anti-rollback counter: at least 1 and above the counter the device committed",
                ),
        )
        .arg(path_arg(
            "out",
            "Manifest file to write (default IMAGE.manifest); the signature goes to FILE.sig",
        ))
        .arg(
            Arg::new("force")
                .long("force")
                .action(ArgAction::SetTrue)
                .help("Replace an existing manifest and signature"),
        )
        .arg(
            json_arg()
                .help("Print a versioned JSON summary (rullst.iot-sign.v1) on standard output"),
        )
}

fn path(matches: &ArgMatches, id: &str) -> Option<PathBuf> {
    matches.get_one::<PathBuf>(id).cloned()
}

fn text(matches: &ArgMatches, id: &str) -> String {
    matches.get_one::<String>(id).cloned().unwrap_or_default()
}

/// Writes one output file, naming it in any error.
fn write(path: &Path, contents: &[u8], force: bool) -> Result<(), IotSignError> {
    write_output(path, contents, force).map_err(|source| IotSignError::Io {
        path: path.to_path_buf(),
        source,
    })
}

/// `path` with `suffix` appended to its file name.
fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

#[derive(Serialize)]
struct KeygenSummary {
    schema: &'static str,
    secret_key: String,
    public_key: String,
    public_key_files: PublicFiles,
}

#[derive(Serialize)]
struct PublicFiles {
    hex: String,
    raw: String,
    rust: String,
}

pub(crate) fn run_keygen(matches: &ArgMatches) -> Result<(), IotSignError> {
    let secret = path(matches, "out").unwrap_or_default();
    let force = matches.get_flag("force");
    let public_paths = PublicKeyPaths::beside(&secret);
    if !force {
        let mut planned = vec![secret.clone()];
        planned.extend(public_paths.all().iter().map(|path| path.to_path_buf()));
        reject_existing("key files", &planned, FORCE_HINT)?;
    }
    let seed = key::generate_seed()?;
    let signing_key = key::signing_key(&seed)?;
    let public = signing_key.verifying_key().to_bytes();
    key::write_secret(&secret, key::key_file_text(&seed).as_bytes(), force).map_err(|source| {
        IotSignError::Io {
            path: secret.clone(),
            source,
        }
    })?;
    let public_hex = key::public_key_hex(&public);
    write(
        &public_paths.hex,
        format!("{public_hex}\n").as_bytes(),
        force,
    )?;
    write(&public_paths.raw, &public, force)?;
    write(
        &public_paths.rust,
        key::public_key_rust(&public).as_bytes(),
        force,
    )?;

    let summary = KeygenSummary {
        schema: KEYGEN_SCHEMA,
        secret_key: secret.display().to_string(),
        public_key: public_hex,
        public_key_files: PublicFiles {
            hex: public_paths.hex.display().to_string(),
            raw: public_paths.raw.display().to_string(),
            rust: public_paths.rust.display().to_string(),
        },
    };
    if matches.get_flag("json") {
        println!("{}", serde_json::to_string_pretty(&summary)?);
    } else {
        println!("Generated an Ed25519 signing key for rullst-iot firmware updates.");
        println!(
            "  secret key  {} (keep it private and out of version control)",
            summary.secret_key
        );
        println!("  public key  {}", summary.public_key);
        println!(
            "  for devices {} (Rust const), {} (raw), {} (hex)",
            summary.public_key_files.rust,
            summary.public_key_files.raw,
            summary.public_key_files.hex
        );
    }
    Ok(())
}

#[derive(Serialize)]
struct SignSummary {
    schema: &'static str,
    target: String,
    version: String,
    rollback_counter: u64,
    firmware_len: u64,
    firmware_sha256: String,
    public_key: String,
    manifest: OutputFile,
    signature: OutputFile,
    verified: bool,
}

#[derive(Serialize)]
struct OutputFile {
    path: String,
    bytes: usize,
    hex: String,
}

fn sign_summary(signed: &SignedUpdate, manifest_path: &Path, signature_path: &Path) -> SignSummary {
    let manifest = &signed.manifest;
    SignSummary {
        schema: SIGN_SCHEMA,
        target: manifest.target().to_string(),
        version: manifest.version().to_string(),
        rollback_counter: manifest.rollback_counter(),
        firmware_len: manifest.firmware_len(),
        firmware_sha256: hex::encode(manifest.firmware_sha256()),
        public_key: hex::encode(signed.public_key),
        manifest: OutputFile {
            path: manifest_path.display().to_string(),
            bytes: signed.manifest_bytes.len(),
            hex: hex::encode(&signed.manifest_bytes),
        },
        signature: OutputFile {
            path: signature_path.display().to_string(),
            bytes: signed.signature.len(),
            hex: hex::encode(signed.signature),
        },
        verified: true,
    }
}

pub(crate) fn run_sign(matches: &ArgMatches) -> Result<(), IotSignError> {
    let image = path(matches, "image").unwrap_or_default();
    let manifest_path = path(matches, "out").unwrap_or_else(|| with_suffix(&image, ".manifest"));
    let signature_path = with_suffix(&manifest_path, ".sig");
    let force = matches.get_flag("force");
    if !force {
        reject_existing(
            "signed-update files",
            &[manifest_path.clone(), signature_path.clone()],
            FORCE_HINT,
        )?;
    }
    let signing_key = key::read_signing_key(&path(matches, "key").unwrap_or_default())?;
    let target = text(matches, "target");
    let version = text(matches, "version");
    let request = SignRequest {
        image: &image,
        target: &target,
        version: &version,
        counter: matches.get_one::<u64>("counter").copied().unwrap_or(0),
    };
    let signed = sign::sign_image(&signing_key, &request)?;
    write(&manifest_path, &signed.manifest_bytes, force)?;
    write(&signature_path, &signed.signature, force)?;

    let summary = sign_summary(&signed, &manifest_path, &signature_path);
    if matches.get_flag("json") {
        println!("{}", serde_json::to_string_pretty(&summary)?);
    } else {
        println!(
            "Signed {} for target {:?}, version {}, rollback counter {}.",
            image.display(),
            summary.target,
            summary.version,
            summary.rollback_counter
        );
        println!(
            "  image      {} bytes, SHA-256 {}",
            summary.firmware_len, summary.firmware_sha256
        );
        println!(
            "  manifest   {} ({} bytes)",
            summary.manifest.path, summary.manifest.bytes
        );
        println!("  signature  {} (64 bytes)", summary.signature.path);
        println!("  public key {}", summary.public_key);
        println!("  verified with rullst-iot's OtaManager::begin_update before writing");
    }
    Ok(())
}
