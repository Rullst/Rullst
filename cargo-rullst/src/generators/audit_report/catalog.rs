//! The checks of `audit --report` and their OWASP ASVS 5.0.0 mapping.
//!
//! Requirement IDs, section names and levels are copied from the official
//! OWASP ASVS 5.0.0 release CSV
//! (`OWASP_Application_Security_Verification_Standard_5.0.0_en.csv`, release
//! tag `v5.0.0_release`). ASVS requirements have no titles; the section name
//! is reported instead. A requirement that cannot be verified there is named
//! by its chapter only.

use super::model::{Group, Report};

pub(crate) const ASVS_NAME: &str = "OWASP Application Security Verification Standard";
pub(crate) const ASVS_VERSION: &str = "5.0.0";
pub(crate) const ASVS_SOURCE: &str = "https://github.com/OWASP/ASVS/releases/tag/v5.0.0_release";
pub(crate) const WCAG_VERSION: &str = "WCAG 2.2";
pub(crate) const GUIDE: &str = "https://rullst.github.io/Rullst/book/security-report.html";

pub(crate) const NOTICE: &str = "This report records bounded static checks for a reviewer. It does not replace a manual security review or a penetration test, and it is not a certification of any standard.";
pub(crate) const MAPPING_NOTE: &str = "A requirement mapped to a check is only partially evidenced by that static check; the mapping names what the check relates to and never states that the requirement is met.";

/// One ASVS 5.0.0 requirement a check relates to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AsvsRef {
    pub id: &'static str,
    pub section: &'static str,
    pub level: u8,
}

/// The static description of one check.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Spec {
    pub id: &'static str,
    pub title: &'static str,
    pub group: Group,
    /// One-line fix hint for findings.
    pub fix: &'static str,
    /// Anchor in the security-report guide.
    pub anchor: &'static str,
    pub asvs: &'static [AsvsRef],
    /// ASVS chapter for checks without a Level 1 requirement.
    pub chapter: Option<&'static str>,
    pub wcag: &'static [&'static str],
}

impl Spec {
    pub(crate) fn docs(&self) -> String {
        format!("{GUIDE}#{}", self.anchor)
    }
}

const fn asvs(id: &'static str, section: &'static str, level: u8) -> AsvsRef {
    AsvsRef { id, section, level }
}

const HEADERS_SECTION: &str = "V3.4 Browser Security Mechanism Headers";
const COOKIE_SECTION: &str = "V3.3 Cookie Setup";

pub(crate) const HEADERS: Spec = Spec {
    id: "security_headers",
    title: "Security headers and CSP",
    group: Group::Security,
    fix: "Serve the router through `Server` (or mount `headers_middleware`) and keep `[security] csp` free of 'unsafe-inline', 'unsafe-eval' and wildcard script sources.",
    anchor: "security-headers-and-csp",
    asvs: &[
        asvs("V3.4.1", HEADERS_SECTION, 1),
        asvs("V3.4.3", HEADERS_SECTION, 2),
        asvs("V3.4.4", HEADERS_SECTION, 2),
        asvs("V3.4.6", HEADERS_SECTION, 2),
    ],
    chapter: None,
    wcag: &[],
};

pub(crate) const CSRF: Spec = Spec {
    id: "csrf",
    title: "CSRF",
    group: Group::Security,
    fix: "Serve the router through `Server` or mount `rullst::security::csrf_middleware`; register API and webhook writes as machine endpoints.",
    anchor: "csrf",
    asvs: &[asvs("V3.5.1", "V3.5 Browser Origin Separation", 1)],
    chapter: None,
    wcag: &[],
};

pub(crate) const COOKIES: Spec = Spec {
    id: "cookies",
    title: "Session and auth cookies",
    group: Group::Security,
    fix: "Set Secure (outside development), HttpOnly on session/auth cookies and SameSite=Lax or Strict; keep `[security] csrf_same_site` Lax or Strict.",
    anchor: "session-and-auth-cookies",
    asvs: &[
        asvs("V3.3.1", COOKIE_SECTION, 1),
        asvs("V3.3.2", COOKIE_SECTION, 2),
        asvs("V3.3.4", COOKIE_SECTION, 2),
    ],
    chapter: None,
    wcag: &[],
};

pub(crate) const RATE_LIMIT: Spec = Spec {
    id: "auth_rate_limit",
    title: "Rate limiting on authentication routes",
    group: Group::Security,
    fix: "Put a rate limiter (`RateLimiter`, `rate_limit_middleware`, `Server::rate_limit`) or `LoginGuard` in front of credential routes.",
    anchor: "rate-limiting",
    asvs: &[asvs("V6.3.1", "V6.3 General Authentication Security", 1)],
    chapter: None,
    wcag: &[],
};

pub(crate) const SECRETS: Spec = Spec {
    id: "committed_secrets",
    title: "Committed secrets",
    group: Group::Security,
    fix: "Remove the value from Git history, rotate it at the provider, and load it from the environment or a secret manager.",
    anchor: "committed-secrets",
    asvs: &[asvs("V13.3.1", "V13.3 Secret Management", 2)],
    chapter: Some("V13 Configuration"),
    wcag: &[],
};

pub(crate) const DEPENDENCIES: Spec = Spec {
    id: "dependency_audit",
    title: "Vulnerable dependencies",
    group: Group::Security,
    fix: "Run `cargo audit`, update the affected crates, and record any remaining exception with an owner and expiry.",
    anchor: "vulnerable-dependencies",
    asvs: &[asvs(
        "V15.2.1",
        "V15.2 Security Architecture and Dependencies",
        1,
    )],
    chapter: None,
    wcag: &[],
};

pub(crate) const IDOR: Spec = Spec {
    id: "idor",
    title: "IDOR / BOLA route classification",
    group: Group::Security,
    fix: "Add `// rullst-access: public|owner|role|admin — reason` above each parameterized route and the matching guard.",
    anchor: "idor",
    asvs: &[
        asvs("V8.2.1", "V8.2 General Authorization Design", 1),
        asvs("V8.2.2", "V8.2 General Authorization Design", 1),
    ],
    chapter: None,
    wcag: &[],
};

pub(crate) const PERSONAL_DATA: Spec = Spec {
    id: "personal_data",
    title: "Personal-data inventory",
    group: Group::Security,
    fix: "Mark personal fields with `#[privacy]` in a `#[derive(PersonalData)]` model and store sensitive text with `#[orm(encrypted)]`.",
    anchor: "personal-data-inventory",
    asvs: &[asvs("V14.1.1", "V14.1 Data Protection Documentation", 2)],
    chapter: Some("V14 Data Protection"),
    wcag: &[],
};

pub(crate) const IMG_ALT: Spec = Spec {
    id: "a11y_img_alt",
    title: "Images have a text alternative",
    group: Group::Accessibility,
    fix: "Give every `<img>` an `alt` attribute (`alt=\"\"` for decorative images).",
    anchor: "accessibility",
    asvs: &[],
    chapter: None,
    wcag: &["1.1.1 Non-text Content (A)"],
};

pub(crate) const FORM_LABELS: Spec = Spec {
    id: "a11y_form_labels",
    title: "Form controls have a label",
    group: Group::Accessibility,
    fix: "Associate each control with `<label for=\"id\">`, wrap it in `<label>`, or add `aria-label`/`aria-labelledby`.",
    anchor: "accessibility",
    asvs: &[],
    chapter: None,
    wcag: &[
        "1.3.1 Info and Relationships (A)",
        "3.3.2 Labels or Instructions (A)",
        "4.1.2 Name, Role, Value (A)",
    ],
};

pub(crate) const HTML_LANG: Spec = Spec {
    id: "a11y_html_lang",
    title: "Pages declare their language",
    group: Group::Accessibility,
    fix: "Add a `lang` attribute to `<html>` (for example `<html lang=\"en\">`).",
    anchor: "accessibility",
    asvs: &[],
    chapter: None,
    wcag: &["3.1.1 Language of Page (A)"],
};

/// Every ASVS 5.0.0 Level 1 requirement, by chapter (70 in total). V16
/// Security Logging and Error Handling and V17 WebRTC have none.
#[rustfmt::skip]
pub(crate) const LEVEL1: &[(&str, &str, &[&str])] = &[
    ("V1", "Encoding and Sanitization", &["V1.2.1", "V1.2.2", "V1.2.3", "V1.2.4", "V1.2.5", "V1.3.1", "V1.3.2", "V1.5.1"]),
    ("V2", "Validation and Business Logic", &["V2.1.1", "V2.2.1", "V2.2.2", "V2.3.1"]),
    ("V3", "Web Frontend Security", &["V3.2.1", "V3.2.2", "V3.3.1", "V3.4.1", "V3.4.2", "V3.5.1", "V3.5.2", "V3.5.3"]),
    ("V4", "API and Web Service", &["V4.1.1", "V4.4.1"]),
    ("V5", "File Handling", &["V5.2.1", "V5.2.2", "V5.3.1", "V5.3.2"]),
    ("V6", "Authentication", &["V6.1.1", "V6.2.1", "V6.2.2", "V6.2.3", "V6.2.4", "V6.2.5", "V6.2.6", "V6.2.7", "V6.2.8", "V6.3.1", "V6.3.2", "V6.4.1", "V6.4.2"]),
    ("V7", "Session Management", &["V7.2.1", "V7.2.2", "V7.2.3", "V7.2.4", "V7.4.1", "V7.4.2"]),
    ("V8", "Authorization", &["V8.1.1", "V8.2.1", "V8.2.2", "V8.3.1"]),
    ("V9", "Self-contained Tokens", &["V9.1.1", "V9.1.2", "V9.1.3", "V9.2.1"]),
    ("V10", "OAuth and OIDC", &["V10.4.1", "V10.4.2", "V10.4.3", "V10.4.4", "V10.4.5"]),
    ("V11", "Cryptography", &["V11.3.1", "V11.3.2", "V11.4.1"]),
    ("V12", "Secure Communication", &["V12.1.1", "V12.2.1", "V12.2.2"]),
    ("V13", "Configuration", &["V13.4.1"]),
    ("V14", "Data Protection", &["V14.2.1", "V14.3.1"]),
    ("V15", "Secure Coding and Architecture", &["V15.1.1", "V15.2.1", "V15.3.1"]),
];

/// Level 1 requirements of one chapter that no check relates to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NotEvaluated {
    pub chapter: &'static str,
    pub name: &'static str,
    pub requirements: Vec<&'static str>,
    /// Whether no Level 1 requirement of the chapter is mapped at all.
    pub whole_chapter: bool,
}

/// The Level 1 requirements outside every check of `report`, by chapter.
pub(crate) fn not_evaluated(report: &Report) -> Vec<NotEvaluated> {
    let mapped = |id: &str| {
        report
            .checks
            .iter()
            .flat_map(|check| check.asvs())
            .chain(PERSONAL_DATA.asvs)
            .any(|reference| reference.level == 1 && reference.id == id)
    };
    LEVEL1
        .iter()
        .filter_map(|(chapter, name, ids)| {
            let requirements: Vec<_> = ids.iter().copied().filter(|id| !mapped(id)).collect();
            let whole_chapter = requirements.len() == ids.len();
            (!requirements.is_empty()).then_some(NotEvaluated {
                chapter,
                name,
                whole_chapter,
                requirements,
            })
        })
        .collect()
}

/// (mapped, total) Level 1 requirement counts.
pub(crate) fn level1_coverage(report: &Report) -> (usize, usize) {
    let total = LEVEL1.iter().map(|(_, _, ids)| ids.len()).sum::<usize>();
    let open = not_evaluated(report)
        .iter()
        .map(|chapter| chapter.requirements.len())
        .sum::<usize>();
    (total - open, total)
}

/// `V3.4.1 (L1, V3.4 Browser Security Mechanism Headers)` lines for a check.
pub(crate) fn describe(reference: &AsvsRef) -> String {
    format!(
        "{} (L{}, {})",
        reference.id, reference.level, reference.section
    )
}
