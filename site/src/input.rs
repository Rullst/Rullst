//! Downloaded JSON inputs: small regular files in the working directory, by fixed name.

use std::fs;
use std::path::Path;

use serde_json::Value;

/// A downloaded input is at most 1 MiB.
pub const LIMIT: u64 = 1024 * 1024;

/// Parses a downloaded file; it must be a regular file of at most 1 MiB inside the working directory.
pub fn read_json(path: &Path) -> Result<Value, String> {
    let base = std::env::current_dir()
        .and_then(fs::canonicalize)
        .map_err(|error| format!("the working directory cannot be read: {error}"))?;
    let resolved =
        fs::canonicalize(path).map_err(|error| format!("{}: {error}", path.display()))?;
    if !resolved.starts_with(&base) {
        return Err("the file must be inside the working directory".to_string());
    }
    let metadata =
        fs::metadata(&resolved).map_err(|error| format!("{}: {error}", path.display()))?;
    if !metadata.is_file() || metadata.len() > LIMIT {
        return Err("the file must be a regular file of at most 1 MiB".to_string());
    }
    let text =
        fs::read_to_string(&resolved).map_err(|error| format!("{}: {error}", path.display()))?;
    serde_json::from_str(&text).map_err(|error| format!("{} is not JSON: {error}", path.display()))
}

/// Python's truth value of a JSON value: null, false, 0, "", [] and {} are false.
pub fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(flag) => *flag,
        Value::Number(number) => number.as_f64().is_some_and(|n| n != 0.0),
        Value::String(text) => !text.is_empty(),
        Value::Array(items) => !items.is_empty(),
        Value::Object(map) => !map.is_empty(),
    }
}

/// A JSON value as Python's `repr` would show it in a warning (close enough for a log).
pub fn show(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => "None".to_string(),
        Some(Value::String(text)) => format!("'{text}'"),
        Some(other) => other.to_string(),
    }
}
