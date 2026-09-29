use super::*;

const FIRST_SECRET: &[u8] = b"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const SECOND_SECRET: &[u8] = b"fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210";

fn signing_key(kid: &str, secret: &[u8]) -> JwtSigningKey {
    JwtSigningKey::new(kid, secret).expect("strong test signing key")
}

fn development_policy() -> ApplicationJwtPolicy {
    ApplicationJwtPolicy::development(
        "https://auth.example.test",
        "rullst-academy",
        Duration::from_secs(60 * 60),
        signing_key("2026-08-a", FIRST_SECRET),
    )
    .expect("development policy")
}

struct SharedTestRevocations(InMemoryJwtRevocationStore);

impl SharedTestRevocations {
    fn new() -> Self {
        Self(InMemoryJwtRevocationStore::new(32).expect("test revocation store"))
    }
}

impl JwtRevocationStore for SharedTestRevocations {
    fn mode(&self) -> JwtRevocationMode {
        JwtRevocationMode::Shared
    }

    fn is_revoked(&self, claims: &ApplicationJwtClaims, now: u64) -> Result<bool, JwtError> {
        self.0.is_revoked(claims, now)
    }
}

#[test]
fn development_round_trip_is_bound_to_claims_and_revocation() {
    let policy = development_policy();
    let revocations = InMemoryJwtRevocationStore::new(16).expect("revocation store");
    let token = policy
        .issue(
            "learner-7",
            ["course:read".to_string(), "score:write".to_string()],
            3,
            Duration::from_secs(300),
        )
        .expect("issued token");
    let claims = policy.verify(&token, &revocations).expect("verified token");
    assert_eq!(claims.sub, "learner-7");
    assert_eq!(claims.session_version, 3);
    assert_eq!(claims.scopes, ["course:read", "score:write"]);
    assert_eq!(claims.token_use, "access");
    assert_eq!(claims.schema_version, 1);

    revocations.revoke_token(&claims).expect("token revocation");
    assert_eq!(policy.verify(&token, &revocations), Err(JwtError::Revoked));
}

#[test]
fn expired_tokens_are_rejected_inside_clock_skew_after_revocation_expiry() {
    let policy = development_policy()
        .with_clock_skew(Duration::from_secs(300))
        .expect("bounded clock skew");
    let store = InMemoryJwtRevocationStore::new(16).expect("revocation store");
    let now = unix_time().expect("clock");
    let token = policy
        .issue_at(
            "learner-7".to_string(),
            ["course:read"],
            1,
            Duration::from_secs(60),
            now - 120,
        )
        .expect("historically issued token");

    // Revocation backends may discard entries at exp. Verification must use
    // the same hard expiry, even when future iat/nbf clock skew is tolerated.
    assert_eq!(policy.verify(&token, &store), Err(JwtError::InvalidToken));
}

#[test]
fn expiry_is_exclusive_while_future_issuance_clock_skew_is_preserved() {
    let policy = development_policy();
    let now = unix_time().expect("clock");
    let token = policy
        .issue_at(
            "learner-7".to_string(),
            ["course:read"],
            1,
            Duration::from_secs(60),
            now + 10,
        )
        .expect("issuer clock is ahead");
    let claims = policy
        .decode_and_validate(&token, now)
        .expect("bounded future clock skew remains supported");
    let store = InMemoryJwtRevocationStore::new(16).expect("revocation store");
    store.revoke_token(&claims).expect("revoke live token");
    assert!(store.is_revoked(&claims, claims.exp - 1).unwrap());
    assert!(!store.is_revoked(&claims, claims.exp).unwrap());
    assert!(policy.validate_claims(&claims, claims.exp - 1).is_ok());
    assert_eq!(
        policy.validate_claims(&claims, claims.exp),
        Err(JwtError::InvalidToken)
    );
}

// TM-AUTH-06: production rejects process-local revocation while a rotation
// retains the previous key only for bounded verification of existing tokens.
#[test]
fn production_requires_shared_revocation_and_rotation_keeps_previous_tokens_verifiable() {
    let first_key = signing_key("2026-08-a", FIRST_SECRET);
    let policy = ApplicationJwtPolicy::production(
        "https://auth.example.test",
        "rullst-academy",
        Duration::from_secs(600),
        first_key,
    )
    .expect("production policy");
    let old_token = policy
        .issue(
            "learner-9",
            ["course:read".to_string()],
            1,
            Duration::from_secs(300),
        )
        .expect("old token");
    let local = InMemoryJwtRevocationStore::new(16).expect("local store");
    assert_eq!(
        policy.verify(&old_token, &local),
        Err(JwtError::RevocationStoreNotShared)
    );

    let policy = policy
        .rotate(signing_key("2026-08-b", SECOND_SECRET))
        .expect("rotated policy");
    let new_token = policy
        .issue(
            "learner-9",
            ["course:read".to_string()],
            2,
            Duration::from_secs(300),
        )
        .expect("new token");
    let shared = SharedTestRevocations::new();
    assert_eq!(
        policy
            .verify(&old_token, &shared)
            .expect("old key retained")
            .session_version,
        1
    );
    assert_eq!(
        policy
            .verify(&new_token, &shared)
            .expect("new key active")
            .session_version,
        2
    );
}

#[test]
fn subject_version_revocation_rejects_only_older_sessions() {
    let policy = development_policy();
    let revocations = InMemoryJwtRevocationStore::new(16).expect("revocation store");
    let old = policy
        .issue(
            "instructor-2",
            Vec::<String>::new(),
            4,
            Duration::from_secs(300),
        )
        .expect("old session token");
    let current = policy
        .issue(
            "instructor-2",
            Vec::<String>::new(),
            5,
            Duration::from_secs(300),
        )
        .expect("current session token");
    revocations
        .revoke_subject_before("instructor-2", 5)
        .expect("subject revocation");
    assert_eq!(policy.verify(&old, &revocations), Err(JwtError::Revoked));
    assert!(policy.verify(&current, &revocations).is_ok());
}

#[test]
fn token_policy_rejects_weak_keys_wrong_audience_and_invalid_inputs() {
    assert!(matches!(
        JwtSigningKey::new("weak", b"short"),
        Err(JwtError::WeakSigningKey)
    ));
    assert!(JwtSigningKey::new("unsafe kid", FIRST_SECRET).is_err());
    assert!(InMemoryJwtRevocationStore::new(0).is_err());
    assert!(InMemoryJwtRevocationStore::new(1_000_001).is_err());

    let policy = development_policy();
    assert_eq!(
        policy.issue(
            "learner",
            Vec::<String>::new(),
            1,
            Duration::from_secs(3601)
        ),
        Err(JwtError::InvalidTimeToLive)
    );
    assert!(
        policy
            .issue(
                "learner",
                ["score:write".to_string(), "score:write".to_string()],
                1,
                Duration::from_secs(60),
            )
            .is_err()
    );

    let token = policy
        .issue(
            "learner",
            ["course:read".to_string()],
            1,
            Duration::from_secs(60),
        )
        .expect("token");
    let wrong_audience = ApplicationJwtPolicy::development(
        "https://auth.example.test",
        "another-service",
        Duration::from_secs(3600),
        signing_key("2026-08-a", FIRST_SECRET),
    )
    .expect("wrong audience policy");
    let revocations = InMemoryJwtRevocationStore::new(16).expect("revocation store");
    assert_eq!(
        wrong_audience.verify(&token, &revocations),
        Err(JwtError::InvalidToken)
    );
}

#[test]
fn revocation_store_is_bounded_and_subject_versions_only_advance() {
    let policy = development_policy();
    let store = InMemoryJwtRevocationStore::new(2).expect("bounded store");
    store
        .revoke_subject_before("learner-a", 8)
        .expect("first subject");
    store
        .revoke_subject_before("learner-a", 4)
        .expect("existing subject update");
    store
        .revoke_subject_before("learner-b", 2)
        .expect("second subject");
    assert_eq!(
        store.revoke_subject_before("learner-c", 2),
        Err(JwtError::RevocationStoreCapacity)
    );
    let token = policy
        .issue(
            "learner-a",
            Vec::<String>::new(),
            7,
            Duration::from_secs(60),
        )
        .expect("token");
    assert_eq!(policy.verify(&token, &store), Err(JwtError::Revoked));
}

fn verified(
    policy: &ApplicationJwtPolicy,
    subject: &str,
    issued_at: u64,
) -> (String, ApplicationJwtClaims) {
    let token = policy
        .issue_at(
            subject.to_string(),
            Vec::<String>::new(),
            1,
            Duration::from_secs(600),
            issued_at,
        )
        .expect("token");
    let claims = policy
        .decode_and_validate(&token, issued_at)
        .expect("fresh token");
    (token, claims)
}

// TM-AUTH-06: one principal repeatedly revoking its own tokens must neither
// exhaust shared capacity nor stop other subjects' logout and revocation.
#[test]
fn one_subject_cannot_exhaust_revocation_capacity_for_other_subjects() {
    let policy = development_policy();
    let store = InMemoryJwtRevocationStore::new(128).expect("bounded store");
    let now = unix_time().expect("clock");
    let (bystander_other, _) = verified(&policy, "learner-b", now - 1);
    let mut abusive = Vec::new();
    for _ in 0..200 {
        let (token, claims) = verified(&policy, "learner-a", now - 1);
        store.revoke_token(&claims).expect("revocation never fails");
        abusive.push(token);
    }
    for token in &abusive {
        assert_eq!(policy.verify(token, &store), Err(JwtError::Revoked));
    }
    // Widening revokes only tokens issued no later than the revoked ones.
    let (later, _) = verified(&policy, "learner-a", now);
    assert!(policy.verify(&later, &store).is_ok());

    let (bystander, claims) = verified(&policy, "learner-b", now - 1);
    store
        .revoke_token(&claims)
        .expect("another subject can still log out");
    assert_eq!(policy.verify(&bystander, &store), Err(JwtError::Revoked));
    // The bystander's revocation stayed exact rather than subject-wide.
    assert!(policy.verify(&bystander_other, &store).is_ok());
    store
        .revoke_subject_before("learner-c", 2)
        .expect("subject revocation still has capacity");
    assert!(store.entry_count().unwrap() <= 128);
}

#[test]
fn subject_revocations_keep_a_reserve_when_token_rows_fill_their_share() {
    let policy = development_policy();
    let store = InMemoryJwtRevocationStore::new(8).expect("bounded store");
    let now = unix_time().expect("clock");
    for index in 0..6 {
        let (_, claims) = verified(&policy, &format!("learner-{index}"), now - 1);
        store.revoke_token(&claims).expect("token row");
    }
    // Token rows are limited to three quarters of the quota; the seventh
    // subject's revocation widens to a subject cutoff that still rejects it.
    let (token, claims) = verified(&policy, "learner-6", now - 1);
    store.revoke_token(&claims).expect("widened revocation");
    assert_eq!(policy.verify(&token, &store), Err(JwtError::Revoked));
    store
        .revoke_subject_before("instructor-1", 2)
        .expect("reserved subject capacity");
    assert_eq!(store.entry_count().unwrap(), 8);
    assert_eq!(
        store.revoke_subject_before("instructor-2", 2),
        Err(JwtError::RevocationStoreCapacity)
    );
    // Existing subjects still update in place at full capacity.
    store
        .revoke_subject_before("learner-6", 3)
        .expect("existing subject update");
}
