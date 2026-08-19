//! Pre-Flight email address deliverability & disposable email provider filter.

use std::collections::HashSet;
use std::sync::LazyLock;

/// Error returned when email deliverability validation fails.
#[derive(Debug, Clone, PartialEq, Eq)]
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
pub fn is_disposable_email(email: &str) -> bool {
    if let Some(domain) = extract_domain(email) {
        is_disposable_domain(domain)
    } else {
        false
    }
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
pub fn validate_email_deliverability(email: &str) -> Result<(), DeliverabilityError> {
    validate_email_syntax(email)?;

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
    fn test_is_disposable_domain() {
        assert!(is_disposable_domain("MAILINATOR.COM"));
        assert!(is_disposable_domain("sharklasers.com."));
        assert!(!is_disposable_domain("gmail.com"));
        assert!(!is_disposable_domain("outlook.com"));
    }
}
