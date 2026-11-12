//! The 13.0.0-alpha.5 removals that make the framework smaller and that
//! application code can name: the Academy production preset
//! (`V13-ACADEMY-REMOVED`), the DeepSeek and Ollama provider types, now
//! presets of the OpenAI-compatible adapter (`V13-AI-PROVIDER-PRESETS`), and
//! the Qdrant adapter of `rullst-orm` (`V13-ORM-QDRANT-REMOVED`, whose Cargo
//! features [`super::files`] locates).
//! [`super::tokens`] calls the identifier and path
//! rules with what it reads and [`super::rust`] the path rule with each `use`
//! path; the script rules live with the others in `files/scripts.rs`.

use super::Rule;
use super::catalog::{ACADEMY_REMOVED, AI_PROVIDER_PRESETS, ORM_QDRANT_REMOVED};

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

/// The public names of the removed Qdrant adapter, exported by
/// `rullst_orm::polyglot` and the crate root. `VectorPoint` and its siblings
/// are generic, so they count only in a file that names the ORM.
const QDRANT_IDENTS: &[&str] = &[
    "QdrantStore",
    "QdrantConfig",
    "VectorRepository",
    "VectorCollectionName",
    "VectorDimensions",
    "VectorPoint",
    "VectorMatch",
    "VectorQueryLimit",
];

/// The `Backend` and `Capability` variants only the Qdrant adapter declared.
const QDRANT_VARIANTS: &[[&str; 2]] = &[["Backend", "Qdrant"], ["Capability", "Vectors"]];

fn names_orm(text: &str) -> bool {
    text.contains("rullst_orm") || text.contains("orm::") || text.contains("polyglot")
}

/// The rule for a distinctive identifier, given the file's text, if any.
pub(super) fn ident_rule(name: &str, text: &str) -> Option<&'static Rule> {
    if ACADEMY_IDENTS.contains(&name) {
        return Some(&ACADEMY_REMOVED);
    }
    if QDRANT_IDENTS.contains(&name) && names_orm(text) {
        return Some(&ORM_QDRANT_REMOVED);
    }
    (AI_PROVIDER_IDENTS.contains(&name) && names_ai_providers(text)).then_some(&AI_PROVIDER_PRESETS)
}

/// The rule for a path such as `ProductionPreset::academy`,
/// `providers::ollama` or `Backend::Qdrant`, if any; the enum variants only in
/// a file that names the ORM.
pub(super) fn path_rule(segments: &[String], text: &str) -> Option<&'static Rule> {
    let has = |left: &str, right: &str| {
        segments
            .windows(2)
            .any(|pair| pair[0] == left && pair[1] == right)
    };
    if has("ProductionPreset", "academy") {
        return Some(&ACADEMY_REMOVED);
    }
    if names_orm(text) && QDRANT_VARIANTS.iter().any(|[left, right]| has(left, right)) {
        return Some(&ORM_QDRANT_REMOVED);
    }
    AI_PROVIDER_MODULES
        .iter()
        .any(|[left, right]| has(left, right))
        .then_some(&AI_PROVIDER_PRESETS)
}
