//! Policy limits, claim checks and header rules at their exact boundaries.
use super::*;

const NOW: u64 = 1_800_000_000;

fn policy_with_ttl(seconds: u64) -> Result<ApplicationJwtPolicy, JwtError> {
    ApplicationJwtPolicy::development(
        "https://auth.example.test",
        "rullst-academy",
        Duration::from_secs(seconds),
        signing_key("2026-08-a", FIRST_SECRET),
    )
}

fn claims_at(now: u64) -> ApplicationJwtClaims {
    ApplicationJwtClaims {
        sub: "learner-7".to_string(),
        iss: "https://auth.example.test".to_string(),
        aud: "rullst-academy".to_string(),
        iat: now,
        nbf: now,
        exp: now + 60,
        jti: "0123456789abcdef".to_string(),
        session_version: 1,
        scopes: vec!["a:read".to_string(), "b:write".to_string()],
        token_use: "access".to_string(),
        schema_version: 1,
    }
}

/// Signs arbitrary claims with the development key, outside `issue`.
fn signed(header: &Header, claims: &serde_json::Value) -> String {
    encode(header, claims, &EncodingKey::from_secret(FIRST_SECRET)).expect("signed token")
}

fn kid_header() -> Header {
    let mut header = Header::new(Algorithm::HS256);
    header.kid = Some("2026-08-a".to_string());
    header
}

/// A valid token padded with an ignored claim to exactly `length` bytes.
fn token_of_length(length: usize) -> String {
    let base = serde_json::to_value(claims_at(unix_time().expect("clock"))).expect("claims");
    let mut header = kid_header();
    for content_type in [None, Some("x".to_string())] {
        header.cty = content_type;
        let padded = |pad: usize| {
            let mut claims = base.clone();
            claims["pad"] = serde_json::Value::String("p".repeat(pad));
            signed(&header, &claims)
        };
        let estimate = (length - padded(0).len()) * 3 / 4;
        for pad in estimate.saturating_sub(4)..estimate + 4 {
            let token = padded(pad);
            if token.len() == length {
                return token;
            }
        }
    }
    panic!("no padding reaches {length} bytes");
}

struct AsyncRevocations(JwtRevocationMode);

impl AsyncJwtRevocationStore for AsyncRevocations {
    fn mode(&self) -> JwtRevocationMode {
        self.0
    }

    async fn is_revoked(&self, _: &ApplicationJwtClaims, _: u64) -> Result<bool, JwtError> {
        Ok(false)
    }
}

#[test]
fn policy_limits_accept_their_maximum_and_reject_the_next_unit() {
    assert_eq!(signing_key("2026-08-a", FIRST_SECRET).kid(), "2026-08-a");
    assert!(policy_with_ttl(2_592_000).is_ok(), "thirty days is allowed");
    assert_eq!(
        policy_with_ttl(2_592_001).err(),
        Some(JwtError::InvalidConfiguration("max_ttl"))
    );
    assert!(development_policy().with_clock_skew(Duration::ZERO).is_ok());
    assert_eq!(
        development_policy()
            .with_clock_skew(Duration::from_secs(301))
            .err(),
        Some(JwtError::InvalidConfiguration("clock_skew"))
    );
    let longest = "i".repeat(256);
    assert!(
        ApplicationJwtPolicy::development(
            longest.as_str(),
            longest.as_str(),
            Duration::from_secs(60),
            signing_key("2026-08-a", FIRST_SECRET),
        )
        .is_ok()
    );
    let too_long = "i".repeat(257);
    for bad in ["", " issuer", "issuer ", "iss\u{7}uer", too_long.as_str()] {
        assert_eq!(
            ApplicationJwtPolicy::development(
                bad,
                "rullst-academy",
                Duration::from_secs(60),
                signing_key("2026-08-a", FIRST_SECRET),
            )
            .err(),
            Some(JwtError::InvalidConfiguration("issuer"))
        );
        assert_eq!(
            ApplicationJwtPolicy::development(
                "https://auth.example.test",
                bad,
                Duration::from_secs(60),
                signing_key("2026-08-a", FIRST_SECRET),
            )
            .err(),
            Some(JwtError::InvalidConfiguration("audience"))
        );
    }
}

#[test]
fn issuance_checks_subject_version_lifetime_and_scopes() {
    let policy = development_policy();
    for (subject, version) in [("learner", 0), (" learner", 1), ("", 1)] {
        assert_eq!(
            policy.issue(subject, ["course:read"], version, Duration::from_secs(60)),
            Err(JwtError::InvalidConfiguration("subject"))
        );
    }
    let token = policy
        .issue("learner", ["course:read"], 1, Duration::from_secs(3_600))
        .expect("the policy's maximum lifetime is allowed");
    let store = InMemoryJwtRevocationStore::new(16).expect("revocation store");
    let claims = policy.verify(&token, &store).expect("verified");
    assert_eq!(claims.exp - claims.iat, 3_600);

    let scopes: Vec<String> = (0..65).map(|index| format!("scope{index:02}")).collect();
    let issued = policy
        .issue("learner", scopes[..64].to_vec(), 1, Duration::from_secs(60))
        .expect("sixty-four scopes are allowed");
    assert_eq!(
        policy
            .verify(&issued, &store)
            .expect("verified")
            .scopes
            .len(),
        64
    );
    assert_eq!(
        policy.issue("learner", scopes, 1, Duration::from_secs(60)),
        Err(JwtError::InvalidConfiguration("scopes"))
    );
    for bad in ["-read", "_read", ":read", ".read", "bad scope", ""] {
        assert_eq!(
            policy.issue("learner", [bad], 1, Duration::from_secs(60)),
            Err(JwtError::InvalidConfiguration("scopes")),
            "scope {bad:?}"
        );
    }
}

#[test]
fn each_claim_rule_rejects_on_its_own() {
    let policy = development_policy();
    assert_eq!(policy.validate_claims(&claims_at(NOW), NOW), Ok(()));
    let mut at_skew = claims_at(NOW + 30);
    at_skew.exp = NOW + 90;
    assert_eq!(policy.validate_claims(&at_skew, NOW), Ok(()));
    let mut longest = claims_at(NOW);
    longest.exp = NOW + 3_600;
    assert_eq!(policy.validate_claims(&longest, NOW), Ok(()));

    let violations: [fn(&mut ApplicationJwtClaims); 11] = [
        |c| c.schema_version = 2,
        |c| c.token_use = "refresh".to_string(),
        |c| c.sub = " learner-7".to_string(),
        |c| c.jti = "j".repeat(65),
        |c| c.session_version = 0,
        |c| {
            c.iat = NOW + 31;
            c.nbf = NOW + 31;
            c.exp = NOW + 91;
        },
        |c| c.exp = NOW,
        |c| c.nbf = NOW + 1,
        |c| {
            c.iat = NOW + 10;
            c.nbf = NOW + 10;
            c.exp = NOW + 10;
        },
        |c| c.exp = NOW + 3_601,
        |c| c.scopes = vec!["b:write".to_string(), "a:read".to_string()],
    ];
    for (index, violate) in violations.into_iter().enumerate() {
        let mut claims = claims_at(NOW);
        violate(&mut claims);
        assert_eq!(
            policy.validate_claims(&claims, NOW),
            Err(JwtError::InvalidToken),
            "violation {index}"
        );
    }
}

#[test]
fn tokens_need_a_jwt_type_and_at_most_sixteen_kibibytes() {
    let policy = development_policy();
    let store = InMemoryJwtRevocationStore::new(16).expect("revocation store");
    let claims = serde_json::to_value(claims_at(unix_time().expect("clock"))).expect("claims");
    assert!(
        policy
            .verify(&signed(&kid_header(), &claims), &store)
            .is_ok()
    );
    for content_type in [None, Some("JOSE".to_string())] {
        let mut header = kid_header();
        header.typ = content_type;
        assert_eq!(
            policy.verify(&signed(&header, &claims), &store),
            Err(JwtError::InvalidToken)
        );
    }
    let largest = token_of_length(16_384);
    assert!(policy.verify(&largest, &store).is_ok());
    let oversized = token_of_length(16_385);
    assert_eq!(
        policy.verify(&oversized, &store),
        Err(JwtError::InvalidToken)
    );
}

#[tokio::test]
async fn async_verification_requires_shared_revocation_only_in_production() {
    let development = development_policy();
    let production = ApplicationJwtPolicy::production(
        "https://auth.example.test",
        "rullst-academy",
        Duration::from_secs(3_600),
        signing_key("2026-08-a", FIRST_SECRET),
    )
    .expect("production policy");
    let token = development
        .issue("learner", ["course:read"], 1, Duration::from_secs(60))
        .expect("token");
    let local = AsyncRevocations(JwtRevocationMode::ProcessLocal);
    let shared = AsyncRevocations(JwtRevocationMode::Shared);
    assert!(development.verify_async(&token, &local).await.is_ok());
    assert!(production.verify_async(&token, &shared).await.is_ok());
    assert_eq!(
        production.verify_async(&token, &local).await,
        Err(JwtError::RevocationStoreNotShared)
    );
}

#[test]
fn in_memory_subject_revocations_need_an_identity_and_a_positive_version() {
    let store = InMemoryJwtRevocationStore::new(16).expect("revocation store");
    for (subject, version) in [("learner", 0), (" learner", 2), ("", 2)] {
        assert_eq!(
            store.revoke_subject_before(subject, version),
            Err(JwtError::InvalidConfiguration("subject revocation"))
        );
    }
    assert_eq!(store.entry_count().unwrap(), 0);
}

/// Each in-memory write prunes a token whose expiry second has arrived.
#[test]
fn in_memory_writes_prune_tokens_expiring_in_the_current_second() {
    let mut checked = 0;
    for _ in 0..10 {
        if checked == 2 {
            break;
        }
        let store = InMemoryJwtRevocationStore::new(16).expect("revocation store");
        let start = unix_time().expect("clock");
        let mut expiring = claims_at(start);
        expiring.exp = start + 1;
        store.revoke_token(&expiring).expect("token revocation");
        if store.entry_count().unwrap() != 1 {
            continue;
        }
        // Bounded, so a clock that never advances fails instead of hanging.
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        while unix_time().expect("clock") < start + 1 {
            assert!(
                std::time::Instant::now() < deadline,
                "the clock must advance"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        if checked == 0 {
            let mut later = claims_at(start);
            later.jti = "fedcba9876543210".to_string();
            later.exp = start + 600;
            store.revoke_token(&later).expect("token revocation");
        } else {
            store
                .revoke_subject_before("learner-9", 2)
                .expect("subject revocation");
        }
        if unix_time().expect("clock") != start + 1 {
            continue;
        }
        assert_eq!(store.entry_count().unwrap(), 1, "round {checked}");
        checked += 1;
    }
    assert_eq!(checked, 2, "both writes ran inside an expiry second");
}
