use super::ApiError;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};

pub(super) const SAFE_INTEGER: i64 = 9_007_199_254_740_991;
pub(super) fn object(value: &Value) -> Result<&Map<String, Value>, ApiError> {
    value.as_object().ok_or(ApiError::Invalid)
}
pub(super) fn keys(value: &Value, allowed: &[&str]) -> Result<(), ApiError> {
    if object(value)?
        .keys()
        .any(|key| !allowed.contains(&key.as_str()))
    {
        return Err(ApiError::Unsupported);
    }
    Ok(())
}
pub(super) fn string(value: &Value) -> Result<&str, ApiError> {
    value.as_str().ok_or(ApiError::Invalid)
}
pub(super) fn field(name: &str) -> bool {
    name.len() <= 40
        && name.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        && !matches!(
            name,
            "self" | "super" | "crate" | "constructor" | "prototype" | "__proto__"
        )
}
pub(super) fn component(name: &str) -> bool {
    name.len() <= 64
        && name.as_bytes().first().is_some_and(u8::is_ascii_uppercase)
        && name.bytes().all(|b| b.is_ascii_alphanumeric())
}
pub(super) fn reference(value: &Value) -> Result<&str, ApiError> {
    keys(value, &["$ref"])?;
    let name = string(&value["$ref"])?
        .strip_prefix("#/components/schemas/")
        .ok_or(ApiError::Unsupported)?;
    if !component(name) {
        return Err(ApiError::Invalid);
    }
    Ok(name)
}
pub(super) fn kind(value: &Value) -> Result<(&str, bool), ApiError> {
    match &value["type"] {
        Value::String(name) => Ok((name, false)),
        Value::Array(types) if types.len() == 2 => {
            let a = string(&types[0])?;
            let b = string(&types[1])?;
            match (a, b) {
                ("null", t) | (t, "null") if t != "null" => Ok((t, true)),
                _ => Err(ApiError::Unsupported),
            }
        }
        _ => Err(ApiError::Unsupported),
    }
}
fn maximum(value: &Value, min_key: &str, max_key: &str, limit: u64) -> Result<(), ApiError> {
    let max = value[max_key].as_u64().ok_or(ApiError::Invalid)?;
    let min = value
        .get(min_key)
        .map(|n| n.as_u64().ok_or(ApiError::Invalid))
        .transpose()?
        .unwrap_or(0);
    if max > limit || min > max {
        return Err(ApiError::Limit);
    }
    Ok(())
}
pub(super) fn node(
    value: &Value,
    schemas: &BTreeMap<String, Value>,
    visiting: &mut BTreeSet<String>,
    depth: usize,
) -> Result<(), ApiError> {
    if depth > 8 {
        return Err(ApiError::Limit);
    }
    if value.get("$ref").is_some() {
        let name = reference(value)?;
        let target = schemas.get(name).ok_or(ApiError::Invalid)?;
        if !visiting.insert(name.into()) {
            return Err(ApiError::Unsupported);
        }
        node(target, schemas, visiting, depth + 1)?;
        visiting.remove(name);
        return Ok(());
    }
    let (type_name, nullable) = kind(value)?;
    match type_name {
        "string" => {
            keys(value, &["type", "minLength", "maxLength"])?;
            maximum(value, "minLength", "maxLength", 4096)?;
        }
        "integer" => {
            keys(value, &["type", "minimum", "maximum"])?;
            let min = value["minimum"].as_i64().ok_or(ApiError::Invalid)?;
            let max = value["maximum"].as_i64().ok_or(ApiError::Invalid)?;
            if min < -SAFE_INTEGER || max > SAFE_INTEGER || min > max {
                return Err(ApiError::Limit);
            }
        }
        "boolean" => keys(value, &["type"])?,
        "array" => {
            keys(value, &["type", "items", "minItems", "maxItems"])?;
            maximum(value, "minItems", "maxItems", 128)?;
            // Inline object types have no stable generated type identity.
            if value["items"].get("$ref").is_none() && kind_of(&value["items"]) == Some("object") {
                return Err(ApiError::Unsupported);
            }
            node(&value["items"], schemas, visiting, depth + 1)?;
        }
        "object" if !nullable => {
            keys(
                value,
                &["type", "properties", "required", "additionalProperties"],
            )?;
            if value["additionalProperties"] != false {
                return Err(ApiError::Unsupported);
            }
            let properties = object(&value["properties"])?;
            if properties.is_empty() || properties.len() > 32 {
                return Err(ApiError::Limit);
            }
            let required = value["required"].as_array().ok_or(ApiError::Invalid)?;
            let names = required
                .iter()
                .map(string)
                .collect::<Result<BTreeSet<_>, _>>()?;
            if names.len() != required.len()
                || names.iter().any(|name| !properties.contains_key(*name))
            {
                return Err(ApiError::Invalid);
            }
            for (name, child) in properties {
                if !field(name) || kind_of(child) == Some("object") {
                    return Err(ApiError::Unsupported);
                }
                if !names.contains(name.as_str()) && kind(child).is_ok_and(|(_, nullable)| nullable)
                {
                    return Err(ApiError::Unsupported);
                }
                node(child, schemas, visiting, depth + 1)?;
            }
        }
        _ => return Err(ApiError::Unsupported),
    }
    Ok(())
}
fn kind_of(value: &Value) -> Option<&str> {
    kind(value).ok().map(|(name, _)| name)
}
pub(super) fn required(schema: &Value, name: &str) -> bool {
    schema["required"]
        .as_array()
        .is_some_and(|fields| fields.iter().any(|field| field == name))
}
pub(super) fn rust_type(value: &Value) -> Result<String, ApiError> {
    if value.get("$ref").is_some() {
        return Ok(format!("Dto{}", reference(value)?));
    }
    let (name, nullable) = kind(value)?;
    let ty = match name {
        "string" => "std::string::String".into(),
        "integer" => "i64".into(),
        "boolean" => "bool".into(),
        "array" => format!("std::vec::Vec<{}>", rust_type(&value["items"])?),
        _ => return Err(ApiError::Unsupported),
    };
    Ok(if nullable {
        format!("std::option::Option<{ty}>")
    } else {
        ty
    })
}
pub(super) fn ts_type(value: &Value) -> Result<String, ApiError> {
    if value.get("$ref").is_some() {
        return Ok(format!("Dto{}", reference(value)?));
    }
    let (name, nullable) = kind(value)?;
    let ty = match name {
        "string" => "string".into(),
        "integer" => "number".into(),
        "boolean" => "boolean".into(),
        "array" => format!("Array<{}>", ts_type(&value["items"])?),
        _ => return Err(ApiError::Unsupported),
    };
    Ok(if nullable { format!("{ty} | null") } else { ty })
}
