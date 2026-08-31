//! Canonical digest helpers backed by the crate's existing crypto dependency.

use sha2::{Digest, Sha256};

/// Returns the lowercase hexadecimal SHA-256 digest for integrity identifiers.
///
/// SHA-256 alone is not suitable for password storage, authentication tags or
/// secret verification. Use Argon2, HMAC or the dedicated Rullst APIs for those
/// purposes.
pub fn sha256_hex(input: impl AsRef<[u8]>) -> String {
    let digest = Sha256::digest(input.as_ref());
    lowercase_hex(digest.as_ref())
}

fn lowercase_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::sha256_hex;

    #[test]
    fn sha256_hex_is_canonical_and_accepts_owned_or_borrowed_bytes() {
        let expected = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        assert_eq!(sha256_hex("abc"), expected);
        let owned = String::from("abc");
        assert_eq!(sha256_hex(owned), expected);
        assert_eq!(
            sha256_hex(""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
