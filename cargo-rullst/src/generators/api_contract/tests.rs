use super::*;
use serde_json::{Value, json};
fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../tests/fixtures/api_contract/openapi.json"
    ))
    .unwrap()
}
fn accepts(value: Value) -> bool {
    operations::Document::parse(value).is_ok()
}
#[test]
fn supported_profile_generates_deterministically_and_checks_without_writes() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    let source = root.join("schema.json");
    let output = root.join("generated");
    fs::write(&source, serde_json::to_vec(&fixture()).unwrap()).unwrap();
    generate(&source, &output, false).unwrap();
    generate(&source, &output, true).unwrap();
    let initial = fs::read(output.join("client.ts")).unwrap();
    generate(&source, &output, false).unwrap();
    assert_eq!(initial, fs::read(output.join("client.ts")).unwrap());
    let mut value = fixture();
    value["components"]["schemas"]["LessonRequest"]["properties"]["title"]["maxLength"] = 79.into();
    fs::write(&source, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(matches!(
        generate(&source, &output, true),
        Err(ApiError::Stale)
    ));
    assert_eq!(initial, fs::read(output.join("client.ts")).unwrap());
    generate(&source, &output, false).unwrap();
    generate(&source, &output, true).unwrap();
    assert_ne!(initial, fs::read(output.join("client.ts")).unwrap());
}
#[test]
fn unsupported_wire_shapes_and_unknown_keywords_are_rejected() {
    for shape in [
        json!({"type":"number"}),
        json!({"type":"string"}),
        json!({"type":"string","maxLength":10,"format":"email"}),
        json!({"type":"integer","minimum":0,"maximum":9_007_199_254_740_992_i64}),
        json!({"type":"object","additionalProperties":true}),
        json!({"oneOf":[{"type":"string"},{"type":"boolean"}]}),
        json!({"$ref":"https://example.invalid/private-schema"}),
        json!({"$ref":"#/components/schemas/Missing"}),
        json!({"$ref":"#/components/schemas/LessonRequest"}),
        json!({"type":"array","items":{"type":"boolean"},"maxItems":129}),
    ] {
        let mut value = fixture();
        value["components"]["schemas"]["LessonRequest"]["properties"]["title"] = shape.clone();
        assert!(!accepts(value), "accepted {shape}");
    }
    let mut value = fixture();
    value["components"]["schemas"]["LessonRequest"]["properties"]["nickname"] =
        json!({"type":["string","null"],"maxLength":40});
    assert!(!accepts(value));
    let mut value = fixture();
    value["components"]["schemas"]["LessonRequest"]["required"] = json!(["title", "title"]);
    assert!(!accepts(value));
}
#[test]
fn operation_parameter_authentication_and_route_ambiguities_fail_closed() {
    for (pointer, replacement) in [
        ("/openapi", json!("3.0.3")),
        ("/security", json!([])),
        (
            "/paths/~1lessons~1{owner}/post/operationId",
            json!("x; injection"),
        ),
        (
            "/paths/~1lessons~1{owner}/post/parameters/0/required",
            json!(false),
        ),
        (
            "/paths/~1lessons~1{owner}/post/parameters/0/name",
            json!("other"),
        ),
        (
            "/paths/~1lessons~1{owner}/post/parameters/1/in",
            json!("header"),
        ),
        (
            "/paths/~1lessons~1{owner}/post/requestBody/required",
            json!(false),
        ),
    ] {
        let mut value = fixture();
        *value.pointer_mut(pointer).unwrap() = replacement;
        assert!(!accepts(value), "accepted {pointer}");
    }
    let mut value = fixture();
    value["paths"]["/lessons/{other}"] = value["paths"]["/lessons/{owner}"].clone();
    assert!(!accepts(value));
    let mut value = fixture();
    value["paths"]["/lessons/{owner}"]["get"]["operationId"] = "save_lesson".into();
    assert!(!accepts(value));
}
#[test]
fn source_budget_duplicate_keys_and_output_conflicts_preserve_files() {
    assert!(input::parse(br#"{"openapi":"3.0.3","openapi":"3.1.1"}"#).is_err());
    assert!(input::parse(&vec![b' '; MAX_SOURCE + 1]).is_err());
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    let source = root.join("schema.json");
    fs::write(&source, serde_json::to_vec(&fixture()).unwrap()).unwrap();
    let output = root.join("out");
    fs::create_dir(&output).unwrap();
    fs::write(output.join("client.ts"), "user-owned").unwrap();
    assert!(matches!(
        generate(&source, &output, false),
        Err(ApiError::Path)
    ));
    assert!(!output.join("contract.rs").exists());
    assert_eq!(
        fs::read_to_string(output.join("client.ts")).unwrap(),
        "user-owned"
    );
}
#[cfg(unix)]
#[test]
fn linked_inputs_and_outputs_are_not_followed() {
    use std::os::unix::fs::symlink;
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    let source = root.join("schema.json");
    fs::write(&source, serde_json::to_vec(&fixture()).unwrap()).unwrap();
    let linked = root.join("linked.json");
    symlink(&source, &linked).unwrap();
    assert!(generate(&linked, &root.join("out"), false).is_err());
    let target = root.join("target");
    fs::create_dir(&target).unwrap();
    let out = root.join("out");
    symlink(&target, &out).unwrap();
    assert!(generate(&source, &out, false).is_err());
    assert_eq!(fs::read_dir(target).unwrap().count(), 0);
}

#[test]
fn handwritten_profile_schema_is_not_treated_as_generated_output() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    let source = root.join("schema.json");
    let output = root.join("out");
    let original = serde_json::to_vec(&fixture()).unwrap();
    fs::write(&source, &original).unwrap();
    fs::create_dir(&output).unwrap();
    fs::write(output.join("openapi.json"), &original).unwrap();
    assert!(matches!(
        generate(&source, &output, false),
        Err(ApiError::Path)
    ));
    assert!(!output.join("contract.rs").exists());
    assert_eq!(fs::read(output.join("openapi.json")).unwrap(), original);
}

#[test]
fn generated_schema_receipt_is_reusable_but_never_authoritative() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    let source = root.join("schema.json");
    let output = root.join("out");
    fs::write(&source, serde_json::to_vec(&fixture()).unwrap()).unwrap();
    generate(&source, &output, false).unwrap();
    generate(&output.join("openapi.json"), &output, true).unwrap();
    let path = output.join("openapi.json");
    let mut value: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    value["components"]["schemas"]["LessonRequest"]["properties"]["title"]["format"] =
        "email".into();
    fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(generate(&path, &output, false).is_err());
}
