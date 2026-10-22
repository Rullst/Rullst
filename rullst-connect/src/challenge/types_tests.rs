use super::*;

const STATE: &str = "s1234567890123456789012345678901";
const VERIFIER: &str = "v123456789012345678901234567890123456789012";

fn challenge(created_at: u64, expires_at: u64) -> Result<LoginChallenge, ChallengeError> {
    LoginChallenge::new(
        "google", STATE, None, VERIFIER, None, created_at, expires_at,
    )
}

#[test]
fn generated_ids_are_43_url_safe_characters_and_distinct() {
    let first = ChallengeId::generate();
    let second = ChallengeId::generate();
    assert_eq!(first.as_str().len(), 43);
    assert!(
        first
            .as_str()
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    );
    assert_ne!(first, second);
    assert_eq!(ChallengeId::parse(first.as_str()).unwrap(), first);
}

#[test]
fn parse_accepts_only_the_generated_shape() {
    let canonical = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQ";
    assert_eq!(ChallengeId::parse(canonical).unwrap().as_str(), canonical);
    for altered in [
        "",
        "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOP",
        "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQR",
        "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOP=",
        "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOP+",
        "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOP!",
    ] {
        assert_eq!(
            ChallengeId::parse(altered),
            Err(ChallengeError::Invalid("challenge id")),
            "{altered:?}"
        );
    }
    // 44 characters decode to 33 bytes: the length check refuses it first.
    assert!(ChallengeId::parse("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRS").is_err());
}

#[test]
fn storage_key_is_a_domain_separated_digest() {
    let id = ChallengeId::parse("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQ").unwrap();
    assert_eq!(
        id.storage_key(),
        "00bb2b69e4d9b9a9be0d7d8fd921a5c24af892578c4ab97a11a6778176c58b4f"
    );
    let zeros = ChallengeId::parse("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA").unwrap();
    assert_eq!(
        zeros.storage_key(),
        "1c9aa81d5d40f9b2830f33af43357072ecc8d2153725c37f43114ce59c9c46e3"
    );
}

#[test]
fn ids_compare_by_value_and_never_print() {
    let id = ChallengeId::parse("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQ").unwrap();
    let same = ChallengeId::parse("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQ").unwrap();
    let other = ChallengeId::parse("bbcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQ").unwrap();
    assert!(id == same);
    assert!(id != other);
    assert_eq!(format!("{id:?}"), "ChallengeId(\"[REDACTED]\")");
}

#[test]
fn return_paths_stay_on_this_origin() {
    for accepted in ["/", "/dashboard", "/courses/7?tab=lessons#top", "/a%20b"] {
        assert_eq!(ReturnPath::parse(accepted).unwrap().as_str(), accepted);
    }
    assert!(ReturnPath::parse(format!("/{}", "a".repeat(1023))).is_ok());
    for refused in [
        String::new(),
        "dashboard".to_string(),
        "//evil.example".to_string(),
        "/\\evil.example".to_string(),
        "/a\\b".to_string(),
        "https://evil.example/".to_string(),
        "/a b".to_string(),
        "/a\tb".to_string(),
        "/a\r\nLocation: x".to_string(),
        "/caf\u{e9}".to_string(),
        format!("/{}", "a".repeat(1024)),
    ] {
        assert_eq!(
            ReturnPath::parse(refused.clone()),
            Err(ChallengeError::Invalid("return path")),
            "{refused:?}"
        );
    }
}

#[test]
fn challenges_validate_every_field() {
    let valid = LoginChallenge::new(
        "google",
        STATE,
        Some(STATE.to_string()),
        VERIFIER,
        Some(ReturnPath::parse("/courses").unwrap()),
        100,
        700,
    )
    .unwrap();
    assert_eq!(valid.provider(), "google");
    assert_eq!(valid.state(), STATE);
    assert_eq!(valid.nonce(), Some(STATE));
    assert_eq!(valid.code_verifier(), VERIFIER);
    assert_eq!(valid.return_to().unwrap().as_str(), "/courses");
    assert_eq!(valid.created_at(), 100);
    assert_eq!(valid.expires_at(), 700);

    let invalid = |provider: &str, state: &str, nonce: Option<&str>, verifier: &str| {
        LoginChallenge::new(
            provider,
            state,
            nonce.map(str::to_string),
            verifier,
            None,
            100,
            200,
        )
        .unwrap_err()
    };
    for provider in ["", "Google", "goo gle", "google.com", &"a".repeat(33)] {
        assert_eq!(
            invalid(provider, STATE, None, VERIFIER),
            ChallengeError::Invalid("provider name"),
            "{provider:?}"
        );
    }
    assert!(LoginChallenge::new("a", STATE, None, VERIFIER, None, 1, 2).is_ok());
    assert!(LoginChallenge::new("my_idp-2", STATE, None, VERIFIER, None, 1, 2).is_ok());
    assert!(LoginChallenge::new("a".repeat(32), STATE, None, VERIFIER, None, 1, 2).is_ok());

    for state in [
        &STATE[1..],
        "s123456789012345678901234567890!",
        &"s".repeat(129),
    ] {
        assert_eq!(
            invalid("google", state, None, VERIFIER),
            ChallengeError::Invalid("state")
        );
        assert_eq!(
            invalid("google", STATE, Some(state), VERIFIER),
            ChallengeError::Invalid("nonce")
        );
    }
    let unreserved = "a-._~5678901234567890123456789012";
    assert!(LoginChallenge::new("x", unreserved, None, VERIFIER, None, 1, 2).is_ok());
    assert!(LoginChallenge::new("x", "s".repeat(128), None, VERIFIER, None, 1, 2).is_ok());
    for verifier in [
        &VERIFIER[1..],
        &"v".repeat(129),
        "v12345678901234567890123456789012345678901/",
    ] {
        assert_eq!(
            invalid("google", STATE, None, verifier),
            ChallengeError::Invalid("code verifier")
        );
    }
    assert!(LoginChallenge::new("x", STATE, None, "v".repeat(128), None, 1, 2).is_ok());
}

#[test]
fn lifetimes_are_between_one_second_and_ten_minutes() {
    assert!(challenge(100, 101).is_ok());
    assert!(challenge(100, 700).is_ok());
    for (created, expires) in [(100, 100), (100, 99), (100, 701), (0, u64::MAX)] {
        assert_eq!(
            challenge(created, expires),
            Err(ChallengeError::Invalid("lifetime")),
            "{created}..{expires}"
        );
    }
    let largest = i64::MAX as u64;
    assert!(challenge(largest - 600, largest).is_ok());
    assert_eq!(
        challenge(largest - 599, largest + 1),
        Err(ChallengeError::Invalid("lifetime"))
    );
}

#[test]
fn expiry_starts_at_the_expiry_second() {
    let challenge = challenge(100, 160).unwrap();
    assert!(!challenge.is_expired_at(159));
    assert!(challenge.is_expired_at(160));
    assert!(challenge.is_expired_at(161));
}

#[test]
fn debug_output_redacts_every_secret() {
    let challenge = LoginChallenge::new(
        "google",
        STATE,
        Some(STATE.to_string()),
        VERIFIER,
        None,
        1,
        2,
    )
    .unwrap();
    let debug = format!("{challenge:?}");
    assert_eq!(
        debug,
        "LoginChallenge { provider: \"google\", state: \"[REDACTED]\", nonce: Some(\"[REDACTED]\"), code_verifier: \"[REDACTED]\", return_to: None, created_at: 1, expires_at: 2 }"
    );
}
