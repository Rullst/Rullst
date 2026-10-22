#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::MonitorError;
use super::test_support::{BASE64_TOKEN, BASE64URL_TOKEN, HEX_TOKEN, ROTATED_TOKEN};
use super::token::{Presented, TokenSet, presented, validate};
use axum::http::{HeaderMap, HeaderValue, header};

#[test]
fn random_base64_base64url_and_hex_tokens_of_32_bytes_are_accepted() {
    for token in [BASE64_TOKEN, BASE64URL_TOKEN, HEX_TOKEN, ROTATED_TOKEN] {
        assert_eq!(validate(token, 1), Ok(()), "{} characters", token.len());
    }
    // Padding is optional and limited to two characters.
    assert_eq!(validate(BASE64_TOKEN.trim_end_matches('='), 1), Ok(()));
}

#[test]
fn tokens_below_32_bytes_are_weak() {
    // 42 base64 characters carry 31.5 bytes.
    assert_eq!(
        validate(&BASE64URL_TOKEN[..42], 3),
        Err(MonitorError::WeakToken { position: 3 })
    );
    assert_eq!(validate(&BASE64URL_TOKEN[..43], 3), Ok(()));
    // A hexadecimal token needs 64 characters.
    assert_eq!(
        validate(&HEX_TOKEN[..63], 2),
        Err(MonitorError::WeakToken { position: 2 })
    );
    assert_eq!(
        validate(&HEX_TOKEN[..50], 2),
        Err(MonitorError::WeakToken { position: 2 })
    );
}

#[test]
fn characters_outside_the_token_alphabets_are_invalid() {
    let invalid = |token: &str| validate(token, 1);
    let expected = Err(MonitorError::InvalidToken { position: 1 });
    assert_eq!(invalid(&format!("{BASE64URL_TOKEN}!")), expected);
    assert_eq!(invalid(&format!("{BASE64URL_TOKEN} x")), expected);
    assert_eq!(invalid(&format!("{BASE64URL_TOKEN}===")), expected);
    assert_eq!(invalid(&format!("abc={BASE64URL_TOKEN}")), expected);
    assert_eq!(invalid(&"Ab1-".repeat(129)), expected);
    // 512 characters is the longest accepted token.
    assert_eq!(invalid(&BASE64URL_TOKEN.repeat(12)[..512]), Ok(()));
    assert_eq!(invalid(&BASE64URL_TOKEN.repeat(12)[..513]), expected);
    for byte in *b"+/-_" {
        let token = format!("{}{}", &BASE64URL_TOKEN[..43], char::from(byte));
        assert_eq!(invalid(&token), Ok(()), "{}", char::from(byte));
    }
}

#[test]
fn low_variety_and_placeholder_tokens_are_refused() {
    let expected = Err(MonitorError::PlaceholderToken { position: 1 });
    assert_eq!(validate(&"a".repeat(64), 1), expected);
    // Nine distinct characters are too few, ten are enough.
    assert_eq!(validate(&"abcdefghi".repeat(5), 1), expected);
    assert_eq!(validate(&"abcdefghij".repeat(5), 1), Ok(()));
    for placeholder in [
        "changeme",
        "CHANGE_ME",
        "change-me",
        "replace",
        "Example",
        "placeholder",
        "yourtoken",
        "your_token",
        "your-token",
    ] {
        let token = format!("{placeholder}{}", &BASE64URL_TOKEN[..43]);
        assert_eq!(validate(&token, 1), expected, "{placeholder}");
    }
}

#[test]
fn errors_name_the_position_and_never_the_token() {
    let tokens = vec![
        BASE64_TOKEN.to_string(),
        "short-but-secret-value".to_string(),
    ];
    let Err(error) = TokenSet::new(&tokens) else {
        panic!("a weak second token is refused");
    };
    assert_eq!(error, MonitorError::WeakToken { position: 2 });
    let message = error.to_string();
    assert!(message.contains("monitor token 2"));
    assert!(!message.contains("short-but-secret-value"));
}

#[test]
fn at_most_eight_tokens_are_accepted() {
    let eight = vec![BASE64_TOKEN.to_string(); 8];
    assert!(TokenSet::new(&eight).is_ok());
    let nine = vec![BASE64_TOKEN.to_string(); 9];
    assert_eq!(
        TokenSet::new(&nine).err(),
        Some(MonitorError::TooManyTokens { maximum: 8 })
    );
}

#[test]
fn a_token_matches_exactly_and_any_configured_token_matches() {
    let set = TokenSet::new(&[BASE64_TOKEN.to_string(), ROTATED_TOKEN.to_string()]).unwrap();
    assert!(!set.is_empty());
    assert!(set.matches(BASE64_TOKEN));
    assert!(set.matches(ROTATED_TOKEN));
    assert!(!set.matches(HEX_TOKEN));
    assert!(!set.matches(&BASE64_TOKEN[..43]));
    assert!(!set.matches(&format!("{BASE64_TOKEN}x")));
    assert!(!set.matches(""));

    let empty = TokenSet::new(&[]).unwrap();
    assert!(empty.is_empty());
    assert!(!empty.matches(BASE64_TOKEN));
}

fn headers(values: &[&str]) -> HeaderMap {
    let mut headers = HeaderMap::new();
    for value in values {
        headers.append(header::AUTHORIZATION, HeaderValue::from_str(value).unwrap());
    }
    headers
}

#[test]
fn the_presented_credential_is_classified_without_guessing() {
    assert_eq!(presented(&headers(&[])), Presented::Absent);
    assert_eq!(
        presented(&headers(&["Basic YWJjOmRlZg=="])),
        Presented::OtherScheme
    );
    assert_eq!(presented(&headers(&["Token abc"])), Presented::OtherScheme);
    assert_eq!(
        presented(&headers(&["Bearer abc"])),
        Presented::Bearer("abc")
    );
    assert_eq!(
        presented(&headers(&["bearer abc"])),
        Presented::Bearer("abc")
    );
    assert_eq!(
        presented(&headers(&["BEARER  abc "])),
        Presented::Bearer("abc")
    );
    assert_eq!(presented(&headers(&["Bearer"])), Presented::Invalid);
    assert_eq!(presented(&headers(&["Bearer "])), Presented::Invalid);
    assert_eq!(presented(&headers(&["Bearer a b"])), Presented::Invalid);
    let longest = format!("Bearer {}", "a".repeat(512));
    assert!(matches!(
        presented(&headers(&[&longest])),
        Presented::Bearer(_)
    ));
    let oversized = format!("Bearer {}", "a".repeat(513));
    assert_eq!(presented(&headers(&[&oversized])), Presented::Invalid);
    assert_eq!(
        presented(&headers(&["Bearer abc", "Bearer def"])),
        Presented::Invalid
    );
    let mut opaque = HeaderMap::new();
    opaque.insert(
        header::AUTHORIZATION,
        HeaderValue::from_bytes(b"Bearer \xff\xfe").unwrap(),
    );
    assert_eq!(presented(&opaque), Presented::Invalid);
}
