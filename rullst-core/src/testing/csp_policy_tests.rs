use super::*;

const ALL: [Inline; 4] = [
    Inline::ScriptElement,
    Inline::StyleElement,
    Inline::ScriptAttribute,
    Inline::StyleAttribute,
];

fn allows(policy: &str, inline: Inline, nonce: Option<&str>) -> bool {
    Policies::parse(policy).allows(inline, nonce)
}

#[test]
fn rullst_default_policy_needs_the_nonce_and_refuses_attributes() {
    let policy = crate::config::DEFAULT_CSP_TEMPLATE.replace("{NONCE}", "n0nce");
    for element in [Inline::ScriptElement, Inline::StyleElement] {
        assert!(allows(&policy, element, Some("n0nce")), "{element:?}");
        assert!(!allows(&policy, element, Some("other")), "{element:?}");
        assert!(!allows(&policy, element, None), "{element:?}");
        assert!(!allows(&policy, element, Some("")), "{element:?}");
    }
    for attribute in [Inline::ScriptAttribute, Inline::StyleAttribute] {
        assert!(!allows(&policy, attribute, Some("n0nce")), "{attribute:?}");
    }
}

#[test]
fn an_ungoverned_kind_is_allowed() {
    for inline in ALL {
        assert!(allows("img-src 'self'", inline, None), "{inline:?}");
        assert!(allows("", inline, None), "{inline:?}");
    }
    assert!(allows("style-src 'none'", Inline::ScriptElement, None));
    assert!(!allows("style-src 'none'", Inline::StyleElement, None));
}

#[test]
fn the_most_specific_directive_governs() {
    for (inline, specific, general) in [
        (Inline::ScriptElement, "script-src-elem", "script-src"),
        (Inline::StyleElement, "style-src-elem", "style-src"),
        (Inline::ScriptAttribute, "script-src-attr", "script-src"),
        (Inline::StyleAttribute, "style-src-attr", "style-src"),
    ] {
        let open = format!("default-src 'none'; {general} 'none'; {specific} 'unsafe-inline'");
        assert!(allows(&open, inline, None), "{inline:?} {open}");
        let closed =
            format!("default-src 'unsafe-inline'; {general} 'unsafe-inline'; {specific} 'none'");
        assert!(!allows(&closed, inline, None), "{inline:?} {closed}");
        let fallback = format!("default-src 'none'; {general} 'unsafe-inline'");
        assert!(allows(&fallback, inline, None), "{inline:?} {fallback}");
        assert!(
            allows("default-src 'unsafe-inline'", inline, None),
            "{inline:?}"
        );
        assert!(!allows("default-src 'self'", inline, None), "{inline:?}");
    }
}

#[test]
fn directive_names_ignore_case_and_the_first_repeat_wins() {
    assert!(allows(
        "SCRIPT-SRC 'unsafe-inline'",
        Inline::ScriptElement,
        None
    ));
    assert!(!allows(
        "script-src 'none'; script-src 'unsafe-inline'",
        Inline::ScriptElement,
        None
    ));
    assert!(allows(
        "script-src 'unsafe-inline'; script-src 'none'",
        Inline::ScriptElement,
        None
    ));
    assert!(allows(
        ";; script-src  'unsafe-inline' ;",
        Inline::ScriptElement,
        None
    ));
}

#[test]
fn every_policy_of_the_header_must_allow_it() {
    let both = "script-src 'unsafe-inline', script-src 'nonce-abc'";
    assert!(!allows(both, Inline::ScriptElement, None));
    assert!(!allows(both, Inline::ScriptElement, Some("other")));
    assert!(allows(
        "script-src 'nonce-abc', script-src 'self' 'nonce-abc'",
        Inline::ScriptElement,
        Some("abc")
    ));
    assert!(allows(
        "img-src 'self', script-src 'unsafe-inline'",
        Inline::ScriptElement,
        None
    ));
}

#[test]
fn unsafe_inline_counts_only_without_nonces_hashes_or_strict_dynamic() {
    for inline in ALL {
        assert!(
            allows("default-src 'UNSAFE-INLINE'", inline, None),
            "{inline:?}"
        );
        assert!(
            !allows("default-src 'unsafe-inline' 'nonce-x'", inline, None),
            "{inline:?}"
        );
        assert_eq!(
            allows("default-src 'unsafe-inline' 'strict-dynamic'", inline, None),
            !matches!(inline, Inline::ScriptElement | Inline::ScriptAttribute),
            "{inline:?}"
        );
    }
    for hash in ["'sha256-AAAA'", "'SHA384-AAAA'", "'sha512-AAAA'"] {
        let policy = format!("default-src 'unsafe-inline' {hash}");
        assert!(!allows(&policy, Inline::StyleAttribute, None), "{hash}");
        // A hash may match an element's content, which is not computed.
        assert!(allows(&policy, Inline::ScriptElement, None), "{hash}");
    }
}

#[test]
fn hashes_allow_attributes_only_with_unsafe_hashes() {
    for attribute in [Inline::ScriptAttribute, Inline::StyleAttribute] {
        assert!(allows(
            "default-src 'unsafe-hashes' 'sha256-AAAA'",
            attribute,
            None
        ));
        assert!(!allows("default-src 'sha256-AAAA'", attribute, None));
        assert!(!allows("default-src 'unsafe-hashes'", attribute, None));
    }
    for element in [Inline::ScriptElement, Inline::StyleElement] {
        assert!(allows("default-src 'sha256-AAAA'", element, None));
        assert!(!allows("default-src 'unsafe-hashes'", element, None));
    }
}

#[test]
fn a_nonce_must_match_exactly_and_never_opens_attributes() {
    let policy = "script-src 'nonce-Abc+/='";
    assert!(allows(policy, Inline::ScriptElement, Some("Abc+/=")));
    assert!(!allows(policy, Inline::ScriptElement, Some("abc+/=")));
    assert!(!allows(policy, Inline::ScriptElement, Some("Abc")));
    assert!(!allows(policy, Inline::ScriptAttribute, Some("Abc+/=")));
    // A short or non-ASCII source is not mistaken for a nonce or hash.
    assert!(allows(
        "script-src 'unsafe-inline' 'n' éééé",
        Inline::ScriptElement,
        None
    ));
}
