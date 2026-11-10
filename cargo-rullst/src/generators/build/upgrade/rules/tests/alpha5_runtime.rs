//! 13.0.0-alpha.5: `OtaManager`'s state fields became private, dynamic
//! Alpine.js directive values in `html!` no longer compile, and the IoT codec
//! errors became `#[non_exhaustive]`.

use super::*;

const OTA: &str = "V13-IOT-OTA-STATE";
const ALPINE: &str = "V13-HTML-DYNAMIC-ALPINE-DIRECTIVE";
const CODEC: &str = "V13-IOT-CODEC-ERROR-MATCH";

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
