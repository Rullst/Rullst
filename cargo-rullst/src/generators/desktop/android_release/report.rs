//! Accept only the two observed SDK signer formats, with an exact certificate.
use super::ReleaseError;

pub(super) fn verify(body: &[u8], expected: &str) -> Result<(), ReleaseError> {
    let text = std::str::from_utf8(body).map_err(|_| ReleaseError::Report)?;
    let mut signers = 0;
    let mut certificates = 0;
    for line in text.lines() {
        if line.starts_with("Number of signers:") {
            if line != "Number of signers: 1" {
                return Err(ReleaseError::Report);
            }
            signers += 1;
        }
        if line.contains("certificate SHA-256 digest:") {
            let digest = line
                .strip_prefix("Signer #1 certificate SHA-256 digest: ")
                .or_else(|| line.strip_prefix("V2 Signer: certificate SHA-256 digest: "))
                .ok_or(ReleaseError::Report)?;
            if digest.len() != 64
                || !digest.bytes().all(|value| value.is_ascii_hexdigit())
                || !digest.eq_ignore_ascii_case(expected)
            {
                return Err(ReleaseError::Report);
            }
            certificates += 1;
        }
    }
    if signers != 1 || certificates == 0 {
        return Err(ReleaseError::Report);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_signer_certificate_cannot_be_replaced_by_key_stamp_or_another_signer() {
        let digest = "ab".repeat(32);
        for label in ["Signer #1", "V2 Signer:"] {
            let valid = format!(
                "Verifies\nNumber of signers: 1\n{label} certificate SHA-256 digest: {digest}\n"
            );
            assert!(verify(valid.as_bytes(), &digest).is_ok());
            assert!(
                verify(
                    valid.replace(&digest, &digest.to_uppercase()).as_bytes(),
                    &digest
                )
                .is_ok()
            );
            for invalid in [
                valid.replace("Number of signers: 1", "Number of signers: 2"),
                valid.replace("Number of signers: 1\n", ""),
                format!("{valid}Number of signers: 1\n"),
                valid.replace("certificate SHA-256", "public key SHA-256"),
                valid.replace(label, "Source Stamp Signer"),
                valid.replace(label, "Signer #2"),
                valid.replace(&digest, &"00".repeat(32)),
                format!("{valid}Signer #2 certificate SHA-256 digest: {digest}\n"),
            ] {
                assert!(verify(invalid.as_bytes(), &digest).is_err());
            }
        }
    }
}
