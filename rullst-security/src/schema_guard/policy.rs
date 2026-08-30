//! Compiled, offline-only JSON Schema policy for route-scoped payloads.

use crate::telemetry::SecurityStore;
use serde_json::{Value, json};
use std::sync::Arc;

/// Maximum serialized schema or OpenAPI document accepted by the policy.
pub const MAX_SCHEMA_BYTES: usize = 2 * 1024 * 1024;
/// Maximum JSON containers and scalar nodes inspected during construction.
pub const MAX_SCHEMA_NODES: usize = 50_000;
/// Maximum schema/OpenAPI nesting depth accepted during construction.
pub const MAX_SCHEMA_DEPTH: usize = 64;

/// Configuration or payload failure raised by a compiled schema policy.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum SchemaPolicyError {
    /// The serialized schema/document exceeds the bounded configuration size.
    #[error("schema document exceeds the {MAX_SCHEMA_BYTES}-byte limit")]
    TooLarge,
    /// The schema/document has more nodes than the construction budget.
    #[error("schema document exceeds the {MAX_SCHEMA_NODES}-node limit")]
    TooManyNodes,
    /// The schema/document exceeds the construction nesting budget.
    #[error("schema document exceeds the {MAX_SCHEMA_DEPTH}-level depth limit")]
    TooDeep,
    /// A reference would require filesystem, network, or another external resolver.
    #[error("schema references must be local JSON fragments")]
    ExternalReference,
    /// The JSON Schema document is not valid Draft 2020-12 configuration.
    #[error("invalid JSON Schema 2020-12 document: {0}")]
    InvalidSchema(String),
    /// The OpenAPI document is outside the supported 3.1 component boundary.
    #[error("invalid OpenAPI 3.1 component contract: {0}")]
    InvalidOpenApi(String),
    /// The runtime instance does not match the compiled schema.
    #[error("JSON payload does not match the configured schema")]
    PayloadRejected,
}

/// Reusable JSON Schema 2020-12 validator with no external reference resolver.
///
/// Construction is bounded and application-owned. The resulting policy can be
/// cloned cheaply and mounted around only the Axum routes that share its input
/// contract. Validation errors never include the rejected payload value.
#[derive(Clone)]
pub struct JsonSchemaPolicy {
    validator: Arc<jsonschema::Validator>,
}

impl std::fmt::Debug for JsonSchemaPolicy {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("JsonSchemaPolicy")
            .field("external_resolution", &"disabled")
            .field("draft", &"2020-12")
            .finish()
    }
}

impl JsonSchemaPolicy {
    /// Compiles one bounded JSON Schema 2020-12 document.
    pub fn from_schema(schema: Value) -> Result<Self, SchemaPolicyError> {
        inspect_schema_document(&schema)?;
        jsonschema::draft202012::meta::validate(&schema)
            .map_err(|error| SchemaPolicyError::InvalidSchema(error.to_string()))?;
        Self::compile(&schema)
    }

    /// Compiles one `components.schemas` entry from an OpenAPI 3.1 document.
    ///
    /// OpenAPI 3.0 schema objects are deliberately rejected because their
    /// dialect is not JSON Schema 2020-12. Component names use a narrow token
    /// boundary and every reference in the supplied document must stay local.
    pub fn from_openapi_component(
        openapi: &Value,
        component: &str,
    ) -> Result<Self, SchemaPolicyError> {
        inspect_schema_document(openapi)?;
        let version = openapi
            .get("openapi")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                SchemaPolicyError::InvalidOpenApi("missing `openapi` version".to_string())
            })?;
        if !version.starts_with("3.1.") {
            return Err(SchemaPolicyError::InvalidOpenApi(
                "only OpenAPI 3.1 component schemas are supported".to_string(),
            ));
        }
        if component.is_empty()
            || component.len() > 80
            || !component
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
        {
            return Err(SchemaPolicyError::InvalidOpenApi(
                "component name must be a bounded ASCII token".to_string(),
            ));
        }
        let schemas = openapi
            .pointer("/components/schemas")
            .and_then(Value::as_object)
            .ok_or_else(|| {
                SchemaPolicyError::InvalidOpenApi(
                    "document has no `components.schemas` object".to_string(),
                )
            })?;
        if !schemas.contains_key(component) {
            return Err(SchemaPolicyError::InvalidOpenApi(
                "requested component is not present".to_string(),
            ));
        }
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "components": { "schemas": schemas },
            "$ref": format!("#/components/schemas/{component}")
        });
        Self::from_schema(schema)
    }

    /// Validates one already-parsed JSON instance without exposing its values
    /// through the returned error.
    pub fn validate(&self, instance: &Value) -> Result<(), SchemaPolicyError> {
        self.validator.validate(instance).map_err(|_| {
            SecurityStore::global().inc_schema_violations();
            SchemaPolicyError::PayloadRejected
        })
    }

    /// Returns whether an instance matches, incrementing security telemetry on
    /// rejection just like [`Self::validate`].
    pub fn is_valid(&self, instance: &Value) -> bool {
        self.validate(instance).is_ok()
    }

    fn compile(schema: &Value) -> Result<Self, SchemaPolicyError> {
        let validator = jsonschema::draft202012::options()
            .with_pattern_options(jsonschema::PatternOptions::regex())
            .should_validate_formats(true)
            .build(schema)
            .map_err(|error| SchemaPolicyError::InvalidSchema(error.to_string()))?;
        Ok(Self {
            validator: Arc::new(validator),
        })
    }
}

fn inspect_schema_document(document: &Value) -> Result<(), SchemaPolicyError> {
    let serialized = serde_json::to_vec(document)
        .map_err(|error| SchemaPolicyError::InvalidSchema(error.to_string()))?;
    if serialized.len() > MAX_SCHEMA_BYTES {
        return Err(SchemaPolicyError::TooLarge);
    }

    let mut nodes = 0usize;
    let mut stack = vec![(document, 1usize)];
    while let Some((value, depth)) = stack.pop() {
        nodes = nodes
            .checked_add(1)
            .ok_or(SchemaPolicyError::TooManyNodes)?;
        if nodes > MAX_SCHEMA_NODES {
            return Err(SchemaPolicyError::TooManyNodes);
        }
        if depth > MAX_SCHEMA_DEPTH {
            return Err(SchemaPolicyError::TooDeep);
        }
        match value {
            Value::Object(object) => {
                for keyword in ["$ref", "$dynamicRef"] {
                    if let Some(reference) = object.get(keyword).and_then(Value::as_str)
                        && !reference.starts_with('#')
                    {
                        return Err(SchemaPolicyError::ExternalReference);
                    }
                }
                stack.extend(object.values().map(|child| (child, depth + 1)));
            }
            Value::Array(array) => {
                stack.extend(array.iter().map(|child| (child, depth + 1)));
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // TM-SEC-16: configured schema enforcement is closed to external refs and shape confusion.
    fn json_schema_policy_rejects_unexpected_shapes_and_external_refs() {
        let policy = JsonSchemaPolicy::from_schema(json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {
                "email": {"type": "string", "format": "email"},
                "count": {"type": "integer", "minimum": 1}
            },
            "required": ["email", "count"],
            "additionalProperties": false
        }))
        .expect("bounded schema");
        assert!(policy.is_valid(&json!({"email": "a@example.com", "count": 1})));
        assert!(!policy.is_valid(&json!({"email": "bad", "count": 0})));
        assert!(!policy.is_valid(&json!({
            "email": "a@example.com",
            "count": 1,
            "role": "admin"
        })));

        assert_eq!(
            JsonSchemaPolicy::from_schema(json!({"$ref": "https://attacker.invalid/a"}))
                .expect_err("external reference must fail"),
            SchemaPolicyError::ExternalReference
        );
        assert!(matches!(
            JsonSchemaPolicy::from_schema(json!({"type": "invalid"})),
            Err(SchemaPolicyError::InvalidSchema(_))
        ));
    }

    #[test]
    fn openapi_31_component_keeps_local_component_references() {
        let document = json!({
            "openapi": "3.1.1",
            "components": {"schemas": {
                "Address": {
                    "type": "object",
                    "properties": {"city": {"type": "string"}},
                    "required": ["city"]
                },
                "User": {
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"},
                        "address": {"$ref": "#/components/schemas/Address"}
                    },
                    "required": ["name", "address"]
                }
            }}
        });
        let policy = JsonSchemaPolicy::from_openapi_component(&document, "User")
            .expect("OpenAPI component policy");
        assert!(policy.is_valid(&json!({"name": "Ada", "address": {"city": "London"}})));
        assert!(!policy.is_valid(&json!({"name": "Ada", "address": {}})));
        assert!(matches!(
            JsonSchemaPolicy::from_openapi_component(&document, "Missing"),
            Err(SchemaPolicyError::InvalidOpenApi(_))
        ));
        let mut v3 = document;
        v3["openapi"] = json!("3.0.4");
        assert!(matches!(
            JsonSchemaPolicy::from_openapi_component(&v3, "User"),
            Err(SchemaPolicyError::InvalidOpenApi(_))
        ));
    }

    #[test]
    fn schema_construction_is_bounded_and_uses_linear_regexes() {
        let mut deep = json!(true);
        for _ in 0..=MAX_SCHEMA_DEPTH {
            deep = json!({"allOf": [deep]});
        }
        assert_eq!(
            JsonSchemaPolicy::from_schema(deep).expect_err("deep schema"),
            SchemaPolicyError::TooDeep
        );
        assert!(
            JsonSchemaPolicy::from_schema(json!({
                "type": "string",
                "pattern": "^(a+)+$"
            }))
            .is_ok()
        );
        assert!(matches!(
            JsonSchemaPolicy::from_schema(json!({
                "type": "string",
                "pattern": "(?<=unsafe)lookbehind"
            })),
            Err(SchemaPolicyError::InvalidSchema(_))
        ));
    }
}
