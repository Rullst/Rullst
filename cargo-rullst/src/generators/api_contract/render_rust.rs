use super::{ApiError, operations::Document, schema};
use serde_json::json;

pub(super) fn render(doc: &Document) -> Result<String, ApiError> {
    let mut output = String::from(include_str!("templates/runtime.rs"));
    if doc.operations.iter().any(|op| op.body.is_some()) {
        output.push_str(include_str!("templates/numbers.rs"));
    } else {
        let start = output.find("    fn decode<T:").ok_or(ApiError::Encoding)?;
        let end = output.find("    fn encode<T:").ok_or(ApiError::Encoding)?;
        output.replace_range(start..end, "");
    }
    let source = serde_json::to_string(&doc.source).map_err(|_| ApiError::Encoding)?;
    output.push_str(&format!("\npub const OPENAPI_JSON: &str = {source:?};\n"));
    let mut constructor = String::from(
        "\nimpl Contract {\n pub fn new() -> Result<Self, ContractError> {\n let document: Value = serde_json::from_str(OPENAPI_JSON).map_err(|_| ContractError::Configuration)?;\n let mut policies = BTreeMap::new();\n",
    );
    for (name, value) in &doc.schemas {
        output.push_str(&format!("\n#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]\n#[serde(deny_unknown_fields)]\npub struct Dto{name} {{\n"));
        for (field, value) in schema::object(&value["properties"])? {
            let optional = !schema::required(&doc.schemas[name], field);
            let mut ty = schema::rust_type(value)?;
            if optional {
                ty = format!("std::option::Option<{ty}>");
                output.push_str(" #[serde(default, skip_serializing_if = \"Option::is_none\")]\n");
            }
            output.push_str(&format!(" pub r#{field}: {ty},\n"));
        }
        output.push_str("}\n");
        constructor.push_str(&format!(" policies.insert({name:?}.into(), JsonSchemaPolicy::from_openapi_component(&document, {name:?}).map_err(|_| ContractError::Configuration)?);\n"));
    }
    for operation in &doc.operations {
        let id = &operation.id;
        let mut properties = serde_json::Map::new();
        let mut required = Vec::new();
        output.push_str(&format!("\npub mod op_{id} {{\n use super::*;\n pub const METHOD: &str = {:?};\n pub const PATH: &str = {:?};\n #[derive(Debug, Clone, PartialEq, serde::Deserialize)]\n #[serde(deny_unknown_fields)]\n pub struct Params {{\n", operation.method, operation.path));
        let mut definitions = Vec::new();
        for parameter in &operation.parameters {
            let prefix = if parameter.location == "path" {
                "p"
            } else {
                "q"
            };
            let name = format!("{prefix}_{}", parameter.name);
            let mut ty = schema::rust_type(&parameter.schema)?;
            if parameter.required {
                required.push(name.clone());
            } else {
                ty = format!("Option<{ty}>");
            }
            output.push_str(&format!(" pub {name}: {ty},\n"));
            properties.insert(name, parameter.schema.clone());
            definitions.push(format!(
                "({prefix:?}, {:?}, {:?})",
                parameter.name,
                schema::kind(&parameter.schema)?.0
            ));
        }
        output.push_str(" }\n");
        let params = json!({"type":"object", "properties":properties, "required":required, "additionalProperties": false});
        let encoded = serde_json::to_string(&params).map_err(|_| ApiError::Encoding)?;
        constructor.push_str(&format!(" policies.insert(\"@{id}\".into(), JsonSchemaPolicy::from_schema(serde_json::from_str({encoded:?}).map_err(|_| ContractError::Configuration)?).map_err(|_| ContractError::Configuration)?);\n"));
        output.push_str(&format!(" pub fn decode_params(contract: &Contract, path: &[(&str, &str)], query: &[(&str, &str)]) -> Result<Params, ContractError> {{\n contract.parameters(\"@{id}\", &[{}], path, query)\n }}\n", definitions.join(",")));
        if let Some(body) = &operation.body {
            output.push_str(&format!(" pub type Request = Dto{body};\n pub fn decode_request(contract: &Contract, bytes: &[u8]) -> Result<Request, ContractError> {{ contract.decode({body:?}, bytes) }}\n"));
        }
        output.push_str(
            " #[derive(Debug, Clone, PartialEq)]\n #[non_exhaustive]\n pub enum Response {\n",
        );
        for (status, name) in &operation.responses {
            output.push_str(&format!(" Status{status}(Dto{name}),\n"));
        }
        output.push_str(" }\n pub fn encode_response(contract: &Contract, response: &Response) -> Result<(u16, Vec<u8>), ContractError> {\n match response {\n");
        for (status, name) in &operation.responses {
            output.push_str(&format!(" Response::Status{status}(body) => Ok(({status}, contract.encode({name:?}, body)?)),\n"));
        }
        output.push_str(" }\n }\n}\n");
    }
    constructor.push_str(" Ok(Self { policies })\n }\n}\n");
    output.push_str(&constructor);
    // Parse the generated module before it can be written into an application.
    syn::parse_file(&output).map_err(|_| ApiError::Encoding)?;
    Ok(output)
}
