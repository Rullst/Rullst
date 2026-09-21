use rullst_security::schema_guard::JsonSchemaPolicy;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub const MAX_WIRE_BYTES: usize = 64 * 1024;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ContractError { Configuration, Payload, Parameters }
impl std::fmt::Display for ContractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Configuration => "API contract configuration rejected",
            Self::Payload => "API payload rejected",
            Self::Parameters => "API parameters rejected",
        })
    }
}
impl std::error::Error for ContractError {}

/// Compile once at startup. Authentication and ownership remain application-owned.
#[derive(Clone)]
pub struct Contract { policies: BTreeMap<String, JsonSchemaPolicy> }
impl Contract {
    fn policy(&self, name: &str) -> Result<&JsonSchemaPolicy, ContractError> {
        self.policies.get(name).ok_or(ContractError::Configuration)
    }
    fn decode<T: DeserializeOwned>(&self, name: &str, bytes: &[u8]) -> Result<T, ContractError> {
        rullst_security::schema_guard::inspect_json_payload(bytes, 24, MAX_WIRE_BYTES).map_err(|_| ContractError::Payload)?;
        let canonical = canonical_numbers(bytes)?;
        let value: Value = serde_json::from_slice(&canonical).map_err(|_| ContractError::Payload)?;
        wire_nodes(&value, &mut 0)?;
        self.policy(name)?.validate(&value).map_err(|_| ContractError::Payload)?;
        serde_json::from_value(value).map_err(|_| ContractError::Payload)
    }
    fn encode<T: Serialize>(&self, name: &str, value: &T) -> Result<Vec<u8>, ContractError> {
        let value = serde_json::to_value(value).map_err(|_| ContractError::Payload)?;
        wire_nodes(&value, &mut 0)?;
        self.policy(name)?.validate(&value).map_err(|_| ContractError::Payload)?;
        let bytes = serde_json::to_vec(&value).map_err(|_| ContractError::Payload)?;
        if bytes.len() > MAX_WIRE_BYTES { return Err(ContractError::Payload); }
        Ok(bytes)
    }
    fn parameters<T: DeserializeOwned>(&self, name: &str, definitions: &[(&str, &str, &str)], path: &[(&str, &str)], query: &[(&str, &str)]) -> Result<T, ContractError> {
        if path.len() + query.len() > 16 { return Err(ContractError::Parameters); }
        let mut object = serde_json::Map::new();
        for (prefix, values) in [("p", path), ("q", query)] {
            for (key, raw) in values {
                if raw.len() > MAX_WIRE_BYTES { return Err(ContractError::Parameters); }
                let kind = definitions.iter().find(|(p,k,_)| *p == prefix && k == key).map(|(_,_,t)| *t).ok_or(ContractError::Parameters)?;
                let value = match kind {
                    "string" => {
                        if prefix == "p" && (raw.is_empty() || matches!(*raw, "." | "..")) { return Err(ContractError::Parameters); }
                        Value::String((*raw).into())
                    },
                    "integer" => {
                        let integer = raw.parse::<i64>().map_err(|_| ContractError::Parameters)?;
                        if integer.to_string() != *raw { return Err(ContractError::Parameters); }
                        integer.into()
                    }
                    "boolean" => match *raw { "true" => true.into(), "false" => false.into(), _ => return Err(ContractError::Parameters) },
                    _ => return Err(ContractError::Configuration),
                };
                if object.insert(format!("{prefix}_{key}"), value).is_some() { return Err(ContractError::Parameters); }
            }
        }
        let value = Value::Object(object);
        self.policy(name)?.validate(&value).map_err(|_| ContractError::Parameters)?;
        serde_json::from_value(value).map_err(|_| ContractError::Parameters)
    }
}

fn wire_nodes(value: &Value, count: &mut usize) -> Result<(), ContractError> {
    *count += 1;
    if *count > 8192 { return Err(ContractError::Payload); }
    match value {
        Value::Array(items) => for item in items { wire_nodes(item, count)?; },
        Value::Object(items) => for item in items.values() { wire_nodes(item, count)?; },
        _ => {}
    }
    Ok(())
}
