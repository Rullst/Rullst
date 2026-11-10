//! 13.0.0-alpha.5: `OtaManager`'s state fields became private, and dynamic
//! Alpine.js directive values in `html!` no longer compile.

use super::*;

const OTA: &str = "V13-IOT-OTA-STATE";
const ALPINE: &str = "V13-HTML-DYNAMIC-ALPINE-DIRECTIVE";

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
