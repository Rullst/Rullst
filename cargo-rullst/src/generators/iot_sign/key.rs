//! Signing-key files for `iot:keygen` and `iot:sign`.
//!
//! The secret file holds one line of 64 hexadecimal digits, the 32-byte
//! Ed25519 seed, after a comment header. It is created with mode 0600 on Unix
//! and its contents are never printed. The public key is written beside it as
//! hexadecimal text, raw bytes and a Rust `const` a device embeds.

use super::IotSignError;
use crate::generators::output_guard::{reject_symlink, write_output};
use ed25519_dalek::SigningKey;
use rullst_iot::{OtaError, OtaManager};
use std::fmt::Write as _;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use zeroize::Zeroizing;

pub(super) const KEY_HEADER: &str =
    "# rullst-iot Ed25519 OTA signing key. SECRET: never commit, share or print it.";
/// A key file is a header and one line; anything larger is not one.
const MAX_KEY_FILE_BYTES: u64 = 4096;

/// The three public-key files written beside the secret file at `secret`.
pub(super) struct PublicKeyPaths {
    pub(super) hex: PathBuf,
    pub(super) raw: PathBuf,
    pub(super) rust: PathBuf,
}

impl PublicKeyPaths {
    pub(super) fn beside(secret: &Path) -> Self {
        let with = |suffix: &str| {
            let mut name = secret.as_os_str().to_owned();
            name.push(suffix);
            PathBuf::from(name)
        };
        Self {
            hex: with(".pub"),
            raw: with(".pub.bin"),
            rust: with(".pub.rs"),
        }
    }

    pub(super) fn all(&self) -> [&Path; 3] {
        [&self.hex, &self.raw, &self.rust]
    }
}

/// The text of a secret key file for `seed`.
pub(super) fn key_file_text(seed: &[u8; 32]) -> Zeroizing<String> {
    let mut text = Zeroizing::new(String::with_capacity(KEY_HEADER.len() + 66));
    text.push_str(KEY_HEADER);
    text.push('\n');
    for byte in seed {
        // Writing to a String cannot fail.
        let _ = write!(text, "{byte:02x}");
    }
    text.push('\n');
    text
}

/// The seed in the text of a key file: comment and blank lines are skipped,
/// and exactly one line of 64 hexadecimal digits must remain.
pub(super) fn parse_key_text(text: &str) -> Result<Zeroizing<[u8; 32]>, &'static str> {
    let mut lines = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'));
    let Some(line) = lines.next() else {
        return Err("it holds no key line");
    };
    if lines.next().is_some() {
        return Err("it holds more than one key line");
    }
    if line.len() != 64 {
        return Err("the key line is not 64 hexadecimal digits (a 32-byte Ed25519 seed)");
    }
    let mut seed = Zeroizing::new([0u8; 32]);
    hex::decode_to_slice(line, seed.as_mut_slice())
        .map_err(|_| "the key line is not hexadecimal")?;
    Ok(seed)
}

/// The signing key for `seed`, refused when the seed looks like a placeholder
/// (every byte equal) or when a device would refuse its public key.
pub(super) fn signing_key(seed: &[u8; 32]) -> Result<SigningKey, IotSignError> {
    if seed.iter().all(|byte| *byte == seed[0]) {
        return Err(IotSignError::PlaceholderKey);
    }
    let key = SigningKey::from_bytes(seed);
    // The device's own check: `OtaManager` refuses a weak or invalid key.
    match OtaManager::new_with_trusted_key("key-check", "0", 0, key.verifying_key().to_bytes()) {
        Ok(_) => Ok(key),
        Err(OtaError::InvalidTrustedKey) => Err(IotSignError::WeakKey),
        Err(error) => Err(IotSignError::Ota(error)),
    }
}

/// Reads and checks the secret key file at `path`.
pub(super) fn read_signing_key(path: &Path) -> Result<SigningKey, IotSignError> {
    let io = |source| IotSignError::Io {
        path: path.to_path_buf(),
        source,
    };
    let invalid = |reason: &'static str| IotSignError::InvalidKeyFile {
        path: path.to_path_buf(),
        reason,
    };
    let metadata = fs::metadata(path).map_err(io)?;
    if !metadata.is_file() {
        return Err(invalid("it is not a regular file"));
    }
    if metadata.len() > MAX_KEY_FILE_BYTES {
        return Err(invalid("it is larger than a key file"));
    }
    if let Some(warning) = shared_warning(path, &metadata) {
        eprintln!("{warning}");
    }
    let mut text = Zeroizing::new(String::new());
    fs::File::open(path)
        .and_then(|file| file.take(MAX_KEY_FILE_BYTES).read_to_string(&mut text))
        .map_err(io)?;
    let seed = parse_key_text(&text).map_err(invalid)?;
    signing_key(&seed)
}

/// Whether Unix permission bits let a group or other users read or write.
pub(super) fn is_shared(mode: u32) -> bool {
    mode & 0o077 != 0
}

/// The warning for a key file that other users can read, if any (Unix only;
/// elsewhere the file inherits its folder's access control).
pub(super) fn shared_warning(path: &Path, metadata: &fs::Metadata) -> Option<String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = metadata.permissions().mode() & 0o777;
        is_shared(mode).then(|| {
            format!(
                "warning: {} can be read by other users (mode {mode:o}); run `chmod 600` on it",
                path.display()
            )
        })
    }
    #[cfg(not(unix))]
    {
        let _ = (path, metadata);
        None
    }
}

/// 32 bytes from the operating system's random number generator.
pub(super) fn generate_seed() -> Result<Zeroizing<[u8; 32]>, IotSignError> {
    use rand::{TryRng, rngs::SysRng};
    let mut seed = Zeroizing::new([0u8; 32]);
    SysRng
        .try_fill_bytes(seed.as_mut_slice())
        .map_err(|error| IotSignError::Random(error.to_string()))?;
    Ok(seed)
}

/// Writes the secret key file: a new file with mode 0600 on Unix or, with
/// `force`, a replacement of an existing one, made private first and then
/// replaced through a sibling temporary file that keeps that mode.
pub(super) fn write_secret(path: &Path, contents: &[u8], force: bool) -> io::Result<()> {
    reject_symlink(path)?;
    if force && path.is_file() {
        restrict(path)?;
        return write_output(path, contents, true);
    }
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    if let Err(error) = file.write_all(contents).and_then(|()| file.sync_all()) {
        drop(file);
        let _ = fs::remove_file(path);
        return Err(error);
    }
    Ok(())
}

#[cfg(unix)]
fn restrict(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
fn restrict(_path: &Path) -> io::Result<()> {
    Ok(())
}

/// The public key as lowercase hexadecimal.
pub(super) fn public_key_hex(public: &[u8; 32]) -> String {
    hex::encode(public)
}

/// A Rust `const` with the public key, for a device to embed.
pub(super) fn public_key_rust(public: &[u8; 32]) -> String {
    let mut out = String::from(
        "// The Ed25519 public key of a rullst-iot OTA publisher, written by\n\
         // `cargo rullst iot:keygen`. Pass it to `OtaManager` as the trusted key.\n\
         pub const OTA_PUBLIC_KEY: [u8; 32] = [\n",
    );
    for row in public.chunks(8) {
        let bytes: Vec<String> = row.iter().map(|byte| format!("0x{byte:02x},")).collect();
        let _ = writeln!(out, "    {}", bytes.join(" "));
    }
    out.push_str("];\n");
    out
}
