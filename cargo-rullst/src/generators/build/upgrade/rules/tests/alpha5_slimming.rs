//! Rules of 13.0.0-alpha.5 for the removals that make the framework smaller:
//! the Academy production preset and its `academy:doctor` command.

use super::super::files::runs_academy_doctor;
use super::files::Project;
use super::*;

const ACADEMY: &str = "V13-ACADEMY-REMOVED";
const PACKAGE: &str = "[package]\nname = \"app\"\n";

fn found(source: &str, code: &str) -> Vec<usize> {
    scan_with(source, "lib.rs", &WorkspaceFacts::default())
        .into_iter()
        .filter(|(found, _)| *found == code)
        .map(|(_, line)| line)
        .collect()
}

#[test]
fn academy_types_and_the_preset_constructor_must_change_at_every_location() {
    let source = r#"
use rullst::{AcademyCheckStatus, AcademyProductionRequirement, ProductionPreset};
use rullst_core::production::AcademyPresetError;
fn check() -> Result<(), AcademyPresetError> {
    let preset = ProductionPreset::academy();
    let all: Vec<rullst::AcademyBoundaryAssessment> = preset.assess([])?;
    preset.validate([(AcademyProductionRequirement::TenantIsolation, AcademyCheckStatus::Pass)])
}
fn typed(_preset: rullst::AcademyProductionPreset) {}
fn order() -> &'static [rullst::ProductionMiddlewareStage] {
    rullst::ProductionPreset::middleware_order()
}
"#;
    // One finding per line, at every line that names a removed item.
    assert_eq!(found(source, ACADEMY), [2, 3, 4, 5, 6, 7, 9]);
}

#[test]
fn other_academy_names_comments_and_strings_do_not_match() {
    let source = r##"
//! `ProductionPreset::academy()` and AcademyCheckStatus were removed.
/// See AcademyProductionPreset.
struct Academy { name: String }
enum AcademyStatus { Open }
const TEXT: &str = "ProductionPreset::academy() AcademyPresetError";
fn academy() -> Academy { Academy { name: "academy".into() } }
fn preset() -> &'static [rullst::ProductionMiddlewareStage] {
    let _ = my_app::Preset::academy();
    rullst::ProductionPreset::middleware_order()
}
"##;
    assert_eq!(found(source, ACADEMY), Vec::<usize>::new());
}

#[test]
fn scripts_running_academy_doctor_must_change() {
    let project = Project::new();
    project
        .write(
            ".github/workflows/ci.yml",
            "steps:\n  - run: cargo rullst doctor\n  - run: cargo rullst academy:doctor --json\n",
        )
        .write(
            "Makefile",
            "check:\n\trullst academy:doctor --evidence e.json\n",
        );
    let mut found = project.findings(PACKAGE);
    found.sort();
    assert_eq!(
        found,
        [
            (".github/workflows/ci.yml".to_string(), ACADEMY, 3),
            ("Makefile".to_string(), ACADEMY, 2),
        ]
    );
    for line in [
        "cargo rullst academy:doctor",
        "rullst academy:doctor --json",
        "x && cargo rullst academy:doctor",
    ] {
        assert!(runs_academy_doctor(line), "{line}");
    }
    for line in [
        "cargo rullst doctor",
        "cargo rullst deploy:doctor",
        "cargo rullst academy:doctors",
        "cargo rullst my-academy:doctor",
    ] {
        assert!(!runs_academy_doctor(line), "{line}");
    }
}

#[test]
fn the_slimming_rules_name_their_rows_kinds_and_release() {
    let alpha4 = Version::parse("13.0.0-alpha.4").unwrap();
    let alpha5 = Version::parse("13.0.0-alpha.5").unwrap();
    for (rule, kind, row) in [(
        &catalog::ACADEMY_REMOVED,
        FindingKind::MustChange,
        "Academy production preset removed",
    )] {
        assert_eq!(rule.kind, kind, "{}", rule.code);
        assert_eq!(rule.row, row, "{}", rule.code);
        assert!(rule.introduced_after(&alpha4), "{}", rule.code);
        assert!(!rule.introduced_after(&alpha5), "{}", rule.code);
    }
    assert!(
        catalog::ACADEMY_REMOVED
            .guidance
            .contains("ProductionPreset::middleware_order()")
    );
}
