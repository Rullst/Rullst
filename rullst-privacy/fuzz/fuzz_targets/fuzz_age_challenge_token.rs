#![no_main]

use libfuzzer_sys::fuzz_target;
use rullst_privacy::age_assurance::*;
use rullst_privacy_fuzz::*;

fuzz_target!(|data: &[u8]| {
    if data.len() > 9000 {
        return;
    }
    let retained = challenge(AgeMethod::SelfDeclaration);
    let original = retained.request_json().unwrap();
    let codec = tokens();
    let token = match data.first().copied().unwrap_or(0) % 4 {
        0 => match std::str::from_utf8(data.get(1..).unwrap_or_default()) {
            Ok(value) => value.to_owned(),
            Err(_) => return,
        },
        1 => authenticate_payload(data.get(1..).unwrap_or_default()),
        2 => authenticate_payload(&splice(&original, data.get(1..).unwrap_or_default())),
        _ => {
            let valid = codec.seal(&retained).unwrap();
            match String::from_utf8(splice(valid.as_bytes(), data.get(1..).unwrap_or_default())) {
                Ok(value) => value,
                Err(_) => return,
            }
        }
    };
    if let Ok(opened) = codec.open_with_clock(&token, &policy(), &binding(), &FixedClock(1001)) {
        let bytes = opened.request_json().unwrap();
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value["policy"], serde_json::to_value(policy()).unwrap());
        assert_eq!(value["binding"], serde_json::to_value(binding()).unwrap());
        assert_eq!(
            opened.threshold(),
            if opened.method() == AgeMethod::FacialEstimation {
                21
            } else {
                18
            }
        );
        let restored = codec
            .open_with_clock(
                &codec.seal(&opened).unwrap(),
                &policy(),
                &binding(),
                &FixedClock(1001),
            )
            .unwrap();
        assert_eq!(bytes, restored.request_json().unwrap());
        assert!(
            codec
                .open_with_clock(
                    &token,
                    &policy(),
                    &binding(),
                    &FixedClock(opened.expires_at())
                )
                .is_err()
        );
        let other = SubjectBinding::new("subject", "other-tenant", "session", "audience", "action")
            .unwrap();
        assert!(
            codec
                .open_with_clock(&token, &policy(), &other, &FixedClock(1001))
                .is_err()
        );
        let changed = AgePolicy::new("fuzz-policy-v2", RiskLevel::Low, 18).unwrap();
        assert!(
            codec
                .open_with_clock(&token, &changed, &binding(), &FixedClock(1001))
                .is_err()
        );
    }
});
