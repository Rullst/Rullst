use super::*;

const ID: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQ";

#[test]
fn set_cookie_is_short_lived_http_only_and_lax() {
    let id = ChallengeId::parse(ID).unwrap();
    let cookie = ChallengeCookie::new("/auth/google", false).unwrap();
    assert_eq!(
        cookie.set(&id, 600),
        "rullst_oauth_challenge=abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQ; Path=/auth/google; Max-Age=600; HttpOnly; SameSite=Lax"
    );
    let secure = ChallengeCookie::new("/auth/google", true).unwrap();
    assert_eq!(
        secure.set(&id, 90),
        "rullst_oauth_challenge=abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQ; Path=/auth/google; Max-Age=90; HttpOnly; SameSite=Lax; Secure"
    );
}

#[test]
fn max_age_never_exceeds_ten_minutes_or_drops_below_one_second() {
    let id = ChallengeId::parse(ID).unwrap();
    let cookie = ChallengeCookie::new("/auth", false).unwrap();
    assert!(cookie.set(&id, 601).contains("; Max-Age=600;"));
    assert!(cookie.set(&id, u64::MAX).contains("; Max-Age=600;"));
    assert!(cookie.set(&id, 0).contains("; Max-Age=1;"));
}

#[test]
fn clearing_matches_the_cookie_scope() {
    assert_eq!(
        ChallengeCookie::new("/auth/google", false).unwrap().clear(),
        "rullst_oauth_challenge=; Path=/auth/google; Max-Age=0; Expires=Thu, 01 Jan 1970 00:00:00 GMT; HttpOnly; SameSite=Lax"
    );
    assert_eq!(
        ChallengeCookie::new("/auth/google", true).unwrap().clear(),
        "rullst_oauth_challenge=; Path=/auth/google; Max-Age=0; Expires=Thu, 01 Jan 1970 00:00:00 GMT; HttpOnly; SameSite=Lax; Secure"
    );
}

#[test]
fn paths_are_plain_absolute_prefixes() {
    for accepted in ["/", "/auth", "/auth/google", "/a-b.c_d~e/f"] {
        assert!(ChallengeCookie::new(accepted, false).is_ok(), "{accepted}");
    }
    assert!(ChallengeCookie::new(format!("/{}", "a".repeat(255)), false).is_ok());
    for refused in [
        String::new(),
        "auth".to_string(),
        "/auth; Domain=evil.example".to_string(),
        "/auth google".to_string(),
        "/auth,x".to_string(),
        "/auth?x=1".to_string(),
        format!("/{}", "a".repeat(256)),
    ] {
        assert_eq!(
            ChallengeCookie::new(refused.clone(), false),
            Err(ChallengeError::Invalid("cookie path")),
            "{refused:?}"
        );
    }
}

#[test]
fn reading_finds_only_the_challenge_cookie() {
    assert_eq!(
        ChallengeCookie::read(["rullst_csrf=a; rullst_oauth_challenge=abc; theme=dark"]),
        Some("abc".to_string())
    );
    assert_eq!(
        ChallengeCookie::read(["rullst_csrf=a", "x=1;rullst_oauth_challenge=abc"]),
        Some("abc".to_string())
    );
    assert_eq!(ChallengeCookie::read(["rullst_csrf=a; theme"]), None);
    assert_eq!(ChallengeCookie::read(std::iter::empty::<&str>()), None);
    assert_eq!(
        ChallengeCookie::read(["xrullst_oauth_challenge=abc; rullst_oauth_challengex=def"]),
        None
    );
}

#[test]
fn ambiguous_empty_or_oversized_values_are_refused() {
    assert_eq!(
        ChallengeCookie::read(["rullst_oauth_challenge=abc; rullst_oauth_challenge=abc"]),
        Some("abc".to_string())
    );
    assert_eq!(
        ChallengeCookie::read(["rullst_oauth_challenge=abc", "rullst_oauth_challenge=def"]),
        None
    );
    assert_eq!(ChallengeCookie::read(["rullst_oauth_challenge="]), None);
    let longest = "a".repeat(128);
    assert_eq!(
        ChallengeCookie::read([format!("rullst_oauth_challenge={longest}").as_str()]),
        Some(longest)
    );
    let oversized = format!("rullst_oauth_challenge={}", "a".repeat(129));
    assert_eq!(ChallengeCookie::read([oversized.as_str()]), None);
    // A later bad value refuses the whole read, even after a good one.
    assert_eq!(
        ChallengeCookie::read(["rullst_oauth_challenge=abc; rullst_oauth_challenge="]),
        None
    );
}
