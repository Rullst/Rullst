use super::{ApiError, operations::Document, schema};

pub(super) fn render(doc: &Document) -> Result<String, ApiError> {
    let mut output = String::from(
        "// Compile with strictNullChecks and exactOptionalPropertyTypes (or stricter).\n",
    );
    output.push_str(include_str!("templates/validate.ts"));
    let schemas = serde_json::to_string(&doc.schemas).map_err(|_| ApiError::Encoding)?;
    let operations = serde_json::to_string(&doc.operations).map_err(|_| ApiError::Encoding)?;
    output.push_str(&format!("\nconst schemas: Record<string, JsonObject> = {schemas};\nconst operations: Operation[] = {operations};\n"));
    for (name, value) in &doc.schemas {
        output.push_str(&format!("\nexport interface Dto{name} {{\n"));
        for (field, child) in schema::object(&value["properties"])? {
            let suffix = if schema::required(value, field) {
                ""
            } else {
                "?"
            };
            output.push_str(&format!(
                " {field:?}{suffix}: {};\n",
                schema::ts_type(child)?
            ));
        }
        output.push_str("}\n");
    }
    let mut methods = String::new();
    for (index, operation) in doc.operations.iter().enumerate() {
        let id = &operation.id;
        output.push_str(&format!("\nexport interface Input_{id} {{\n"));
        for parameter in &operation.parameters {
            let prefix = if parameter.location == "path" {
                "p"
            } else {
                "q"
            };
            let optional = if parameter.required { "" } else { "?" };
            output.push_str(&format!(
                " {prefix}_{}{optional}: {};\n",
                parameter.name,
                schema::ts_type(&parameter.schema)?
            ));
        }
        if let Some(body) = &operation.body {
            output.push_str(&format!(" body: Dto{body};\n"));
        }
        output.push_str("}\n");
        let responses = operation
            .responses
            .iter()
            .map(|(status, name)| format!("{{ status: {status}; body: Dto{name} }}"))
            .collect::<Vec<_>>();
        output.push_str(&format!(
            "export type Result_{id} = {};\n",
            responses.join(" | ")
        ));
        methods.push_str(&format!(" public async call_{id}(input: Input_{id}, signal?: AbortSignal): Promise<Result_{id}> {{\n return await this.execute(operations[{index}], input as unknown as JsonObject, signal) as Result_{id};\n }}\n"));
    }
    output.push_str(
        &include_str!("templates/client.ts").replace("/* GENERATED_METHODS */", &methods),
    );
    Ok(output)
}
