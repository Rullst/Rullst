use super::{
    ApiError,
    schema::{self, keys, object, string},
};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Serialize)]
pub(super) struct Parameter {
    pub name: String,
    pub location: String,
    pub required: bool,
    pub schema: Value,
}
#[derive(Serialize)]
pub(super) struct Operation {
    pub id: String,
    pub method: String,
    pub path: String,
    pub parameters: Vec<Parameter>,
    pub body: Option<String>,
    pub responses: BTreeMap<u16, String>,
}
pub(super) struct Document {
    pub source: Value,
    pub schemas: BTreeMap<String, Value>,
    pub operations: Vec<Operation>,
}
fn body_reference(content: &Value, schemas: &BTreeMap<String, Value>) -> Result<String, ApiError> {
    keys(content, &["application/json"])?;
    let media = &content["application/json"];
    keys(media, &["schema"])?;
    let name = schema::reference(&media["schema"])?;
    if !schemas.contains_key(name) {
        return Err(ApiError::Invalid);
    }
    Ok(name.into())
}
fn description(value: &Value) -> Result<(), ApiError> {
    if string(value)?.len() > 1024 {
        return Err(ApiError::Limit);
    }
    Ok(())
}
fn path_parameters(path: &str) -> Result<BTreeSet<String>, ApiError> {
    if path.len() > 256 || !path.starts_with('/') || path.contains("//") {
        return Err(ApiError::Invalid);
    }
    let mut parameters = BTreeSet::new();
    let parts = path[1..].split('/').collect::<Vec<_>>();
    if parts.len() > 16 {
        return Err(ApiError::Limit);
    }
    for part in parts {
        if let Some(name) = part.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
            if !schema::field(name) || !parameters.insert(name.into()) {
                return Err(ApiError::Invalid);
            }
        } else if part.is_empty()
            || !part
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
        {
            return Err(ApiError::Unsupported);
        }
    }
    Ok(parameters)
}
impl Document {
    pub fn parse(mut source: Value) -> Result<Self, ApiError> {
        // Generated output can be reviewed and reused as input; its receipt is
        // descriptive metadata and never substitutes for schema validation.
        if let Some(receipt) = source.get("x-rullst-generated") {
            keys(
                receipt,
                &["generator", "profile", "version", "schema_sha256"],
            )?;
            if receipt["generator"] != "cargo-rullst"
                || receipt["profile"] != "rullst.api.v1"
                || string(&receipt["version"])?.len() > 64
                || !string(&receipt["schema_sha256"])?
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit())
                || string(&receipt["schema_sha256"])?.len() != 64
            {
                return Err(ApiError::Invalid);
            }
            source
                .as_object_mut()
                .ok_or(ApiError::Invalid)?
                .remove("x-rullst-generated");
        }
        keys(
            &source,
            &[
                "openapi",
                "info",
                "paths",
                "components",
                "security",
                "x-rullst-profile",
            ],
        )?;
        if !matches!(
            source["openapi"].as_str(),
            Some("3.1.0" | "3.1.1" | "3.1.2")
        ) || source["x-rullst-profile"] != "rullst.api.v1"
        {
            return Err(ApiError::Unsupported);
        }
        keys(&source["info"], &["title", "version"])?;
        description(&source["info"]["title"])?;
        description(&source["info"]["version"])?;
        keys(&source["components"], &["schemas", "securitySchemes"])?;
        if source["security"] != serde_json::json!([{"bearerAuth": []}])
            || source["components"]["securitySchemes"]
                != serde_json::json!({"bearerAuth":{"type":"http","scheme":"bearer"}})
        {
            return Err(ApiError::Unsupported);
        }
        let schemas = object(&source["components"]["schemas"])?
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<BTreeMap<_, _>>();
        if schemas.is_empty() || schemas.len() > 32 {
            return Err(ApiError::Limit);
        }
        for (name, value) in &schemas {
            if !schema::component(name) || value["type"] != "object" {
                return Err(ApiError::Unsupported);
            }
            schema::node(value, &schemas, &mut BTreeSet::from([name.clone()]), 0)?;
        }
        let paths = object(&source["paths"])?;
        if paths.is_empty() || paths.len() > 32 {
            return Err(ApiError::Limit);
        }
        let mut operations = Vec::new();
        let mut ids = BTreeSet::new();
        let mut path_shapes = BTreeSet::new();
        for (path, item) in paths {
            let path_names = path_parameters(path)?;
            // Different parameter names cannot identify competing route shapes.
            let shape = path
                .split('/')
                .map(|p| if p.starts_with('{') { "{}" } else { p })
                .collect::<Vec<_>>()
                .join("/");
            if !path_shapes.insert(shape) {
                return Err(ApiError::Invalid);
            }
            keys(item, &["get", "post", "put", "patch", "delete"])?;
            if object(item)?.is_empty() {
                return Err(ApiError::Invalid);
            }
            for (method, operation) in object(item)? {
                keys(
                    operation,
                    &["operationId", "parameters", "requestBody", "responses"],
                )?;
                let id = string(&operation["operationId"])?;
                if !schema::field(id) || !ids.insert(id.to_owned()) || ids.len() > 32 {
                    return Err(ApiError::Invalid);
                }
                let parameters = operation["parameters"]
                    .as_array()
                    .ok_or(ApiError::Invalid)?;
                if parameters.len() > 16 {
                    return Err(ApiError::Limit);
                }
                let mut params = Vec::new();
                let mut seen = BTreeSet::new();
                let mut actual_path = BTreeSet::new();
                for parameter in parameters {
                    keys(parameter, &["name", "in", "required", "schema"])?;
                    let name = string(&parameter["name"])?;
                    let location = string(&parameter["in"])?;
                    let required = parameter["required"].as_bool().ok_or(ApiError::Invalid)?;
                    if !schema::field(name) || !seen.insert((location, name)) {
                        return Err(ApiError::Invalid);
                    }
                    let node = &parameter["schema"];
                    schema::node(node, &schemas, &mut BTreeSet::new(), 0)?;
                    let (kind, nullable) = schema::kind(node)?;
                    if nullable {
                        return Err(ApiError::Unsupported);
                    }
                    match location {
                        "path"
                            if kind == "string"
                                && required
                                && node["minLength"].as_u64().is_some_and(|min| min >= 1) =>
                        {
                            actual_path.insert(name.into());
                        }
                        "query" if matches!(kind, "string" | "integer" | "boolean") => {}
                        _ => return Err(ApiError::Unsupported),
                    }
                    params.push(Parameter {
                        name: name.into(),
                        location: location.into(),
                        required,
                        schema: node.clone(),
                    });
                }
                if actual_path != path_names {
                    return Err(ApiError::Invalid);
                }
                let body = match operation.get("requestBody") {
                    Some(body) if matches!(method.as_str(), "post" | "put" | "patch") => {
                        keys(body, &["required", "content"])?;
                        if body["required"] != true {
                            return Err(ApiError::Unsupported);
                        }
                        Some(body_reference(&body["content"], &schemas)?)
                    }
                    None if matches!(method.as_str(), "get" | "delete") => None,
                    _ => return Err(ApiError::Unsupported),
                };
                let mut responses = BTreeMap::new();
                for (status, response) in object(&operation["responses"])? {
                    let number = status.parse::<u16>().map_err(|_| ApiError::Invalid)?;
                    if number.to_string() != *status
                        || !(200..=599).contains(&number)
                        || matches!(number, 204 | 205 | 304)
                    {
                        return Err(ApiError::Unsupported);
                    }
                    // Redirects are not followed and cannot be typed as JSON results.
                    if (300..400).contains(&number) {
                        return Err(ApiError::Unsupported);
                    }
                    keys(response, &["description", "content"])?;
                    description(&response["description"])?;
                    responses.insert(number, body_reference(&response["content"], &schemas)?);
                }
                if responses.len() > 16
                    || !responses.keys().any(|s| (200..300).contains(s))
                    || !responses.contains_key(&401)
                    || !responses.contains_key(&403)
                {
                    return Err(ApiError::Invalid);
                }
                operations.push(Operation {
                    id: id.into(),
                    method: method.to_uppercase(),
                    path: path.clone(),
                    parameters: params,
                    body,
                    responses,
                });
            }
        }
        Ok(Self {
            source,
            schemas,
            operations,
        })
    }
}
