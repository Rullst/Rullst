use super::*;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use ring::{
    hmac,
    rand::{SystemRandom, generate},
};
use std::sync::atomic::{AtomicUsize, Ordering};

fn secret() -> [u8; 32] {
    generate(&SystemRandom::new()).unwrap().expose()
}

fn signed_record(key: &[u8], domain: &[u8], bytes: &[u8]) -> String {
    let message = format!("ra1.active.{}", URL_SAFE_NO_PAD.encode(bytes));
    let mut authenticated = domain.to_vec();
    authenticated.extend_from_slice(message.as_bytes());
    let mac = hmac::sign(&hmac::Key::new(hmac::HMAC_SHA256, key), &authenticated);
    format!("{message}.{}", URL_SAFE_NO_PAD.encode(mac.as_ref()))
}

const DOMAIN: &[u8] = b"rullst.age-challenge-token.v1\0";

#[test]
fn token_restores_the_exact_server_challenge_and_checks_current_time_and_context() {
    let codec = ChallengeTokens::new("active", &secret()).unwrap();
    let original = challenge(AgeMethod::VerifiedAttribute);
    let token = codec.seal(&original).unwrap();
    let restored = codec
        .open_with_clock(&token, &policy(), &binding(), &FixedClock(1001))
        .unwrap();
    assert_eq!(
        restored.request_json().unwrap(),
        original.request_json().unwrap()
    );
    assert!(token.len() <= ChallengeTokens::MAX_TOKEN_BYTES);
    for (now, expected) in [(999, AgeError::InvalidChallenge), (1300, AgeError::Expired)] {
        assert!(
            matches!(codec.open_with_clock(&token, &policy(), &binding(), &FixedClock(now)), Err(error) if error == expected)
        );
    }
    let other = SubjectBinding::new(
        "other-subject",
        "school-1",
        "session-ref",
        "academy",
        "restricted-action",
    )
    .unwrap();
    assert!(matches!(
        codec.open_with_clock(&token, &policy(), &other, &FixedClock(1001)),
        Err(AgeError::BindingMismatch)
    ));
    let changed = AgePolicy::new("policy-v1", RiskLevel::Restricted, 18).unwrap();
    assert!(matches!(
        codec.open_with_clock(&token, &changed, &binding(), &FixedClock(1001)),
        Err(AgeError::BindingMismatch)
    ));
    let live = AgeChallenge::issue(
        &policy(),
        binding(),
        AgeMethod::VerifiedAttribute,
        SystemAgeClock.now().unwrap(),
    )
    .unwrap();
    assert!(
        codec
            .open(&codec.seal(&live).unwrap(), &policy(), &binding())
            .is_ok()
    );
    assert_eq!(format!("{codec:?}"), "ChallengeTokens([redacted])");
}

#[test]
fn active_and_previous_keys_rotate_without_allowing_key_id_substitution() {
    let old = secret();
    let new = secret();
    let old_codec = ChallengeTokens::new("old", &old).unwrap();
    let old_token = old_codec
        .seal(&challenge(AgeMethod::VerifiedAttribute))
        .unwrap();
    let rotated = ChallengeTokens::new("active", &new)
        .unwrap()
        .with_previous_key("old", &old)
        .unwrap();
    assert!(
        rotated
            .open_with_clock(&old_token, &policy(), &binding(), &FixedClock(1001))
            .is_ok()
    );
    let new_token = rotated
        .seal(&challenge(AgeMethod::VerifiedAttribute))
        .unwrap();
    assert!(new_token.starts_with("ra1.active."));
    assert!(matches!(
        old_codec.open_with_clock(&new_token, &policy(), &binding(), &FixedClock(1001)),
        Err(AgeError::InvalidChallengeToken)
    ));
    let retired = ChallengeTokens::new("active", &new).unwrap();
    assert!(matches!(
        retired.open_with_clock(&old_token, &policy(), &binding(), &FixedClock(1001)),
        Err(AgeError::InvalidChallengeToken)
    ));
    let shared_key_different_id = ChallengeTokens::new("active", &old).unwrap();
    let changed_id = old_token.replacen(".old.", ".active.", 1);
    assert!(matches!(
        shared_key_different_id.open_with_clock(
            &changed_id,
            &policy(),
            &binding(),
            &FixedClock(1001)
        ),
        Err(AgeError::InvalidChallengeToken)
    ));
}

struct ObservedClock(AtomicUsize);
impl AgeClock for ObservedClock {
    fn now(&self) -> Result<i64, AgeError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(1001)
    }
}

#[test]
fn every_single_bit_change_in_a_transport_token_is_rejected() {
    let codec = ChallengeTokens::new("active", &secret()).unwrap();
    let token = codec
        .seal(&challenge(AgeMethod::VerifiedAttribute))
        .unwrap();
    let original = token.as_bytes();
    let clock = ObservedClock(AtomicUsize::new(0));
    for offset in 0..original.len() {
        for bit in 0..7 {
            let mut changed = original.to_vec();
            changed[offset] ^= 1 << bit;
            let changed = String::from_utf8(changed).unwrap();
            assert!(
                codec
                    .open_with_clock(&changed, &policy(), &binding(), &clock)
                    .is_err(),
                "accepted a changed token byte at {offset}, bit {bit}"
            );
        }
    }
    assert_eq!(clock.0.load(Ordering::SeqCst), 0);
}

#[test]
fn malformed_or_unauthenticated_tokens_are_rejected_before_interpreting_the_challenge() {
    let key = secret();
    let codec = ChallengeTokens::new("active", &key).unwrap();
    let challenge = challenge(AgeMethod::VerifiedAttribute);
    let token = codec.seal(&challenge).unwrap();
    let mut fields: Vec<String> = token.split('.').map(str::to_owned).collect();
    fields[2] = URL_SAFE_NO_PAD.encode(b"not-json");
    let swapped_payload = fields.join(".");
    let clock = ObservedClock(AtomicUsize::new(0));
    for malformed in [
        String::new(),
        "ra1.active".to_owned(),
        format!("{token}.extra"),
        token.replacen("ra1.", "ra2.", 1),
        token.replacen(".active.", ".unknown.", 1),
        swapped_payload,
        format!("{token}="),
        "x".repeat(ChallengeTokens::MAX_TOKEN_BYTES + 1),
        signed_record(
            &key,
            b"another-domain\0",
            &challenge.request_json().unwrap(),
        ),
        signed_record(&secret(), DOMAIN, &challenge.request_json().unwrap()),
    ] {
        assert!(matches!(
            codec.open_with_clock(&malformed, &policy(), &binding(), &clock),
            Err(AgeError::InvalidChallengeToken)
        ));
    }
    assert_eq!(clock.0.load(Ordering::SeqCst), 0);
}

#[test]
fn authenticated_records_still_require_a_strict_schema_and_exact_policy_lifetime() {
    let key = secret();
    let codec = ChallengeTokens::new("active", &key).unwrap();
    let original = challenge(AgeMethod::VerifiedAttribute)
        .request_json()
        .unwrap();
    let mut extra: serde_json::Value = serde_json::from_slice(&original).unwrap();
    extra["unexpected"] = true.into();
    for bytes in [
        b"not-json".to_vec(),
        b"{}".to_vec(),
        serde_json::to_vec(&extra).unwrap(),
        vec![b'x'; 4097],
    ] {
        let token = signed_record(&key, DOMAIN, &bytes);
        assert!(matches!(
            codec.open_with_clock(&token, &policy(), &binding(), &FixedClock(1001)),
            Err(AgeError::InvalidChallengeToken)
        ));
    }
    for (issued, expires) in [(1000, 1301), (-1, 299), (i64::MAX, i64::MAX)] {
        let mut record: serde_json::Value = serde_json::from_slice(&original).unwrap();
        record["issued_at"] = issued.into();
        record["expires_at"] = expires.into();
        let token = signed_record(&key, DOMAIN, &serde_json::to_vec(&record).unwrap());
        assert!(matches!(
            codec.open_with_clock(&token, &policy(), &binding(), &FixedClock(1001)),
            Err(AgeError::InvalidChallenge)
        ));
    }
}

#[test]
fn key_configuration_is_bounded_and_rejects_ambiguous_ids_and_default_secrets() {
    let key = secret();
    for id in ["", ".", "a.b", "key id", "á", &"a".repeat(65)] {
        assert!(matches!(
            ChallengeTokens::new(id, &key),
            Err(AgeError::InvalidConfiguration)
        ));
    }
    for bad in [Vec::new(), vec![0; 32], vec![1; 31], vec![1; 33]] {
        assert!(matches!(
            ChallengeTokens::new("active", &bad),
            Err(AgeError::InvalidConfiguration)
        ));
    }
    let mut codec = ChallengeTokens::new("active", &key).unwrap();
    assert!(matches!(
        codec.clone().with_previous_key("active", &secret()),
        Err(AgeError::InvalidConfiguration)
    ));
    for index in 0..7 {
        codec = codec
            .with_previous_key(format!("previous-{index}"), &secret())
            .unwrap();
    }
    assert!(matches!(
        codec.with_previous_key("ninth", &secret()),
        Err(AgeError::InvalidConfiguration)
    ));
}

#[cfg(feature = "sqlite")]
#[tokio::test]
async fn transported_challenge_requires_current_identity_and_cannot_be_reused_on_another_application_instance()
 {
    let secret = secret();
    let first_codec = ChallengeTokens::new("active", &secret).unwrap();
    let second_codec = ChallengeTokens::new("active", &secret).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("replay.sqlite");
    let first = SqliteReplayStore::open(&path, 10).await.unwrap();
    let second = SqliteReplayStore::open(&path, 10).await.unwrap();
    let policy = AgePolicy::new("native-low-risk", RiskLevel::Low, 18).unwrap();
    let challenge =
        AgeChallenge::issue(&policy, binding(), AgeMethod::SelfDeclaration, 1000).unwrap();
    let token = first_codec.seal(&challenge).unwrap();
    let restored = second_codec
        .open_with_clock(&token, &policy, &binding(), &FixedClock(1001))
        .unwrap();
    let gate = DeclarationGate::new(second.clone()).unwrap();
    assert_eq!(
        gate.assess_with_clock(
            &policy,
            &binding(),
            &restored,
            AgeDeclaration::MeetsThreshold,
            &FixedClock(1001)
        )
        .await
        .unwrap()
        .assurance(),
        Assurance::Declared
    );
    let restored_again = first_codec
        .open_with_clock(&token, &policy, &binding(), &FixedClock(1001))
        .unwrap();
    let other = DeclarationGate::new(first.clone()).unwrap();
    assert_eq!(
        other
            .assess_with_clock(
                &policy,
                &binding(),
                &restored_again,
                AgeDeclaration::MeetsThreshold,
                &FixedClock(1001)
            )
            .await,
        Err(AgeError::Replay)
    );
    first.close().await;
    second.close().await;
}
