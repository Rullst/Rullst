//! The 13.0.0-alpha.5 removals that make the framework smaller and that
//! application code can name: the Academy production preset
//! (`V13-ACADEMY-REMOVED`) and the DeepSeek and Ollama provider types, now
//! presets of the OpenAI-compatible adapter (`V13-AI-PROVIDER-PRESETS`).
//! [`super::tokens`] calls the identifier and path
//! rules with what it reads and [`super::rust`] the path rule with each `use`
//! path; the script rules live with the others in `files/scripts.rs`.

use super::Rule;
use super::catalog::{ACADEMY_REMOVED, AI_PROVIDER_PRESETS};

/// The `rullst-core` Academy production types, removed with
/// `ProductionPreset::academy()`. Each name is specific to that preset.
const ACADEMY_IDENTS: &[&str] = &[
    "AcademyProductionRequirement",
    "AcademyCheckStatus",
    "AcademyBoundaryAssessment",
    "AcademyPresetError",
    "AcademyProductionPreset",
];

/// The `rullst-ai` provider types replaced by the compatible presets. An
/// application can give its own type such a name, so they count only in a
/// file that names `rullst-ai` (`rullst_ai` or `ai::providers`).
const AI_PROVIDER_IDENTS: &[&str] = &["DeepSeekProvider", "OllamaProvider"];

/// The removed provider modules, `rullst_ai::ai::providers::deepseek` and
/// `::ollama` (also through `rullst::ai::providers`).
const AI_PROVIDER_MODULES: &[[&str; 2]] = &[["providers", "deepseek"], ["providers", "ollama"]];

fn names_ai_providers(text: &str) -> bool {
    text.contains("rullst_ai") || text.contains("ai::providers")
}

/// The rule for a distinctive identifier, given the file's text, if any.
pub(super) fn ident_rule(name: &str, text: &str) -> Option<&'static Rule> {
    if ACADEMY_IDENTS.contains(&name) {
        return Some(&ACADEMY_REMOVED);
    }
    (AI_PROVIDER_IDENTS.contains(&name) && names_ai_providers(text)).then_some(&AI_PROVIDER_PRESETS)
}

/// The rule for a path such as `ProductionPreset::academy` or
/// `providers::ollama`, if any.
pub(super) fn path_rule(segments: &[String]) -> Option<&'static Rule> {
    let has = |left: &str, right: &str| {
        segments
            .windows(2)
            .any(|pair| pair[0] == left && pair[1] == right)
    };
    if has("ProductionPreset", "academy") {
        return Some(&ACADEMY_REMOVED);
    }
    AI_PROVIDER_MODULES
        .iter()
        .any(|[left, right]| has(left, right))
        .then_some(&AI_PROVIDER_PRESETS)
}
