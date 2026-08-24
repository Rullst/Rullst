//! Explicit JSON-mode and provider-enforced JSON Schema request types.

use super::AiError;

/// A named JSON Schema sent to providers that support native structured output.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct StructuredOutputSchema {
    name: String,
    description: Option<String>,
    schema: serde_json::Value,
}

impl StructuredOutputSchema {
    /// Creates and validates a provider-safe schema descriptor.
    pub fn new(name: impl Into<String>, schema: serde_json::Value) -> Result<Self, AiError> {
        let name = name.into();
        if name.is_empty()
            || name.len() > 64
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        {
            return Err(AiError::InvalidSchema(
                "schema name must contain 1-64 ASCII letters, digits, '_' or '-'".to_string(),
            ));
        }
        if !schema.is_object() {
            return Err(AiError::InvalidSchema(
                "the JSON Schema root must be an object".to_string(),
            ));
        }

        Ok(Self {
            name,
            description: None,
            schema,
        })
    }

    /// Adds a provider-facing description for the output schema.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Returns the stable schema name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the optional provider-facing description.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Returns the JSON Schema document.
    pub fn schema(&self) -> &serde_json::Value {
        &self.schema
    }
}

pub(crate) fn clean_json_markdown(value: &str) -> &str {
    let trimmed = value.trim();
    let without_opening = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .unwrap_or(trimmed);
    without_opening
        .strip_suffix("```")
        .unwrap_or(without_opening)
        .trim()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_provider_safe_schema_names() {
        let schema = serde_json::json!({"type": "object"});
        assert!(StructuredOutputSchema::new("answer-v1", schema.clone()).is_ok());
        assert!(StructuredOutputSchema::new("", schema.clone()).is_err());
        assert!(StructuredOutputSchema::new("spaces are unsafe", schema).is_err());
        assert!(StructuredOutputSchema::new("answer", serde_json::json!([])).is_err());
    }

    #[test]
    fn strips_only_an_outer_markdown_fence() {
        assert_eq!(
            clean_json_markdown("```json\n{\"ok\":true}\n```"),
            "{\"ok\":true}"
        );
        assert_eq!(clean_json_markdown(" {\"ok\":true} "), "{\"ok\":true}");
    }
}
