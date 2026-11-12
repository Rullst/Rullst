//! 13.0.0-alpha.5: `OtaManager`'s state fields became private, dynamic
//! Alpine.js directive values in `html!` no longer compile, and the IoT codec
//! errors, Privacy's `ConsentChoice` and the Labs outcome and state enums
//! became `#[non_exhaustive]`.

use super::*;

const OTA: &str = "V13-IOT-OTA-STATE";
const ALPINE: &str = "V13-HTML-DYNAMIC-ALPINE-DIRECTIVE";
const CODEC: &str = "V13-IOT-CODEC-ERROR-MATCH";
const CARD: &str = "V13-IOT-SENSOR-CARD-HTML";
const LABS: &str = "V13-LABS-NON-EXHAUSTIVE-MATCH";
const CONSENT: &str = "V13-PRIVACY-CONSENT-CHOICE-MATCH";

#[test]
fn ota_manager_field_accesses_are_reviewed_once_per_file() {
    let source = r#"
use rullst_iot::{BootPartition, OtaManager};

fn boot(mut ota: OtaManager) -> OtaManager {
    ota.current_partition = BootPartition::PartitionB;
    assert_eq!(ota.status, rullst_iot::OtaStatus::Idle);
    println!("{}", ota.firmware_version);
    ota
}
"#;
    let found = scan_with(source, "lib.rs", &WorkspaceFacts::default());
    assert_eq!(found, [(OTA, 5)]);
    // Inside a macro body too, at its first access.
    for field in ["current_partition", "status", "firmware_version"] {
        let source = format!(
            "use rullst_iot::OtaManager;\nfn check(ota: &OtaManager) {{\n    assert!(matches!(ota.{field}, _));\n}}\n"
        );
        assert_eq!(
            scan_with(&source, "lib.rs", &WorkspaceFacts::default()),
            [(OTA, 3)],
            "{field}"
        );
    }
}

#[test]
fn the_getters_and_files_without_ota_manager_are_not_reported() {
    assert_only(
        r#"
use rullst_iot::OtaManager;

fn boot(ota: OtaManager) -> (bool, String) {
    let ota = ota.with_running_partition(rullst_iot::BootPartition::PartitionB);
    (ota.status() == rullst_iot::OtaStatus::Idle, ota.firmware_version().to_string())
}
"#,
        &[],
    );
    // `.status` of another type in a file that never names `OtaManager`.
    assert_only(
        "fn code(output: std::process::Output) -> bool { output.status.success() }\n",
        &[],
    );
    // Another field of the manager's file is not one of the three.
    assert_only(
        "use rullst_iot::OtaManager;\nfn other(item: &Item) -> u32 { item.partition_count }\n",
        &[],
    );
}

#[test]
fn dynamic_alpine_directive_values_must_change_at_every_location() {
    let source = r#"
fn view(state: &str, open: bool) -> Markup {
    html! {
        <div x-data={state} data-x-show={open} x-text="name"></div>
        <p x-init={format!("load({})", 1)} title={state}>"x"</p>
    }
}
"#;
    let found = scan_with(source, "lib.rs", &WorkspaceFacts::default());
    // One finding per line: both directives of line 4, then line 5.
    assert_eq!(found, [(ALPINE, 4), (ALPINE, 5)]);
    // Static directives, other dynamic attributes and `on*` handlers (their
    // own rule) are not Alpine findings.
    assert_only(
        r#"fn view(x: &str) -> Markup { html! { <div x-data="{ open: false }" data-x={x} max-x={x} onclick={x}></div> } }"#,
        &["V13-HTML-DYNAMIC-EVENT-HANDLER"],
    );
}

#[test]
fn exhaustive_matches_on_iot_codec_errors_must_change_at_every_location() {
    let source = r#"
use rullst_iot::{CoapCodecError, MqttCodecError};

fn mqtt(error: MqttCodecError) -> u8 {
    match error {
        MqttCodecError::EmptyTopic | MqttCodecError::InvalidTopic => 1,
        MqttCodecError::PacketTooLarge => 2,
        other => match other { MqttCodecError::TopicTooLong => 3, _ => 4 },
    }
}

fn coap(error: rullst::iot::CoapCodecError) -> &'static str {
    match error {
        rullst::iot::CoapCodecError::TokenTooLong => "token",
        CoapCodecError::DatagramTooLarge => "size",
    }
}
"#;
    // The outer MQTT `match` ends with a binding, which catches everything;
    // the CoAP one (line 13) has no catch-all arm.
    assert_eq!(
        scan_with(source, "lib.rs", &WorkspaceFacts::default()),
        [(CODEC, 13)]
    );
    let source = "fn f(e: MqttCodecError) -> bool { match e { MqttCodecError::EmptyTopic => true, MqttCodecError::InvalidTopic => false } }\nfn g(e: CoapCodecError) -> bool { match e { CoapCodecError::TokenTooLong => true, CoapCodecError::OptionTooLarge => false } }\n";
    assert_eq!(
        scan_with(source, "lib.rs", &WorkspaceFacts::default()),
        [(CODEC, 1), (CODEC, 2)]
    );
}

#[test]
fn codec_error_matches_with_a_wildcard_or_other_enums_are_not_reported() {
    assert_only(
        r#"
fn f(e: MqttCodecError) -> bool {
    match e {
        MqttCodecError::AllocationFailed => true,
        _ => false,
    }
}
fn g(e: CoapCodecError) -> bool {
    match e {
        CoapCodecError::AllocationFailed | _ => true,
    }
}
fn h(e: MyCodecError) -> bool {
    match e {
        MyCodecError::A => true,
        MyCodecError::B => false,
    }
}
fn i(e: MqttCodecError) -> bool { matches!(e, MqttCodecError::EmptyTopic) }
"#,
        &[],
    );
}

#[test]
fn sensor_card_calls_are_reviewed_once_per_file() {
    let source = r#"
use rullst_iot::IotDashboard;

fn page(reading: &rullst_iot::SensorTelemetry) -> String {
    let card: String = IotDashboard::render_sensor_card(reading);
    let again = rullst::iot::IotDashboard::render_sensor_card(reading);
    card + &again
}
"#;
    assert_eq!(
        scan_with(source, "lib.rs", &WorkspaceFacts::default()),
        [(CARD, 5)]
    );
    // Other renderers and the bare type are not reported.
    assert_only(
        "fn f(t: &T) -> String { IotDashboard::render_card(t) + &render_sensor_cards(t) }\n",
        &[],
    );
}

#[test]
fn exhaustive_matches_on_labs_enums_must_change_in_files_that_name_the_crate() {
    let source = r#"
use rullst_labs::sqlite::{CaseGrade, JobState};

fn label(state: JobState) -> &'static str {
    match state {
        JobState::Queued | JobState::Running => "waiting",
        JobState::Completed => "graded",
        JobState::Failed | JobState::Expired | JobState::Uncertain => "failed",
        JobState::Cancelled | JobState::Simulated => "stopped",
    }
}

fn grade(grade: CaseGrade) -> bool {
    match grade {
        CaseGrade::Passed => true,
        CaseGrade::WrongAnswer | CaseGrade::Trapped(_) => false,
    }
}

fn failure(error: rullst_labs::ExecutionFailure) -> u8 {
    match error {
        rullst_labs::ExecutionFailure::Compile => 1,
        _ => 2,
    }
}
"#;
    // The two matches without a catch-all arm, not the one with `_`.
    assert_eq!(
        scan_with(source, "lib.rs", &WorkspaceFacts::default()),
        [(LABS, 5), (LABS, 14)]
    );
    // Every listed enum counts, one per match.
    for name in [
        "JobResult",
        "CaseOutput",
        "TrapKind",
        "WorkerOutcome",
        "ExecutionFailure",
        "ResultEvidence",
        "ExecutionProfile",
    ] {
        let source = format!(
            "use rullst_labs::*;\nfn f(x: {name}) -> u8 {{ match x {{ {name}::A => 1, {name}::B(_) => 2 }} }}\n"
        );
        assert_eq!(
            scan_with(&source, "lib.rs", &WorkspaceFacts::default()),
            [(LABS, 2)],
            "{name}"
        );
    }
}

#[test]
fn labs_enum_names_elsewhere_or_with_a_wildcard_are_not_reported() {
    // An application's own `JobState` in a file that does not name the crate.
    assert_only(
        "enum JobState { A, B }\nfn f(s: JobState) -> u8 { match s { JobState::A => 1, JobState::B => 2 } }\n",
        &[],
    );
    assert_only(
        "use rullst_labs::sqlite::JobState;\nfn f(s: JobState) -> u8 { match s { JobState::Queued => 1, other => 2 } }\nfn g(s: Other) -> u8 { match s { Other::A => 1, Other::B => 2 } }\n",
        &[],
    );
    // The IoT rule keeps precedence for a match on a codec error.
    assert_eq!(
        scan_with(
            "use rullst_labs::JobState;\nfn f(e: MqttCodecError) -> u8 { match e { MqttCodecError::EmptyTopic => 1, MqttCodecError::InvalidTopic => 2 } }\n",
            "lib.rs",
            &WorkspaceFacts::default()
        ),
        [(CODEC, 2)]
    );
}

#[test]
fn exhaustive_consent_choice_matches_must_change_through_either_path() {
    for import in [
        "use rullst_privacy::consent::ConsentChoice;",
        "use rullst::privacy::consent::ConsentChoice;",
    ] {
        let source = format!(
            "{import}\nfn on(c: ConsentChoice) -> bool {{\n    match c {{\n        ConsentChoice::Granted => true,\n        ConsentChoice::Unset | ConsentChoice::Declined | ConsentChoice::Withdrawn => false,\n    }}\n}}\n"
        );
        assert_eq!(
            scan_with(&source, "lib.rs", &WorkspaceFacts::default()),
            [(CONSENT, 3)],
            "{import}"
        );
    }
    // With a wildcard, `==`, or an application's own `ConsentChoice`.
    assert_only(
        "use rullst_privacy::consent::ConsentChoice;\nfn on(c: ConsentChoice) -> bool { match c { ConsentChoice::Granted => true, _ => false } }\nfn off(c: ConsentChoice) -> bool { c == ConsentChoice::Declined }\n",
        &[],
    );
    assert_only(
        "enum ConsentChoice { Yes, No }\nfn on(c: ConsentChoice) -> bool { match c { ConsentChoice::Yes => true, ConsentChoice::No => false } }\n",
        &[],
    );
}
