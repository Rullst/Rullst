//! 7-bit ASCII sender and recipient forms for SES v2 `SendEmail`.
//!
//! SES does not support the SMTPUTF8 extension (RFC 6531), so every address
//! string must be 7-bit ASCII: the local part must already be ASCII, an
//! internationalized domain is sent as its Punycode (IDNA A-label) form, and
//! a non-ASCII "friendly from" display name is sent as RFC 2047 MIME
//! encoded-words. Sources: the SES v2 `Destination` note
//! (<https://docs.aws.amazon.com/ses/latest/APIReference-V2/API_Destination.html>)
//! and the SES `SendEmail` `Source` parameter
//! (<https://docs.aws.amazon.com/ses/latest/APIReference/API_SendEmail.html>);
//! the SES v2 `FromEmailAddress` reference does not restate the rule.
//! Errors never include the address.

use crate::error::MailError;
use crate::validator::mailbox_parts;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;

/// RFC 2047 caps an encoded-word at 75 characters. `=?UTF-8?B?` and `?=`
/// take 12, leaving 63 base64 characters, which encode at most 45 bytes.
const MAX_ENCODED_WORD_BYTES: usize = 45;

/// The recipient as SES accepts it: a bare address with an ASCII domain.
pub(super) fn recipient(to: &str) -> Result<String, MailError> {
    let (address, _) = mailbox_parts(to).map_err(|_| invalid())?;
    ascii_address(address)
}

/// The sender as SES accepts it. An all-ASCII value is sent unchanged.
pub(super) fn sender(from: &str) -> Result<String, MailError> {
    if from.is_ascii() {
        return Ok(from.to_string());
    }
    let (address, name) = mailbox_parts(from).map_err(|_| invalid())?;
    let address = ascii_address(address)?;
    Ok(match name {
        Some(name) => format!("{} <{address}>", display_name(&name)),
        None => address,
    })
}

fn ascii_address(address: &str) -> Result<String, MailError> {
    let (local, domain) = address.rsplit_once('@').ok_or_else(invalid)?;
    if !local.is_ascii() {
        return Err(MailError::ValidationError(
            "AWS SES requires an ASCII local part because it does not support SMTPUTF8".to_string(),
        ));
    }
    if domain.is_ascii() {
        return Ok(address.to_string());
    }
    let domain = idna::domain_to_ascii(domain).map_err(|_| {
        MailError::ValidationError(
            "AWS SES requires an address domain with a valid IDNA A-label".to_string(),
        )
    })?;
    Ok(format!("{local}@{domain}"))
}

/// A quoted-string for an ASCII name, otherwise UTF-8 base64 encoded-words.
fn display_name(name: &str) -> String {
    if name.is_ascii() {
        let mut quoted = String::with_capacity(name.len() + 2);
        quoted.push('"');
        for character in name.chars() {
            if matches!(character, '"' | '\\') {
                quoted.push('\\');
            }
            quoted.push(character);
        }
        quoted.push('"');
        return quoted;
    }
    let mut words = Vec::new();
    let mut start = 0;
    for (index, character) in name.char_indices() {
        if index + character.len_utf8() - start > MAX_ENCODED_WORD_BYTES {
            words.push(encoded_word(&name[start..index]));
            start = index;
        }
    }
    words.push(encoded_word(&name[start..]));
    words.join(" ")
}

fn encoded_word(text: &str) -> String {
    format!("=?UTF-8?B?{}?=", STANDARD.encode(text))
}

fn invalid() -> MailError {
    MailError::ValidationError("AWS SES requires one valid address".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses_become_seven_bit_ascii() {
        assert_eq!(
            recipient("maria@b\u{fc}cher.de").unwrap(),
            "maria@xn--bcher-kva.de"
        );
        assert_eq!(
            recipient("Maria <maria@example.com>").unwrap(),
            "maria@example.com"
        );
        assert_eq!(
            sender("Jos\u{e9} Silva <no-reply@acme.com.br>").unwrap(),
            "=?UTF-8?B?Sm9zw6kgU2lsdmE=?= <no-reply@acme.com.br>"
        );
        assert_eq!(
            sender("\"Acme, Inc.\" <billing@b\u{fc}cher.de>").unwrap(),
            "\"Acme, Inc.\" <billing@xn--bcher-kva.de>"
        );
        assert_eq!(
            sender("billing@b\u{fc}cher.de").unwrap(),
            "billing@xn--bcher-kva.de"
        );
        // An all-ASCII sender keeps its exact spelling.
        for unchanged in ["Acme Billing <billing@acme.com>", "billing@acme.com"] {
            assert_eq!(sender(unchanged).unwrap(), unchanged);
        }
    }

    #[test]
    fn long_names_split_into_bounded_encoded_words() {
        let name = "\u{e9}".repeat(100);
        let encoded = sender(&format!("{name} <a@example.com>")).unwrap();
        let (words, _) = encoded.rsplit_once(" <").unwrap();
        let words: Vec<&str> = words.split(' ').collect();
        assert!(words.len() > 1);
        let mut decoded = Vec::new();
        for word in words {
            assert!(word.len() <= 75, "{word}");
            let payload = word
                .strip_prefix("=?UTF-8?B?")
                .and_then(|rest| rest.strip_suffix("?="))
                .unwrap();
            decoded.extend(STANDARD.decode(payload).unwrap());
        }
        assert_eq!(String::from_utf8(decoded).unwrap(), name);
    }

    #[test]
    fn unicode_local_parts_fail_without_echoing_the_address() {
        for address in ["jos\u{e9}@example.com", "Jos\u{e9} <jos\u{e9}@example.com>"] {
            let error = sender(address).unwrap_err();
            assert!(matches!(&error, MailError::ValidationError(_)));
            assert!(!error.to_string().contains("example.com"));
        }
        let error = recipient("jos\u{e9}@example.com").unwrap_err();
        assert!(!error.to_string().contains("example.com"));
    }
}
