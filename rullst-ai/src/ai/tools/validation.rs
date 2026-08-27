use super::{AiTool, ToolExecutionError, ToolParam};
use serde_json::Value;
use std::collections::BTreeSet;

const MAX_TOOL_NAME_BYTES: usize = 64;
const MAX_TOOL_DESCRIPTION_BYTES: usize = 4 * 1024;
const MAX_PARAM_DESCRIPTION_BYTES: usize = 2 * 1024;
const MAX_TOOL_PARAMETERS: usize = 64;

pub(super) fn validate_tool(tool: &impl AiTool) -> Result<(), ToolExecutionError> {
    validate_identifier("tool name", tool.name())?;
    validate_non_empty_bounded(
        "tool description",
        tool.description(),
        MAX_TOOL_DESCRIPTION_BYTES,
    )?;
    let parameters = tool.parameters();
    if parameters.len() > MAX_TOOL_PARAMETERS {
        return Err(ToolExecutionError::InvalidPolicy(format!(
            "tool '{}' has more than {MAX_TOOL_PARAMETERS} parameters",
            tool.name()
        )));
    }
    let mut names = BTreeSet::new();
    for param in parameters {
        validate_identifier("tool parameter", &param.name)?;
        validate_non_empty_bounded(
            "tool parameter description",
            &param.description,
            MAX_PARAM_DESCRIPTION_BYTES,
        )?;
        if !is_supported_json_type(&param.param_type) {
            return Err(ToolExecutionError::InvalidPolicy(format!(
                "tool '{}' parameter '{}' has unsupported JSON type '{}'",
                tool.name(),
                param.name,
                param.param_type
            )));
        }
        if !names.insert(param.name.clone()) {
            return Err(ToolExecutionError::InvalidPolicy(format!(
                "tool '{}' declares duplicate parameter '{}'",
                tool.name(),
                param.name
            )));
        }
    }
    Ok(())
}

pub(super) fn validate_payload(
    tool: &str,
    payload: &Value,
    parameters: &[ToolParam],
) -> Result<(), ToolExecutionError> {
    let object = payload
        .as_object()
        .ok_or_else(|| ToolExecutionError::InvalidPayload {
            tool: tool.to_string(),
            reason: "payload must be a JSON object".to_string(),
        })?;
    for key in object.keys() {
        if !parameters.iter().any(|param| param.name == *key) {
            return Err(ToolExecutionError::InvalidPayload {
                tool: tool.to_string(),
                reason: format!("unknown property '{key}'"),
            });
        }
    }
    for param in parameters {
        match object.get(&param.name) {
            None if param.required => {
                return Err(ToolExecutionError::InvalidPayload {
                    tool: tool.to_string(),
                    reason: format!("missing required property '{}'", param.name),
                });
            }
            Some(value) if !matches_json_type(value, &param.param_type) => {
                return Err(ToolExecutionError::InvalidPayload {
                    tool: tool.to_string(),
                    reason: format!(
                        "property '{}' must have JSON type '{}'",
                        param.name, param.param_type
                    ),
                });
            }
            _ => {}
        }
    }
    Ok(())
}

pub(super) fn validate_identifier(label: &str, value: &str) -> Result<(), ToolExecutionError> {
    if value.is_empty()
        || value.len() > MAX_TOOL_NAME_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return Err(ToolExecutionError::InvalidPolicy(format!(
            "{label} must be 1-{MAX_TOOL_NAME_BYTES} ASCII letters, digits, '_' or '-'"
        )));
    }
    Ok(())
}

pub(super) fn validate_non_empty_bounded(
    label: &str,
    value: &str,
    max_bytes: usize,
) -> Result<(), ToolExecutionError> {
    if value.trim().is_empty() || value.len() > max_bytes {
        return Err(ToolExecutionError::InvalidPolicy(format!(
            "{label} must be non-empty and at most {max_bytes} bytes"
        )));
    }
    Ok(())
}

pub(super) fn serialized_size(value: &Value, tool: &str) -> Result<usize, ToolExecutionError> {
    serde_json::to_vec(value)
        .map(|bytes| bytes.len())
        .map_err(|_| ToolExecutionError::InvalidPayload {
            tool: tool.to_string(),
            reason: "payload could not be serialized".to_string(),
        })
}

fn is_supported_json_type(param_type: &str) -> bool {
    matches!(
        param_type,
        "string" | "number" | "integer" | "boolean" | "object" | "array" | "null"
    )
}

fn matches_json_type(value: &Value, param_type: &str) -> bool {
    match param_type {
        "string" => value.is_string(),
        "number" => value.is_number(),
        "integer" => value.as_i64().is_some() || value.as_u64().is_some(),
        "boolean" => value.is_boolean(),
        "object" => value.is_object(),
        "array" => value.is_array(),
        "null" => value.is_null(),
        _ => false,
    }
}
