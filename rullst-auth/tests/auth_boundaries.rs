use argon2::{Params, password_hash::phc::PasswordHash};
use axum::http::{HeaderMap, HeaderValue, header::COOKIE};
use rullst_auth::{AuthError, extract_session_cookie, needs_rehash, validate_app_key};

#[test]
fn rehash_is_required_for_each_independent_parameter_change() {
    // These PHC fixtures exercise policy inspection, not password verification.
    let phc = |algorithm, version, memory, time, parallelism| {
        format!(
            "${algorithm}$v={version}$m={memory},t={time},p={parallelism}$c29tZXNhbHQ$YhhQvA1/zHGEoWnUBY/J2iY/R/hG93WqG2k73D655b0"
        )
    };
    let memory = Params::DEFAULT_M_COST;
    let time = Params::DEFAULT_T_COST;
    let parallelism = Params::DEFAULT_P_COST;
    let current = phc("argon2id", 19, memory, time, parallelism);
    PasswordHash::new(&current).expect("valid current-policy PHC");
    assert!(!needs_rehash(&current));

    for (field, changed) in [
        ("algorithm", phc("argon2i", 19, memory, time, parallelism)),
        ("version", phc("argon2id", 16, memory, time, parallelism)),
        ("memory", phc("argon2id", 19, memory + 1, time, parallelism)),
        ("time", phc("argon2id", 19, memory, time + 1, parallelism)),
        (
            "parallelism",
            phc("argon2id", 19, memory, time, parallelism + 1),
        ),
    ] {
        PasswordHash::new(&changed).expect("a parameter change must remain valid PHC");
        assert!(
            needs_rehash(&changed),
            "a change to {field} requires rehash"
        );
    }
}

#[test]
fn application_key_entropy_accepts_the_inclusive_128_bit_boundary() {
    // Public test data, never an application secret: sixteen equally frequent
    // byte values, twice each, give the documented estimate of exactly 128 bits.
    let boundary = b"0123456789abcdef0123456789abcdef";
    validate_app_key(boundary).expect("the lower entropy bound is inclusive");

    let mut below = *boundary;
    below[0] = below[1];
    assert!(matches!(
        validate_app_key(&below),
        Err(AuthError::MissingAppKey(_))
    ));
    let mut above = *boundary;
    above[0] = b'X';
    validate_app_key(&above).expect("more estimated entropy remains accepted");
}

fn headers(value: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.append(COOKIE, HeaderValue::from_static("theme=dark"));
    headers.append(
        COOKIE,
        HeaderValue::from_str(&format!("rullst_session={value}; other=1"))
            .expect("valid HTTP header fixture"),
    );
    headers
}

#[test]
fn session_cookie_size_limit_applies_to_the_value_and_is_inclusive() {
    for length in [4095, 4096] {
        let value = "a".repeat(length);
        assert_eq!(extract_session_cookie(&headers(&value)), Some(value));
    }
    for length in [4097, 8192] {
        assert_eq!(extract_session_cookie(&headers(&"a".repeat(length))), None);
    }
}

#[test]
fn short_session_cookies_reject_internal_non_graphic_bytes() {
    for value in ["left right", "left\tright"] {
        assert_eq!(extract_session_cookie(&headers(value)), None);
    }
    assert_eq!(
        extract_session_cookie(&headers("left-right")),
        Some("left-right".into())
    );
}
