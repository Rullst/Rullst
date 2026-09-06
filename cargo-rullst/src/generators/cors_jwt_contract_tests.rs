#![allow(clippy::expect_used)]

use super::*;

#[test]
fn dependency_updates_fail_closed_without_a_dependency_table() {
    let manifest = "[package]\nname = \"missing-dependencies\"\n";
    assert!(
        ensure_jwt_dependencies(manifest)
            .expect_err("JWT update must require dependencies")
            .to_string()
            .contains("[dependencies]")
    );
    assert!(
        ensure_tower_http_cors_dependency(manifest)
            .expect_err("CORS update must require dependencies")
            .to_string()
            .contains("[dependencies]")
    );
}

#[test]
fn cors_feature_update_accepts_supported_toml_shapes_and_preserves_comments() {
    for (declaration, expected) in [
        (
            "tower-http = '0.7' # reviewed\n",
            "tower-http = { version = '0.7', features = [\"cors\"] } # reviewed",
        ),
        (
            "tower-http = { version = \"0.7\" }\n",
            "tower-http = { version = \"0.7\", features = [\"cors\"]}",
        ),
        ("tower-http = {}\n", "tower-http = {features = [\"cors\"]}"),
    ] {
        let manifest = format!("[dependencies]\n{declaration}");
        let (updated, changed) =
            ensure_tower_http_cors_dependency(&manifest).expect("supported declaration");
        assert!(changed);
        assert!(updated.contains(expected), "unexpected update:\n{updated}");
    }

    let quoted_key =
        "[dependencies]\n\"tower-http\" = { version = \"0.7\", features = ['cors'] }\n";
    let (unchanged, changed) = ensure_tower_http_cors_dependency(quoted_key).expect("quoted key");
    assert!(!changed);
    assert_eq!(unchanged, quoted_key);
}

#[test]
fn malformed_tower_http_declarations_are_never_rewritten() {
    assert!(
        add_cors_feature_to_dependency("tower-http")
            .expect_err("missing equals sign")
            .to_string()
            .contains("invalid tower-http")
    );
    assert!(
        add_cors_feature_to_dependency("tower-http = { version = \"0.7\", features = [\"trace\" }")
            .expect_err("unterminated feature array")
            .to_string()
            .contains("inline array")
    );
    for declaration in [
        "tower-http = { version = \"0.7\"",
        "tower-http = [\"0.7\"]",
        "tower-http = true",
    ] {
        assert!(
            add_cors_feature_to_dependency(declaration)
                .expect_err("unsupported TOML shape")
                .to_string()
                .contains("version string or single-line inline table")
        );
    }
}
