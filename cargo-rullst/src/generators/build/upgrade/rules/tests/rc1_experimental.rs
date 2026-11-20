//! 13.0.0-rc.1 took the word Experimental out of two public APIs: Labs
//! renamed `ExecutionProfile::LinuxExperimental` and
//! `ResultEvidence::Experimental` (`V13-LABS-VARIANTS-RENAMED`), and IoT
//! removed the `experimental-simulators` feature with its fixtures
//! (`V13-IOT-SIMULATORS-REMOVED`).

use super::alpha4_orm_nexus::manifest;
use super::*;

const LABS: &str = "V13-LABS-VARIANTS-RENAMED";
const IOT: &str = "V13-IOT-SIMULATORS-REMOVED";

/// The lines at which `code` is found in `source`.
fn found(source: &str, code: &str) -> Vec<usize> {
    scan_with(source, "lib.rs", &WorkspaceFacts::default())
        .into_iter()
        .filter(|(found, _)| *found == code)
        .map(|(_, line)| line)
        .collect()
}

#[test]
fn the_rc1_rules_must_change_and_name_the_replacements() {
    let alpha6 = Version::parse("13.0.0-alpha.6").unwrap();
    let rc1 = Version::parse("13.0.0-rc.1").unwrap();
    for (rule, code, row, replacement) in [
        (
            &catalog::LABS_VARIANTS_RENAMED,
            LABS,
            "Labs pinned-runner names",
            "ResultEvidence::SignedReceipt",
        ),
        (
            &catalog::IOT_SIMULATORS_REMOVED,
            IOT,
            "IoT simulator fixtures removed",
            "sha2::Sha256::digest",
        ),
    ] {
        assert_eq!(rule.code, code);
        assert_eq!(rule.kind, FindingKind::MustChange, "{code}");
        assert_eq!(rule.introduced, "13.0.0-rc.1", "{code}");
        assert_eq!(rule.row, row, "{code}");
        assert!(rule.message.contains("13.0.0-rc.1"), "{code}");
        assert!(rule.guidance.contains(replacement), "{code}");
        // An alpha.6 application gets them; an rc.1 one already has the change.
        assert!(rule.introduced_after(&alpha6), "{code}");
        assert!(!rule.introduced_after(&rc1), "{code}");
        // Must-change rules stop the gate themselves; only review rules for
        // removed APIs repeat their guidance there.
        assert!(!REMOVED_API_RULES.iter().any(|removed| removed.code == code));
    }
    let labs = &catalog::LABS_VARIANTS_RENAMED;
    assert!(labs.guidance.contains("ExecutionProfile::PinnedRunner"));
    assert!(labs.guidance.contains("keeps `\"LinuxExperimental\"`"));
}

#[test]
fn old_labs_variant_paths_must_change() {
    let source = r#"
use rullst_labs::sqlite::ResultEvidence;
use rullst_labs::{ContentHash, ExecutionProfile, ToolIdentity};
fn profile(tools: ToolIdentity, receipt_key: ContentHash) -> ExecutionProfile {
    ExecutionProfile::LinuxExperimental { tools, receipt_key }
}
fn signed(evidence: &ResultEvidence) -> bool {
    matches!(evidence, ResultEvidence::Experimental { .. })
}
fn key(profile: &ExecutionProfile) -> Option<&ContentHash> {
    let ExecutionProfile::LinuxExperimental { receipt_key, .. } = profile else {
        return None;
    };
    match profile {
        rullst_labs::ExecutionProfile::LinuxExperimental { .. } => Some(receipt_key),
        _ => None,
    }
}
"#;
    assert_eq!(found(source, LABS), [5, 8, 11, 15]);
    assert_eq!(found(source, IOT), Vec::<usize>::new());
}

#[test]
fn old_labs_variants_in_use_trees_and_after_glob_imports_must_change() {
    let source = r#"
use rullst_labs::sqlite::ResultEvidence::Experimental;
use rullst_labs::sqlite::ResultEvidence::{Simulation, Experimental as Signed};
use rullst_labs::ExecutionProfile::{LinuxExperimental};
use rullst_labs::ExecutionProfile::*;
fn pinned(profile: &rullst_labs::ExecutionProfile) -> bool {
    matches!(profile, LinuxExperimental { .. })
}
"#;
    // Line 3 is a `use` group only the `use` tree scan expands.
    assert_eq!(found(source, LABS), [2, 3, 4, 7]);
}

#[test]
fn new_labs_names_strings_and_other_experimental_variants_are_left_alone() {
    let source = r#"
use rullst_labs::{ExecutionProfile, sqlite::ResultEvidence};
// LinuxExperimental and ResultEvidence::Experimental in a comment.
const STORED: &str = "{\"LinuxExperimental\":{}}";
const EVIDENCE: &str = "ResultEvidence::Experimental";
enum Maturity { Experimental, Stable }
fn current(profile: &ExecutionProfile, evidence: &ResultEvidence, maturity: Maturity) -> bool {
    matches!(profile, ExecutionProfile::PinnedRunner { .. })
        && matches!(evidence, ResultEvidence::SignedReceipt { .. })
        && !matches!(evidence, ResultEvidence::Simulation)
        && matches!(maturity, Maturity::Experimental)
}
"#;
    assert_eq!(found(source, LABS), Vec::<usize>::new());
}

#[test]
fn removed_iot_fixture_types_must_change() {
    let source = r#"
use rullst_iot::hsm::SimulatedHsmDevice;
use rullst_iot::hsm::SimulatedHsmProfile;
use rullst_iot::pqc::SimulatedPqcFixture;
use rullst_iot::SimulatedMqttPayloadFormatter;
"#;
    assert_eq!(found(source, IOT), [2, 3, 4, 5]);
    assert_eq!(found(source, LABS), Vec::<usize>::new());
    let kept = r#"
use rullst_iot::{MqttPublish, SensorTelemetry};
// SimulatedHsmDevice in a comment.
const NAME: &str = "SimulatedPqcFixture";
struct SimulatedClock;
"#;
    assert_eq!(found(kept, IOT), Vec::<usize>::new());
}

#[test]
fn manifests_report_the_removed_iot_feature() {
    for (text, line) in [
        (
            "[dependencies]\nrullst-iot = { version = \"=13.0.0-alpha.6\", features = [\"experimental-simulators\"] }\n",
            2,
        ),
        // A renamed dependency, beside other features.
        (
            "[dependencies]\nsensors = { package = \"rullst-iot\", version = \"=13.0.0-alpha.6\", default-features = false, features = [\"std\", \"experimental-simulators\"] }\n",
            2,
        ),
        (
            "[package]\nname = \"app\"\n\n[dev-dependencies.rullst-iot]\nversion = \"=13.0.0-alpha.6\"\nfeatures = [\"experimental-simulators\"]\n",
            4,
        ),
        (
            "[workspace]\nmembers = [\"app\"]\n\n[workspace.dependencies]\nrullst-iot = { version = \"=13.0.0-alpha.6\", features = [\"experimental-simulators\"] }\n",
            5,
        ),
        // Enabled from `[features]`, also through an optional dependency.
        (
            "[dependencies]\nrullst-iot = { version = \"=13.0.0-alpha.6\", optional = true }\n\n[features]\n# \"rullst-iot/experimental-simulators\"\nsimulators = [\"dep:rullst-iot\", \"rullst-iot?/experimental-simulators\"]\n",
            6,
        ),
        (
            "[dependencies]\nrullst-iot = \"=13.0.0-alpha.6\"\n\n[features]\nsimulators = [\"rullst-iot/experimental-simulators\"]\n",
            5,
        ),
    ] {
        assert_eq!(manifest(text).0, [(IOT, line)], "{text}");
    }
    for text in [
        "[dependencies]\nrullst-iot = { version = \"=13.0.0-alpha.6\", features = [\"std\"] }\n",
        // Another package's feature of the same name.
        "[dependencies]\nother = { version = \"1\", features = [\"experimental-simulators\"] }\nrullst-iot = \"=13.0.0-alpha.6\"\n\n[features]\nsim = [\"other/experimental-simulators\", \"rullst-iot/std\"]\nexperimental-simulators = []\n",
    ] {
        assert_eq!(manifest(text).0, [], "{text}");
    }
}
