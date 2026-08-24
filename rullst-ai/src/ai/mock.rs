//! Deterministic offline fixtures shared by built-in providers.

use super::{AiError, Message, StructuredOutputSchema};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProviderMode {
    Live,
    Mock,
}

impl ProviderMode {
    pub(crate) fn from_credential(credential: &str) -> Self {
        let credential = credential.trim();
        if credential.is_empty() || credential.starts_with("mock_") {
            Self::Mock
        } else {
            Self::Live
        }
    }

    pub(crate) const fn is_mock(self) -> bool {
        matches!(self, Self::Mock)
    }
}

pub(crate) fn chat_response(provider: &str, model: &str, messages: &[Message]) -> String {
    let last_user = messages
        .iter()
        .rev()
        .find(|message| message.role == "user")
        .map_or("Hello", |message| message.content.as_str());
    format!("Mock response from {provider} (model: {model}): Echo '{last_user}'")
}

pub(crate) fn vision_response(
    provider: &str,
    model: &str,
    text: &str,
    image_bytes: &[u8],
) -> String {
    format!(
        "Mock vision response from {provider} (model: {model}, image_bytes: {}): Echo '{text}'",
        image_bytes.len()
    )
}

pub(crate) fn json_response(provider: &str, model: &str, text: &str) -> String {
    serde_json::json!({
        "mock": true,
        "provider": provider,
        "model": model,
        "prompt": text,
    })
    .to_string()
}

pub(crate) fn embedding(text: &str) -> Vec<f32> {
    const DIMENSIONS: usize = 16;
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

    let mut state = FNV_OFFSET;
    for byte in text.as_bytes() {
        state ^= u64::from(*byte);
        state = state.wrapping_mul(FNV_PRIME);
    }

    (0..DIMENSIONS)
        .map(|index| {
            state ^= (index as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
            state = state.rotate_left(13).wrapping_mul(FNV_PRIME);
            let sample = ((state >> 40) & 0x00ff_ffff) as f32 / 0x00ff_ffff as f32;
            sample.mul_add(2.0, -1.0)
        })
        .collect()
}

pub(crate) fn structured_response(schema: &StructuredOutputSchema) -> Result<String, AiError> {
    let value = value_for_schema(schema.schema(), 0)?;
    serde_json::to_string(&value).map_err(AiError::from)
}

fn value_for_schema(
    schema: &serde_json::Value,
    depth: usize,
) -> Result<serde_json::Value, AiError> {
    if depth > 16 {
        return Err(AiError::InvalidSchema(
            "mock schema nesting exceeds 16 levels".to_string(),
        ));
    }
    if let Some(value) = schema.get("const") {
        return Ok(value.clone());
    }
    if let Some(value) = schema.get("default") {
        return Ok(value.clone());
    }
    if let Some(value) = schema
        .get("enum")
        .and_then(serde_json::Value::as_array)
        .and_then(|values| values.first())
    {
        return Ok(value.clone());
    }
    if let Some(branch) = schema
        .get("anyOf")
        .or_else(|| schema.get("oneOf"))
        .and_then(serde_json::Value::as_array)
        .and_then(|branches| branches.first())
    {
        return value_for_schema(branch, depth + 1);
    }

    let schema_type = schema
        .get("type")
        .and_then(|value| match value {
            serde_json::Value::String(value) => Some(value.as_str()),
            serde_json::Value::Array(values) => values.iter().find_map(|value| {
                value
                    .as_str()
                    .and_then(|value| (value != "null").then_some(value))
            }),
            _ => None,
        })
        .ok_or_else(|| {
            AiError::InvalidSchema("mock schemas require an explicit supported type".to_string())
        })?;

    match schema_type {
        "object" => {
            let properties = schema
                .get("properties")
                .and_then(serde_json::Value::as_object)
                .ok_or_else(|| {
                    AiError::InvalidSchema(
                        "mock object schemas require a properties object".to_string(),
                    )
                })?;
            let mut object = serde_json::Map::new();
            for (name, property_schema) in properties {
                object.insert(name.clone(), value_for_schema(property_schema, depth + 1)?);
            }
            Ok(serde_json::Value::Object(object))
        }
        "array" => {
            let item_schema = schema.get("items").ok_or_else(|| {
                AiError::InvalidSchema("mock array schemas require items".to_string())
            })?;
            let item_count = schema
                .get("minItems")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0)
                .min(32) as usize;
            let mut items = Vec::with_capacity(item_count);
            for _ in 0..item_count {
                items.push(value_for_schema(item_schema, depth + 1)?);
            }
            Ok(serde_json::Value::Array(items))
        }
        "string" => Ok(serde_json::Value::String("mock_string".to_string())),
        "integer" => Ok(serde_json::json!(0)),
        "number" => Ok(serde_json::json!(0.0)),
        "boolean" => Ok(serde_json::json!(false)),
        "null" => Ok(serde_json::Value::Null),
        unsupported => Err(AiError::InvalidSchema(format!(
            "mock schema type '{unsupported}' is unsupported"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embeddings_are_stable_and_input_sensitive() {
        assert_eq!(embedding("same"), embedding("same"));
        assert_ne!(embedding("same"), embedding("different"));
        assert_eq!(embedding("same").len(), 16);
    }

    #[test]
    fn builds_deterministic_schema_fixture() {
        let schema = StructuredOutputSchema::new(
            "answer",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "ok": {"type": "boolean"},
                    "items": {"type": "array", "items": {"type": "string"}, "minItems": 1}
                },
                "required": ["ok", "items"],
                "additionalProperties": false
            }),
        )
        .expect("valid test schema");
        let response = structured_response(&schema).expect("fixture should be generated");
        assert_eq!(response, r#"{"items":["mock_string"],"ok":false}"#);
    }
}
