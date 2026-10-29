//! Optional features and dependencies of the generated manifest.

use super::*;
use crate::blueprints::SAAS_BLUEPRINT_ID;

fn manifest(blueprint: usize, wants_ai: bool) -> String {
    build_cargo_toml(
        "features",
        false,
        true,
        "Sqlite",
        &[],
        wants_ai,
        false,
        blueprint,
        "Zero-Bundle HTMX",
        &std::env::temp_dir().join(format!("rullst-manifest-{}", rand::random::<u64>())),
    )
    .expect("Cargo manifest")
}

fn rullst_features(manifest: &str) -> Vec<String> {
    let parsed: toml::Value = toml::from_str(manifest).expect("valid Cargo manifest");
    parsed["dependencies"]["rullst"]["features"]
        .as_array()
        .expect("rullst features")
        .iter()
        .map(|feature| feature.as_str().expect("feature name").to_owned())
        .collect()
}

#[test]
fn the_nexus_ai_assistant_follows_the_wizard_ai_choice() {
    let saas = rullst_features(&manifest(SAAS_BLUEPRINT_ID, false));
    assert!(saas.iter().any(|feature| feature == "nexus"), "{saas:?}");
    for absent in ["ai", "nexus-ai"] {
        assert!(!saas.iter().any(|feature| feature == absent), "{saas:?}");
    }

    let saas_ai = rullst_features(&manifest(SAAS_BLUEPRINT_ID, true));
    for present in ["ai", "nexus", "nexus-ai"] {
        assert!(
            saas_ai.iter().any(|feature| feature == present),
            "{saas_ai:?}"
        );
    }

    // The Blank and API starters mount no Nexus.
    let blank_ai = rullst_features(&manifest(BLANK_BLUEPRINT_ID, true));
    assert!(
        blank_ai.iter().any(|feature| feature == "ai"),
        "{blank_ai:?}"
    );
    assert!(!blank_ai.iter().any(|feature| feature.starts_with("nexus")));
}

