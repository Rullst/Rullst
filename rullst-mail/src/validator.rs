//! Pre-Flight email address deliverability & disposable email provider filter.

use std::collections::HashSet;
use std::sync::LazyLock;

/// Error returned when email deliverability validation fails.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeliverabilityError {
    /// The email address format/syntax is invalid.
    InvalidSyntax(String),
    /// The email address uses a known disposable or temporary email domain.
    DisposableDomain(String),
    /// The domain name part is missing or malformed.
    MissingDomain,
}

impl std::fmt::Display for DeliverabilityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeliverabilityError::InvalidSyntax(msg) => write!(f, "Invalid email syntax: {}", msg),
            DeliverabilityError::DisposableDomain(dom) => {
                write!(f, "Email uses a blocked disposable provider: {}", dom)
            }
            DeliverabilityError::MissingDomain => write!(f, "Email address is missing domain part"),
        }
    }
}

impl std::error::Error for DeliverabilityError {}

/// Known disposable and temporary email domains.
static DISPOSABLE_DOMAINS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    let mut s = HashSet::new();
    let domains = [
        "10minutemail.com",
        "10minutemail.net",
        "20minutemail.com",
        "armyspy.com",
        "binkmail.com",
        "bobmail.info",
        "burnermail.io",
        "cachedot.net",
        "chacuo.net",
        "crazymailing.com",
        "cuvox.de",
        "dayrep.com",
        "deadaddress.com",
        "despam.it",
        "dispostable.com",
        "dodgit.com",
        "drdrb.net",
        "einrot.com",
        "emailondeck.com",
        "emailtemporal.org",
        "fakemailgenerator.com",
        "fleckens.hu",
        "fmail.com",
        "getairmail.com",
        "getnada.com",
        "gmailnator.com",
        "grr.la",
        "guerrillamail.biz",
        "guerrillamail.com",
        "guerrillamail.de",
        "guerrillamail.net",
        "guerrillamail.org",
        "guerrillamailblock.com",
        "gustr.com",
        "harakirimail.com",
        "hidemail.de",
        "inboxalias.com",
        "inboxkitten.com",
        "incognitomail.com",
        "instantemailaddress.com",
        "jourrapide.com",
        "junkmail.com",
        "kasmail.com",
        "klzlk.com",
        "letthemeatspam.com",
        "maildrop.cc",
        "mailcatch.com",
        "mailexpire.com",
        "mailforspam.com",
        "mailimate.com",
        "mailinator.com",
        "mailinator.net",
        "mailinator2.com",
        "mailnesia.com",
        "mailnull.com",
        "mailpoof.com",
        "mailsac.com",
        "mailtemp.net",
        "meltmail.com",
        "mintemail.com",
        "mohmal.com",
        "mytemp.email",
        "nada.ltd",
        "netmails.net",
        "noclickemail.com",
        "nomail.xl.cx",
        "nospam.ze.tc",
        "notsharingmy.info",
        "nowmymail.com",
        "objectmail.com",
        "oneoffmail.com",
        "owlymail.com",
        "pokemail.net",
        "proxymail.eu",
        "rcpt.at",
        "rhyta.com",
        "safetymail.info",
        "sharklasers.com",
        "shitmail.me",
        "smailpro.com",
        "soverin.net",
        "spam4.me",
        "spambog.com",
        "spambox.us",
        "spamcero.com",
        "spamfree24.org",
        "spamgourmet.com",
        "spamhole.com",
        "spamevader.com",
        "spaml.com",
        "superrito.com",
        "teleworm.us",
        "temp-mail.org",
        "temp-mail.ru",
        "tempail.com",
        "tempgmail.com",
        "tempi.im",
        "tempinbox.com",
        "tempmail.address",
        "tempmail.com",
        "tempmail.de",
        "tempmail.net",
        "tempmailaddress.com",
        "throwawaymail.com",
        "trash-mail.at",
        "trash-mail.com",
        "trashmail.com",
        "trashmail.de",
        "trashmail.net",
        "trashmailer.com",
        "trbvm.com",
        "twinmail.de",
        "upgradedmail.com",
        "vmani.com",
        "wegwerfmail.de",
        "wegwerfmail.net",
        "wegwerfmail.org",
        "whyspam.me",
        "yopmail.com",
        "yopmail.fr",
        "yopmail.net",
        "zippymail.info",
    ];
    for d in domains {
        s.insert(d);
    }
    s
});

/// Extracts the domain portion of an email address in lowercase.
pub fn extract_domain(email: &str) -> Option<&str> {
    let clean = email.trim();
    let at_idx = clean.rfind('@')?;
    let domain = clean[at_idx + 1..].trim();
    if domain.is_empty() {
        None
    } else {
        Some(domain)
    }
}

/// Checks whether a given domain name is a known disposable email service.
pub fn is_disposable_domain(domain: &str) -> bool {
    let dom_lower = domain.to_ascii_lowercase();
    let clean = dom_lower.trim_end_matches('.');
    DISPOSABLE_DOMAINS.contains(clean)
}

/// Checks whether a given email address belongs to a disposable email service.
///
/// Display-name and angle-bracket forms are checked by their bare address.
pub fn is_disposable_email(email: &str) -> bool {
    let address = recipient_address(email).unwrap_or(email);
    if let Some(domain) = extract_domain(address) {
        is_disposable_domain(domain)
    } else {
        false
    }
}

/// Largest bare recipient address accepted, matching the suppression key bound.
const MAX_RECIPIENT_ADDRESS_BYTES: usize = 320;

/// Extracts the bare `local@domain` address from one recipient.
///
/// This is the single recipient parser used by the delivery pipeline, the
/// deliverability check and suppression normalization. It accepts a bare
/// address, `<address>` or `Display Name <address>`, where the name may be a
/// quoted string. Lists, groups, comments, quoted local parts, domain literals
/// and control characters are rejected rather than guessed at.
pub(crate) fn recipient_address(value: &str) -> Result<&str, DeliverabilityError> {
    if value.chars().any(char::is_control) {
        return Err(recipient_form_error());
    }
    let value = value.trim();
    let address = match value.strip_suffix('>') {
        Some(head) => {
            let (name, address) = head.rsplit_once('<').ok_or_else(recipient_form_error)?;
            validate_display_name(name.trim_end())?;
            address
        }
        None => value,
    };
    if address.len() > MAX_RECIPIENT_ADDRESS_BYTES
        || address.chars().any(|character| {
            character.is_whitespace()
                || matches!(
                    character,
                    '<' | '>' | '(' | ')' | '[' | ']' | ':' | ';' | ',' | '"' | '\\'
                )
        })
    {
        return Err(recipient_form_error());
    }
    validate_email_syntax(address)?;
    Ok(address)
}

/// Splits one mailbox accepted by [`recipient_address`] into its bare address
/// and optional display name, with surrounding quotes and escapes removed.
pub(crate) fn mailbox_parts(value: &str) -> Result<(&str, Option<String>), DeliverabilityError> {
    let address = recipient_address(value)?;
    let name = value
        .trim()
        .strip_suffix('>')
        .and_then(|head| head.rsplit_once('<'))
        .map(|(name, _)| name.trim())
        .filter(|name| !name.is_empty())
        .map(|name| {
            let Some(quoted) = name
                .strip_prefix('"')
                .and_then(|rest| rest.strip_suffix('"'))
            else {
                return name.to_string();
            };
            let mut unquoted = String::with_capacity(quoted.len());
            let mut escaped = false;
            for character in quoted.chars() {
                if escaped || character != '\\' {
                    unquoted.push(character);
                    escaped = false;
                } else {
                    escaped = true;
                }
            }
            unquoted
        });
    Ok((address, name))
}

fn validate_display_name(name: &str) -> Result<(), DeliverabilityError> {
    if let Some(quoted) = name
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
    {
        let mut escaped = false;
        for character in quoted.chars() {
            match (escaped, character) {
                (true, _) => escaped = false,
                (false, '\\') => escaped = true,
                (false, '"') => return Err(recipient_form_error()),
                (false, _) => {}
            }
        }
        return if escaped {
            Err(recipient_form_error())
        } else {
            Ok(())
        };
    }
    if name.chars().any(|character| {
        matches!(
            character,
            '<' | '>' | '(' | ')' | '[' | ']' | ':' | ';' | '@' | '\\' | ',' | '"'
        )
    }) {
        return Err(recipient_form_error());
    }
    Ok(())
}

fn recipient_form_error() -> DeliverabilityError {
    DeliverabilityError::InvalidSyntax(
        "Recipient must be one address, optionally as `Name <address>`".into(),
    )
}

/// Validates email address syntax (RFC compliant basic checks).
pub fn validate_email_syntax(email: &str) -> Result<(), DeliverabilityError> {
    let clean = email.trim();
    if clean.is_empty() {
        return Err(DeliverabilityError::InvalidSyntax("Email is empty".into()));
    }

    let at_count = clean.chars().filter(|&c| c == '@').count();
    if at_count != 1 {
        return Err(DeliverabilityError::InvalidSyntax(
            "Email must contain exactly one '@' symbol".into(),
        ));
    }

    let parts: Vec<&str> = clean.split('@').collect();
    let (user, domain) = (parts[0], parts[1]);

    if user.is_empty() {
        return Err(DeliverabilityError::InvalidSyntax(
            "Local part before '@' is empty".into(),
        ));
    }

    if domain.is_empty() || !domain.contains('.') {
        return Err(DeliverabilityError::MissingDomain);
    }

    if domain.starts_with('.') || domain.ends_with('.') {
        return Err(DeliverabilityError::InvalidSyntax(
            "Domain cannot start or end with a dot".into(),
        ));
    }

    Ok(())
}

/// Comprehensive pre-flight email deliverability check: validates syntax and ensures domain is not disposable.
///
/// A display-name or angle-bracket recipient is parsed to its bare address
/// first, so the disposable-domain check never sees a trailing `>`.
pub fn validate_email_deliverability(email: &str) -> Result<(), DeliverabilityError> {
    let email = recipient_address(email)?;

    if let Some(domain) = extract_domain(email) {
        if is_disposable_domain(domain) {
            return Err(DeliverabilityError::DisposableDomain(domain.to_string()));
        }
    } else {
        return Err(DeliverabilityError::MissingDomain);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mailbox_parts_split_display_names() {
        assert_eq!(
            mailbox_parts("Acme Billing <billing@acme.com>"),
            Ok(("billing@acme.com", Some("Acme Billing".to_string())))
        );
        assert_eq!(
            mailbox_parts(r#""Doe, \"J\"" <j@example.com>"#),
            Ok(("j@example.com", Some(r#"Doe, "J""#.to_string())))
        );
        assert_eq!(mailbox_parts("j@example.com"), Ok(("j@example.com", None)));
        assert_eq!(
            mailbox_parts("<j@example.com>"),
            Ok(("j@example.com", None))
        );
        assert!(mailbox_parts("Acme <billing@acme.com").is_err());
    }

    #[test]
    fn test_valid_emails() {
        assert!(validate_email_deliverability("user@company.com").is_ok());
        assert!(validate_email_deliverability("john.doe+tag@sub.domain.org").is_ok());
        assert!(validate_email_deliverability("contact@rullst.dev").is_ok());
    }

    #[test]
    fn test_disposable_emails_blocked() {
        assert_eq!(
            validate_email_deliverability("test@mailinator.com"),
            Err(DeliverabilityError::DisposableDomain(
                "mailinator.com".to_string()
            ))
        );
        assert_eq!(
            validate_email_deliverability("bot@10minutemail.com"),
            Err(DeliverabilityError::DisposableDomain(
                "10minutemail.com".to_string()
            ))
        );
        assert_eq!(
            validate_email_deliverability("spammer@tempmail.com"),
            Err(DeliverabilityError::DisposableDomain(
                "tempmail.com".to_string()
            ))
        );
        assert_eq!(
            validate_email_deliverability("fake@guerrillamail.com"),
            Err(DeliverabilityError::DisposableDomain(
                "guerrillamail.com".to_string()
            ))
        );
    }

    #[test]
    fn test_syntax_errors() {
        assert!(validate_email_deliverability("").is_err());
        assert!(validate_email_deliverability("notanemail").is_err());
        assert!(validate_email_deliverability("user@").is_err());
        assert!(validate_email_deliverability("@domain.com").is_err());
        assert!(validate_email_deliverability("user@domain").is_err());
    }

    #[test]
    fn recipient_parser_extracts_one_bare_address_or_rejects() {
        for (input, expected) in [
            ("alice@example.com", "alice@example.com"),
            ("  alice@example.com ", "alice@example.com"),
            ("<alice@example.com>", "alice@example.com"),
            ("Alice <alice@example.com>", "alice@example.com"),
            ("Alice Q. O'Neil <alice@example.com>", "alice@example.com"),
            ("\"Doe, Alice\" <alice@example.com>", "alice@example.com"),
            (
                "\"bob@evil.example <x>\" <alice@example.com>",
                "alice@example.com",
            ),
            (
                "\"Say \\\"hi\\\"\" <alice@example.com>",
                "alice@example.com",
            ),
            ("José <jose@example.com>", "jose@example.com"),
        ] {
            assert_eq!(recipient_address(input), Ok(expected), "{input}");
        }
        for input in [
            "",
            "Alice <alice@example.com",
            "Alice alice@example.com>",
            "<alice@example.com> trailing",
            "alice@example.com, bob@example.com",
            "alice@example.com,bob",
            "a <b@example.com>, c <d@example.com>",
            "group: alice@example.com;",
            "alice@example.com (comment)",
            "bob@evil.example <alice@example.com>",
            "\"unterminated <alice@example.com>",
            "\"a\"b\" <alice@example.com>",
            "\"a\\\" <alice@example.com>",
            "\"quoted local\"@example.com",
            "user@[192.0.2.1]",
            "a b@example.com",
            "alice@example.com\t",
            "<<alice@example.com>>",
        ] {
            assert!(recipient_address(input).is_err(), "{input}");
        }
        let long = format!("{}@example.com", "a".repeat(MAX_RECIPIENT_ADDRESS_BYTES));
        assert!(recipient_address(&long).is_err());
    }

    #[test]
    fn display_name_recipients_cannot_bypass_the_disposable_filter() {
        for input in ["x <x@mailinator.com>", "<x@MAILINATOR.com>"] {
            assert!(matches!(
                validate_email_deliverability(input),
                Err(DeliverabilityError::DisposableDomain(_))
            ));
            assert!(is_disposable_email(input));
        }
        assert!(validate_email_deliverability("Alice <alice@example.com").is_err());
    }

    #[test]
    fn test_is_disposable_domain() {
        assert!(is_disposable_domain("MAILINATOR.COM"));
        assert!(is_disposable_domain("sharklasers.com."));
        assert!(!is_disposable_domain("gmail.com"));
        assert!(!is_disposable_domain("outlook.com"));
    }
}
