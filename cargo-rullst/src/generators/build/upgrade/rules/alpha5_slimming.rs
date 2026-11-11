//! The 13.0.0-alpha.5 removals that make the framework smaller and that
//! application code can name: the Academy production preset
//! (`V13-ACADEMY-REMOVED`). [`super::tokens`] calls the identifier and path
//! rules with what it reads and [`super::rust`] the path rule with each `use`
//! path; the script rules live with the others in `files/scripts.rs`.

use super::Rule;
use super::catalog::ACADEMY_REMOVED;

/// The `rullst-core` Academy production types, removed with
/// `ProductionPreset::academy()`. Each name is specific to that preset.
const ACADEMY_IDENTS: &[&str] = &[
    "AcademyProductionRequirement",
    "AcademyCheckStatus",
    "AcademyBoundaryAssessment",
    "AcademyPresetError",
    "AcademyProductionPreset",
];

/// The rule for a distinctive identifier anywhere in the code, if any.
pub(super) fn ident_rule(name: &str) -> Option<&'static Rule> {
    ACADEMY_IDENTS.contains(&name).then_some(&ACADEMY_REMOVED)
}

/// The rule for a path such as `ProductionPreset::academy`, if any.
pub(super) fn path_rule(segments: &[String]) -> Option<&'static Rule> {
    segments
        .windows(2)
        .any(|pair| pair[0] == "ProductionPreset" && pair[1] == "academy")
        .then_some(&ACADEMY_REMOVED)
}
