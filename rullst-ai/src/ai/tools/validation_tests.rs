#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use crate::ai::tools::ToolRisk;

#[derive(Clone)]
struct SchemaTool {
    name: String,
    description: String,
    parameters: Vec<ToolParam>,
}

impl SchemaTool {
    fn new(parameters: Vec<ToolParam>) -> Self {
        Self {
            name: "schema-tool".to_string(),
            description: "A bounded schema tool".to_string(),
            parameters,
        }
    }
}

impl AiTool for SchemaTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters(&self) -> Vec<ToolParam> {
        self.parameters.clone()
    }

    fn risk(&self) -> ToolRisk {
        ToolRisk::ReadOnly
    }

    fn execute(&self, payload: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        Ok(payload)
    }
}

fn parameter(name: &str, param_type: &str, required: bool) -> ToolParam {
    ToolParam {
        name: name.to_string(),
        param_type: param_type.to_string(),
        description: format!("The {name} value"),
        required,
    }
}

#[test]
fn tool_schema_rejects_invalid_identifiers_descriptions_types_and_duplicates() {
    for name in ["", "contains space", &"x".repeat(MAX_TOOL_NAME_BYTES + 1)] {
        let mut tool = SchemaTool::new(Vec::new());
        tool.name = name.to_string();
        assert!(matches!(
            validate_tool(&tool),
            Err(ToolExecutionError::InvalidPolicy(_))
        ));
    }

    for description in [
        "   ".to_string(),
        "x".repeat(MAX_TOOL_DESCRIPTION_BYTES + 1),
    ] {
        let mut tool = SchemaTool::new(Vec::new());
        tool.description = description;
        assert!(matches!(
            validate_tool(&tool),
            Err(ToolExecutionError::InvalidPolicy(_))
        ));
    }

    let too_many = (0..=MAX_TOOL_PARAMETERS)
        .map(|index| parameter(&format!("p{index}"), "string", false))
        .collect();
    assert!(validate_tool(&SchemaTool::new(too_many)).is_err());

    let invalid_name = SchemaTool::new(vec![parameter("bad.name", "string", false)]);
    assert!(validate_tool(&invalid_name).is_err());

    for description in ["".to_string(), "x".repeat(MAX_PARAM_DESCRIPTION_BYTES + 1)] {
        let mut param = parameter("value", "string", false);
        param.description = description;
        assert!(validate_tool(&SchemaTool::new(vec![param])).is_err());
    }

    assert!(validate_tool(&SchemaTool::new(vec![parameter("value", "binary", false)])).is_err());
    assert!(
        validate_tool(&SchemaTool::new(vec![
            parameter("value", "string", false),
            parameter("value", "number", false),
        ]))
        .is_err()
    );
}

#[test]
fn every_supported_json_type_accepts_its_value_and_rejects_wrong_values() {
    let cases = [
        ("string", serde_json::json!("value")),
        ("number", serde_json::json!(1.5)),
        ("integer", serde_json::json!(7)),
        ("boolean", serde_json::json!(true)),
        ("object", serde_json::json!({"nested": true})),
        ("array", serde_json::json!([1, 2])),
        ("null", Value::Null),
    ];

    for (param_type, value) in cases {
        let parameters = vec![parameter("value", param_type, true)];
        validate_tool(&SchemaTool::new(parameters.clone())).expect("supported JSON type");
        validate_payload(
            "schema-tool",
            &serde_json::json!({"value": value}),
            &parameters,
        )
        .expect("matching value");
        assert!(
            validate_payload(
                "schema-tool",
                &serde_json::json!({"value": "wrong"}),
                &parameters,
            )
            .is_err()
                || param_type == "string"
        );
    }

    let integer = vec![parameter("value", "integer", true)];
    assert!(validate_payload("schema-tool", &serde_json::json!({"value": 1.5}), &integer).is_err());
    assert!(matches_json_type(&serde_json::json!(u64::MAX), "integer"));
    assert!(!matches_json_type(&Value::Null, "unsupported"));
}

#[test]
fn payload_contract_rejects_non_objects_unknown_missing_and_mistyped_fields() {
    let parameters = vec![
        parameter("required", "boolean", true),
        parameter("optional", "array", false),
    ];
    assert!(validate_payload("schema-tool", &Value::Null, &parameters).is_err());
    assert!(
        validate_payload("schema-tool", &serde_json::json!({"extra": 1}), &parameters).is_err()
    );
    assert!(validate_payload("schema-tool", &serde_json::json!({}), &parameters).is_err());
    assert!(
        validate_payload(
            "schema-tool",
            &serde_json::json!({"required": "true"}),
            &parameters,
        )
        .is_err()
    );
    validate_payload(
        "schema-tool",
        &serde_json::json!({"required": true}),
        &parameters,
    )
    .expect("optional field may be absent");
    assert!(serialized_size(&serde_json::json!({"ok": true}), "schema-tool").unwrap() > 0);
}
