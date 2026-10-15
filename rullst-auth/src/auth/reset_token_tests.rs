use super::*;

const ZEROS: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const COUNTING: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn lower_hex(value: &str) -> bool {
    value
        .bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[test]
fn generated_tokens_are_64_lowercase_hex_characters_and_differ() {
    let first = PasswordResetToken::generate().unwrap();
    let second = PasswordResetToken::generate().unwrap();
    for token in [&first, &second] {
        assert_eq!(token.expose().len(), 64);
        assert!(lower_hex(token.expose()));
    }
    assert_ne!(first.expose(), second.expose());
    // 32 random bytes: a run of zero bytes would show as "00" pairs only.
    assert_ne!(first.expose(), ZEROS);
}

#[test]
fn the_stored_digest_is_what_the_submitted_token_looks_up() {
    let token = PasswordResetToken::generate().unwrap();
    let stored = token.digest();
    assert_eq!(stored.len(), 64);
    assert!(lower_hex(&stored));
    assert_ne!(stored, token.expose());
    assert_eq!(
        password_reset_token_digest(token.expose()),
        Some(stored.clone())
    );
    assert!(password_reset_digests_match(
        &stored,
        &password_reset_token_digest(token.expose()).unwrap()
    ));
}

#[test]
fn digests_are_domain_separated_sha256() {
    // SHA-256("rullst.auth.password-reset.v1\0" || token), computed
    // independently; the plain SHA-256 of ZEROS is 60e05bd1….
    assert_eq!(
        password_reset_token_digest(ZEROS).unwrap(),
        "6f6fc021372d5667cd86f2b4856b25c5c4a463cc334908b21c3d9fd474635ed8"
    );
    assert_eq!(
        password_reset_token_digest(COUNTING).unwrap(),
        "50b0580885275eb54f3a29fa9440752f93afa1f146fcd4fc750fb54ae23a6b47"
    );
}

#[test]
fn malformed_submitted_tokens_have_no_digest() {
    let uppercase = COUNTING.to_ascii_uppercase();
    let short = &COUNTING[..63];
    let long = format!("{COUNTING}0");
    let not_hex = format!("g{}", &COUNTING[1..]);
    let with_space = format!(" {}", &COUNTING[1..]);
    let multibyte = format!("é{}", &COUNTING[2..]);
    for token in [
        "",
        short,
        long.as_str(),
        uppercase.as_str(),
        not_hex.as_str(),
        with_space.as_str(),
        multibyte.as_str(),
    ] {
        assert_eq!(password_reset_token_digest(token), None, "{}", token.len());
    }
    // Every lowercase hex digit is accepted.
    assert!(password_reset_token_digest(&"f".repeat(64)).is_some());
    assert!(password_reset_token_digest(&"9".repeat(64)).is_some());
    assert!(password_reset_token_digest(&"a".repeat(64)).is_some());
}

#[test]
fn digest_comparison_needs_a_full_stored_digest() {
    let stored = password_reset_token_digest(ZEROS).unwrap();
    let other = password_reset_token_digest(COUNTING).unwrap();
    assert!(password_reset_digests_match(&stored, &stored.clone()));
    assert!(!password_reset_digests_match(&stored, &other));
    let mut last_differs = stored.clone();
    last_differs.pop();
    last_differs.push('0');
    assert!(!password_reset_digests_match(&stored, &last_differs));
    assert!(!password_reset_digests_match(&stored, &stored[..63]));
    // Equal values that are not digests never match.
    assert!(!password_reset_digests_match("", ""));
    assert!(!password_reset_digests_match("abc", "abc"));
    assert!(!password_reset_digests_match(&stored[..63], &stored[..63]));
}

#[test]
fn debug_output_never_shows_the_token() {
    let token = PasswordResetToken::generate().unwrap();
    let debug = format!("{token:?}");
    assert_eq!(debug, "PasswordResetToken([REDACTED])");
    assert!(!debug.contains(token.expose()));
}
