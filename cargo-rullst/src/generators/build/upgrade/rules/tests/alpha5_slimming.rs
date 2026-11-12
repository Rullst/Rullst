//! Rules of 13.0.0-alpha.5 for the removals that make the framework smaller:
//! the Academy production preset and its `academy:doctor` command, and the
//! DeepSeek and Ollama provider types.

use super::super::files::runs_academy_doctor;
use super::files::Project;
use super::*;

const ACADEMY: &str = "V13-ACADEMY-REMOVED";
const PRESETS: &str = "V13-AI-PROVIDER-PRESETS";
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
fn deepseek_and_ollama_provider_paths_and_names_must_change() {
    let source = r#"
use rullst::ai::providers::{deepseek::DeepSeekProvider, openai::OpenAiProvider};
use rullst_ai::ai::providers::ollama;
fn local() -> ollama::OllamaProvider {
    ollama::OllamaProvider::new("127.0.0.1:11434", "llama3")
}
fn cloud() -> DeepSeekProvider {
    DeepSeekProvider::new("key").with_model("deepseek-v4-pro")
}
fn kept() -> OpenAiProvider { OpenAiProvider::new("key") }
"#;
    assert_eq!(found(source, PRESETS), [2, 3, 4, 5, 7, 8]);
    // Either name of the crate is enough for the type names.
    let facade =
        "use rullst::ai::providers::openai::OpenAiProvider;\nfn f(_: DeepSeekProvider) {}\n";
    assert_eq!(found(facade, PRESETS), [2]);
    let direct = "extern crate rullst_ai;\nfn f(_: OllamaProvider) {}\n";
    assert_eq!(found(direct, PRESETS), [2]);
}

#[test]
fn provider_names_outside_rullst_ai_and_kept_presets_do_not_match() {
    // An application's own types of the same names, without `rullst-ai`.
    let own = r#"
pub struct OllamaProvider { host: String }
pub struct DeepSeekProvider;
mod providers { pub mod ollama {} }
fn build() -> OllamaProvider { OllamaProvider { host: String::new() } }
"#;
    assert_eq!(found(own, PRESETS), Vec::<usize>::new());
    let presets = r#"
use rullst::ai::providers::openai_compatible::OpenAiCompatibleProvider;
// The removed OllamaProvider and DeepSeekProvider are documented here.
const TEXT: &str = "rullst::ai::providers::ollama::OllamaProvider";
fn local() -> Result<OpenAiCompatibleProvider, rullst::ai::AiError> {
    OpenAiCompatibleProvider::ollama("127.0.0.1:11434", "llama3")
}
fn cloud() -> Result<OpenAiCompatibleProvider, rullst::ai::AiError> {
    OpenAiCompatibleProvider::deepseek("key")?.try_with_model("deepseek-v4-pro")
}
"#;
    assert_eq!(found(presets, PRESETS), Vec::<usize>::new());
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
    for (rule, kind, row) in [
        (
            &catalog::ACADEMY_REMOVED,
            FindingKind::MustChange,
            "Academy production preset removed",
        ),
        (
            &catalog::AI_PROVIDER_PRESETS,
            FindingKind::MustChange,
            "DeepSeek and Ollama as OpenAI-compatible presets",
        ),
    ] {
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
    assert!(
        catalog::AI_PROVIDER_PRESETS
            .guidance
            .contains("OpenAiCompatibleProvider::ollama")
    );
}
