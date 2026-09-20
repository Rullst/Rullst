//! Production syntax linkage runs in ordinary CI; the prover is a manual pilot.
#[path = "fixtures/verus_linkage.rs"]
mod linkage;
use std::{fs, path::Path};
fn inputs() -> (String, String) {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    (
        fs::read_to_string(workspace.join("rullst-privacy/src/age_assurance/policy.rs")).unwrap(),
        fs::read_to_string(workspace.join("rullst-privacy/verification/age-policy.spec.rs"))
            .unwrap(),
    )
}
#[test]
fn age_policy_projection_copies_production_and_exports_all_negative_controls() {
    let (source, spec) = inputs();
    let mut projection = linkage::project(&source, &spec).unwrap();
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let lib = fs::read_to_string(workspace.join("rullst-privacy/src/lib.rs")).unwrap();
    let age =
        fs::read_to_string(workspace.join("rullst-privacy/src/age_assurance/mod.rs")).unwrap();
    projection.receipt["module_linkage"] = linkage::module_linkage(&lib, &age).unwrap();
    let manifest = fs::read_to_string(workspace.join("rullst-privacy/Cargo.toml")).unwrap();
    let manifest_value: toml::Value = toml::from_str(&manifest).unwrap();
    assert_eq!(
        manifest_value["package"]["name"].as_str(),
        Some("rullst-privacy")
    );
    assert!(
        manifest_value.get("lib").is_none(),
        "review any alternate crate root before proving source linkage"
    );
    assert!(manifest_value["features"].get("age-assurance").is_some());
    use sha2::{Digest, Sha256};
    projection.receipt["module_source_sha256"] = serde_json::json!({
        "rullst-privacy/Cargo.toml":hex::encode(Sha256::digest(manifest.as_bytes())),
        "rullst-privacy/src/lib.rs":hex::encode(Sha256::digest(lib.as_bytes())),
        "rullst-privacy/src/age_assurance/mod.rs":hex::encode(Sha256::digest(age.as_bytes())),
    });
    assert!(
        linkage::module_linkage(
            &lib,
            &age.replace("mod policy;", "#[path = \"other.rs\"]\nmod policy;")
        )
        .is_err()
    );
    assert!(
        linkage::module_linkage(
            &lib.replace("pub mod age_assurance;", "mod age_assurance;"),
            &age
        )
        .is_err()
    );
    assert_eq!(projection.cases.len(), 4);
    assert_eq!(
        projection.receipt["runtime_rewrites"],
        serde_json::json!([])
    );
    assert_eq!(projection.receipt["entry_point"], "AgePolicy::permits");
    assert_eq!(
        projection.cases,
        linkage::project(&source, &spec).unwrap().cases
    );
    if let Some(output) = std::env::var_os("RULLST_VERUS_OUTPUT") {
        let output = Path::new(&output);
        assert!(
            output.is_absolute(),
            "the runner must select an absolute fresh output directory"
        );
        fs::create_dir(output).unwrap();
        for (name, code) in projection.cases {
            fs::write(output.join(format!("{name}.rs")), code).unwrap();
        }
        fs::write(
            output.join("linkage.json"),
            serde_json::to_vec_pretty(&projection.receipt).unwrap(),
        )
        .unwrap();
    }
}
#[test]
fn changed_domains_signatures_macros_and_missing_bodies_require_review() {
    let (source, spec) = inputs();
    for (old, new) in [
        (
            "use serde::{Deserialize, Serialize};",
            "use serde::{Deserialize, Serialize};\nuse unreviewed::String;",
        ),
        ("pub enum RiskLevel", "struct String;\npub enum RiskLevel"),
        ("    Restricted,", "    Restricted,\n    Unknown,"),
        ("method: AgeMethod", "method: u8"),
        ("pub fn permits", "#[cfg(unix)]\n    pub fn permits"),
        (
            "pub enum RiskLevel",
            "#[unreviewed_macro]\npub enum RiskLevel",
        ),
        ("    risk: RiskLevel,", "    risk: u8,"),
        ("pub fn permits", "pub fn other_method"),
        (
            "RiskLevel::Low => true",
            "RiskLevel::Low => self.risk_spec() == RiskLevel::Low",
        ),
        ("RiskLevel::Low => true", "RiskLevel::Low => unreviewed!()"),
    ] {
        let changed = source.replacen(old, new, 1);
        assert_ne!(changed, source);
        assert!(
            linkage::project(&changed, &spec).is_err(),
            "unreviewed change accepted: {old}"
        );
    }
    let changed = source.replace(
        "RiskLevel::Restricted => method == AgeMethod::VerifiedAttribute",
        "RiskLevel::Restricted => true",
    );
    assert!(
        linkage::project(&changed, &spec).is_err(),
        "already-weakened code cannot masquerade as a functioning negative control"
    );
}
