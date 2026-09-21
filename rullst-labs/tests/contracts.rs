use rullst_labs::*;
use serde_json::json;

fn reference(value: &str) -> Reference {
    Reference::new(value).unwrap()
}
fn exercise() -> Exercise {
    Exercise::new(
        Scope::new("school", "rust").unwrap(),
        reference("sum"),
        reference("v1"),
        vec![GraderCase {
            id: reference("case-1"),
            input: [3, 5],
            expected: 8,
        }],
        ExecutionLimits::new(30, 100_000, 64).unwrap(),
    )
    .unwrap()
}

#[test]
fn malformed_identity_and_expanded_execution_policy_are_rejected() {
    for value in ["", "../../host", "a/b", "name.service", "a b", "\0", "é"] {
        assert!(Reference::new(value).is_err());
        assert!(serde_json::from_value::<Reference>(json!(value)).is_err());
    }
    assert!(Reference::new("a".repeat(97)).is_err());
    for value in ["", "g".repeat(64).as_str(), "ABCDEF"] {
        assert!(ContentHash::new(value).is_err());
    }
    assert_eq!(
        ContentHash::of(b"abc").as_str(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    for (field, value) in [
        ("wall_seconds", 61u64),
        ("fuel_per_case", 1_000_001),
        ("memory_pages", 257),
        ("memory_pages", 31),
    ] {
        let mut limits = json!({"wall_seconds":30,"fuel_per_case":100_000,"memory_pages":64});
        limits[field] = json!(value);
        assert!(serde_json::from_value::<ExecutionLimits>(limits).is_err());
    }
    assert!(
        serde_json::from_value::<ExecutionLimits>(
            json!({"wall_seconds":30,"fuel_per_case":100_000,"memory_pages":64,"network":true})
        )
        .is_err()
    );
    for timestamp in [i64::MIN, -1, 0, 253_402_300_800, i64::MAX] {
        assert!(Permission::until(timestamp).is_err());
    }
}

#[test]
fn exercise_digest_binds_scope_revision_grader_and_limits() {
    let original = exercise();
    let digest = original.digest().unwrap();
    let original_wire = serde_json::to_value(&original).unwrap();
    assert_eq!(
        serde_json::from_value::<Exercise>(original_wire.clone())
            .unwrap()
            .digest()
            .unwrap(),
        digest
    );
    for (pointer, value) in [
        ("/scope/tenant", json!("other-school")),
        ("/scope/course", json!("other-course")),
        ("/revision", json!("v2")),
        ("/cases/0/expected", json!(9)),
        ("/cases/0/input/0", json!(4)),
        ("/limits/fuel_per_case", json!(200_000)),
    ] {
        let mut changed = original_wire.clone();
        *changed.pointer_mut(pointer).unwrap() = value;
        assert_ne!(
            serde_json::from_value::<Exercise>(changed)
                .unwrap()
                .digest()
                .unwrap(),
            digest
        );
    }
    let mut duplicate = original_wire.clone();
    duplicate["cases"] = json!([original_wire["cases"][0], original_wire["cases"][0]]);
    assert!(serde_json::from_value::<Exercise>(duplicate).is_err());
    let mut empty = original_wire.clone();
    empty["cases"] = json!([]);
    assert!(serde_json::from_value::<Exercise>(empty).is_err());
    let mut oversized = original_wire;
    oversized["cases"] = json!(
        (0..65)
            .map(|i| json!({"id":format!("case-{i}"),"input":[0,0],"expected":0}))
            .collect::<Vec<_>>()
    );
    assert!(serde_json::from_value::<Exercise>(oversized).is_err());
}

#[test]
fn untrusted_source_and_expected_answers_are_bounded_and_not_debug_output() {
    let secret_source = "fn hidden_learner_solution() {}";
    let source = RustSource::new(secret_source).unwrap();
    assert!(!format!("{source:?}").contains(secret_source));
    assert_eq!(source.digest(), ContentHash::of(secret_source.as_bytes()));
    assert_eq!(
        serde_json::from_str::<RustSource>(&serde_json::to_string(&source).unwrap())
            .unwrap()
            .expose_source(),
        secret_source
    );
    for source in [
        "".into(),
        " \r\n\t".into(),
        "a\0b".into(),
        "a".repeat(MAX_SOURCE_BYTES + 1),
    ] {
        assert!(RustSource::new(source).is_err());
    }
    assert!(RustSource::new("a".repeat(MAX_SOURCE_BYTES)).is_ok());
    let case = GraderCase {
        id: reference("hidden-case"),
        input: [7123456789, 9345678123],
        expected: 8234567891,
    };
    let debug = format!("{case:?}");
    assert!(!debug.contains("7123456789"));
    assert!(!debug.contains("8234567891"));
    let hidden = Exercise::new(
        Scope::new("school", "course").unwrap(),
        reference("lesson"),
        reference("v1"),
        vec![case],
        ExecutionLimits::new(30, 100_000, 64).unwrap(),
    )
    .unwrap();
    assert!(!format!("{hidden:?}").contains("8234567891"));
}

#[test]
fn duplicate_or_unknown_json_fields_cannot_change_grader_semantics() {
    let wire = serde_json::to_string(&exercise()).unwrap();
    let duplicate = wire.replacen(
        "\"revision\":\"v1\"",
        "\"revision\":\"v1\",\"revision\":\"v2\"",
        1,
    );
    assert!(serde_json::from_str::<Exercise>(&duplicate).is_err());
    let mut wire = serde_json::to_value(exercise()).unwrap();
    wire["shell"] = json!("untrusted command");
    assert!(serde_json::from_value::<Exercise>(wire).is_err());
}

#[test]
fn fully_escaped_compiler_diagnostics_fit_the_bounded_transport() {
    let output = WorkerOutput {
        binding: AttemptBinding {
            request: ContentHash::of(b"request"),
            profile: ContentHash::of(b"profile"),
            source: ContentHash::of(b"source"),
            nonce: reference(&"n".repeat(96)),
        },
        outcome: WorkerOutcome::CompileRejected {
            diagnostics: Diagnostic::new("\"".repeat(MAX_DIAGNOSTIC_BYTES)).unwrap(),
        },
    };
    let wire = serde_json::to_vec(&output).unwrap();
    assert!(wire.len() > 16_384);
    assert_eq!(WorkerOutput::from_bytes(&wire).unwrap(), output);
    assert!(WorkerOutput::from_bytes(&vec![b' '; 20_481]).is_err());
    #[cfg(feature = "receipt-signing")]
    {
        let signer = ReceiptSigner::from_seed(zeroize::Zeroizing::new([23; 32])).unwrap();
        let signed = signer
            .sign(ExecutionReceipt {
                output,
                started_at: 1_800_000_000,
                finished_at: 1_800_000_001,
                observation_digest: ContentHash::of(b"observations"),
                teardown: Teardown::Confirmed,
            })
            .unwrap();
        assert_eq!(
            SignedReceipt::from_bytes(&serde_json::to_vec(&signed).unwrap()).unwrap(),
            signed
        );
    }
}
