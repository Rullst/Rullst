//! Deterministic generated wire cases. Source is data only, never compiled.
use rullst_labs::*;
use serde_json::json;

fn input(index: usize) -> WorkerInput {
    let source = RustSource::new(format!(
        "// private-source-{index}\n{}",
        ["\\\"", "é", "\t", "\r\n"][index % 4].repeat(index + 1)
    ))
    .unwrap();
    let limits = ExecutionLimits::new(
        [5, 60][index % 2],
        [1000, 1_000_000][index % 2],
        [32, 256][index % 2],
    )
    .unwrap();
    let values = [i64::MIN, -1, 0, 1, i64::MAX];
    WorkerInput::new(
        AttemptBinding {
            request: ContentHash::of(&index.to_le_bytes()),
            profile: ContentHash::of(b"fixed-profile"),
            source: source.digest(),
            nonce: Reference::new(format!("nonce-{index}")).unwrap(),
        },
        source,
        (0..=index % MAX_CASES)
            .map(|case| [values[case % 5], values[(case + index) % 5]])
            .collect(),
        limits,
    )
    .unwrap()
}

fn assert_input_invariants(decoded: &WorkerInput) {
    assert_eq!(decoded.binding().source, decoded.source().digest());
    assert!((1..=MAX_SOURCE_BYTES).contains(&decoded.source().expose_source().len()));
    assert!((1..=MAX_CASES).contains(&decoded.inputs().len()));
    assert!((5..=60).contains(&decoded.limits().wall_seconds()));
    assert!((1000..=1_000_000).contains(&decoded.limits().fuel_per_case()));
    assert!((32..=256).contains(&decoded.limits().memory_pages()));
}

#[test]
fn generated_wire_round_trips_keep_limits_cases_and_source_binding() {
    for index in 0..128 {
        let original = input(index);
        let bytes = serde_json::to_vec(&original).unwrap();
        let decoded = WorkerInput::from_bytes(&bytes).unwrap();
        assert_input_invariants(&decoded);
        assert_eq!(decoded.binding(), original.binding());
        assert_eq!(decoded.inputs(), original.inputs());
        assert_eq!(decoded.limits(), original.limits());
        assert_eq!(
            decoded.source().expose_source(),
            original.source().expose_source()
        );
        assert_eq!(serde_json::to_vec(&decoded).unwrap(), bytes);

        for (pointer, replacement) in [
            ("/source", json!("changed source")),
            ("/inputs", json!([])),
            ("/inputs", json!(vec![[0, 0]; MAX_CASES + 1])),
            ("/limits/wall_seconds", json!(u32::MAX)),
            ("/limits/fuel_per_case", json!(u64::MAX)),
            ("/limits/memory_pages", json!(u32::MAX)),
        ] {
            let mut changed = serde_json::to_value(&original).unwrap();
            *changed.pointer_mut(pointer).unwrap() = replacement;
            assert!(WorkerInput::from_bytes(&serde_json::to_vec(&changed).unwrap()).is_err());
        }
    }
    // The maximum escaped source must fit the transport even with every case.
    let source = RustSource::new("\"".repeat(MAX_SOURCE_BYTES)).unwrap();
    let mut binding = input(0).binding().clone();
    binding.source = source.digest();
    binding.nonce = Reference::new("n".repeat(96)).unwrap();
    let maximum = WorkerInput::new(
        binding,
        source,
        vec![[i64::MIN, i64::MAX]; MAX_CASES],
        ExecutionLimits::new(60, 1_000_000, 256).unwrap(),
    )
    .unwrap();
    let bytes = serde_json::to_vec(&maximum).unwrap();
    assert!(bytes.len() < 131_072);
    assert_input_invariants(&WorkerInput::from_bytes(&bytes).unwrap());
    assert!(WorkerInput::from_bytes(&vec![b' '; 131_073]).is_err());
}

#[test]
fn truncated_and_byte_mutated_frames_never_bypass_decoded_invariants() {
    let output = WorkerOutput {
        binding: input(7).binding().clone(),
        outcome: WorkerOutcome::Executed {
            artifact: ContentHash::of(b"artifact"),
            cases: vec![
                CaseOutput::Value(i64::MIN),
                CaseOutput::Trap(TrapKind::Fuel),
            ],
        },
    };
    let receipt = ExecutionReceipt {
        output: output.clone(),
        started_at: 1_800_000_000,
        finished_at: 1_800_000_001,
        observation_digest: ContentHash::of(b"observations"),
        teardown: Teardown::Confirmed,
    };
    let frames = [
        serde_json::to_vec(&input(7)).unwrap(),
        serde_json::to_vec(&output).unwrap(),
        // This is shape/parser coverage, not signature authenticity evidence.
        serde_json::to_vec(&json!({"receipt":receipt,"signature":"0".repeat(128)})).unwrap(),
    ];
    for (kind, bytes) in frames.iter().enumerate() {
        let accepts = |wire: &[u8]| match kind {
            0 => WorkerInput::from_bytes(wire).is_ok(),
            1 => WorkerOutput::from_bytes(wire).is_ok(),
            _ => SignedReceipt::from_bytes(wire).is_ok(),
        };
        for length in 0..bytes.len() {
            assert!(!accepts(&bytes[..length]));
        }
        for index in 0..bytes.len() {
            for mask in [1, 32, 128, 255] {
                let mut changed = bytes.clone();
                changed[index] ^= mask;
                if let Ok(decoded) = WorkerInput::from_bytes(&changed) {
                    assert_input_invariants(&decoded);
                    assert!(
                        WorkerInput::from_bytes(&serde_json::to_vec(&decoded).unwrap()).is_ok()
                    );
                }
                if let Ok(decoded) = WorkerOutput::from_bytes(&changed) {
                    decoded.validate().unwrap();
                    assert_eq!(
                        WorkerOutput::from_bytes(&serde_json::to_vec(&decoded).unwrap()).unwrap(),
                        decoded
                    );
                }
                if let Ok(decoded) = SignedReceipt::from_bytes(&changed) {
                    decoded.receipt.validate().unwrap();
                    assert_eq!(
                        SignedReceipt::from_bytes(&serde_json::to_vec(&decoded).unwrap()).unwrap(),
                        decoded
                    );
                }
            }
        }
    }
}
