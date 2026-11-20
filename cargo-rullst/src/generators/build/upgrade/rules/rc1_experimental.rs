//! 13.0.0-rc.1 took the word Experimental out of two public APIs.
//! `rullst-labs` renamed `ExecutionProfile::LinuxExperimental` to
//! `PinnedRunner` and `ResultEvidence::Experimental` to `SignedReceipt`
//! (`V13-LABS-VARIANTS-RENAMED`), and `rullst-iot` removed the
//! `experimental-simulators` feature with its fixtures
//! (`V13-IOT-SIMULATORS-REMOVED`; [`super::files`] locates the feature).
//! [`super::tokens`] calls these rules with the identifiers and paths it reads
//! and [`super::rust`] the path rule with each `use` path. A string literal
//! never matches, so a configuration that still names the stored
//! `"LinuxExperimental"` profile, as it must, is left alone.

use super::Rule;
use super::catalog::{IOT_SIMULATORS_REMOVED, LABS_VARIANTS_RENAMED};

/// The removed `rullst-iot` fixture types; distinctive enough on their own.
const REMOVED_IOT_FIXTURES: &[&str] = &[
    "SimulatedHsmDevice",
    "SimulatedHsmProfile",
    "SimulatedPqcFixture",
    "SimulatedMqttPayloadFormatter",
];

/// The rule for an identifier: the old Labs profile variant, also bare after
/// `use ExecutionProfile::*`, or a removed IoT fixture type.
pub(super) fn ident_rule(name: &str) -> Option<&'static Rule> {
    if name == "LinuxExperimental" {
        return Some(&LABS_VARIANTS_RENAMED);
    }
    REMOVED_IOT_FIXTURES
        .contains(&name)
        .then_some(&IOT_SIMULATORS_REMOVED)
}

/// The rule for a path that names `ResultEvidence::Experimental`; a bare
/// `Experimental` is too common a name to report.
pub(super) fn path_rule(segments: &[String]) -> Option<&'static Rule> {
    segments
        .windows(2)
        .any(|pair| pair[0] == "ResultEvidence" && pair[1] == "Experimental")
        .then_some(&LABS_VARIANTS_RENAMED)
}
