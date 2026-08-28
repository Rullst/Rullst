// src/generators/openapi.rs — OpenAPI generator.

use crate::generators::is_rullst_project;
use colored::*;
use std::fs;
use std::path::Path;

fn extract_description_from_handler(handler_path: &str) -> Option<String> {
    let parts: Vec<&str> = handler_path.split("::").collect();
    if parts.len() < 2 {
        return None;
    }
    let action = parts.last()?.to_string();
    let controller_module = parts[parts.len() - 2];

    // Find in src/controllers/<controller_module>.rs
    let controller_path = Path::new("src/controllers").join(format!("{}.rs", controller_module));
    if !controller_path.exists() {
        return None;
    }

    let content = fs::read_to_string(controller_path).ok()?;
    let lines: Vec<&str> = content.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        if line.contains(&format!("pub async fn {}", action)) {
            let mut comments = Vec::new();
            let mut j = i;
            while j > 0 {
                j -= 1;
                let prev_line = lines[j].trim();
                if prev_line.starts_with("///") {
                    comments.push(prev_line["///".len()..].trim().to_string());
                } else if prev_line.starts_with("#[") || prev_line.is_empty() {
                    // skip decorators and empty lines
                    continue;
                } else {
                    break;
                }
            }
            if !comments.is_empty() {
                comments.reverse();
                return Some(comments.join(" "));
            }
        }
    }
    None
}

pub fn generate_openapi_spec() -> Result<(), Box<dyn std::error::Error>> {
    if !is_rullst_project() {
        println!(
            "{}",
            "❌ Error: This command must be executed in the root of a valid Rullst project."
                .red()
                .bold()
        );
        std::process::exit(1);
    }

    println!(
        "{}",
        "🔍 Scanning project to extract routes and generate OpenAPI specification..."
            .cyan()
            .bold()
    );

    let main_path = Path::new("src/main.rs");
    if !main_path.exists() {
        println!("{}", "❌ Error: File src/main.rs not found.".red());
        std::process::exit(1);
    }

    let main_content = fs::read_to_string(main_path)?;

    // Parses Axum get/post/put/delete routing patterns
    let route_regex = regex::Regex::new(
        r#"(get|post|put|delete|patch|options|head)\s*\(\s*"([^"]+)"\s*=>\s*([\w_:]+)\s*\)"#,
    )?;

    let mut paths_map: std::collections::BTreeMap<String, serde_json::Value> =
        std::collections::BTreeMap::new();

    for cap in route_regex.captures_iter(&main_content) {
        let method = cap[1].to_lowercase();
        let path = cap[2].to_string();
        let handler_path = cap[3].to_string();

        // Convert route parameters from Axum format (:id) to OpenAPI format ({id})
        let openapi_path = path
            .split('/')
            .map(|segment| {
                if segment.starts_with(':') {
                    format!("{{{}}}", &segment[1..])
                } else {
                    segment.to_string()
                }
            })
            .collect::<Vec<String>>()
            .join("/");

        let description = extract_description_from_handler(&handler_path)
            .unwrap_or_else(|| format!("Ação '{}' executada pelo handler.", handler_path));

        let mut parameters = serde_json::json!([]);
        for segment in path.split('/') {
            if segment.starts_with(':') {
                parameters.as_array_mut().unwrap().push(serde_json::json!({
                    "name": &segment[1..],
                    "in": "path",
                    "required": true,
                    "schema": {
                        "type": "string"
                    }
                }));
            }
        }

        let mut operation = serde_json::json!({
            "summary": description,
            "responses": {
                "200": {
                    "description": "Success"
                }
            }
        });

        if !parameters.as_array().unwrap().is_empty() {
            operation
                .as_object_mut()
                .unwrap()
                .insert("parameters".to_string(), parameters);
        }

        let path_item = paths_map
            .entry(openapi_path)
            .or_insert_with(|| serde_json::json!({}));
        path_item.as_object_mut().unwrap().insert(method, operation);
    }

    let openapi = serde_json::json!({
        "openapi": "3.0.0",
        "info": {
            "title": "Especificação da API Rullst",
            "description": "Specification automatically generated via the cargo-rullst static analyzer.",
            "version": "1.0.0"
        },
        "paths": paths_map
    });

    let output_path = Path::new("openapi.json");
    fs::write(output_path, serde_json::to_string_pretty(&openapi)?)?;

    println!(
        "{}",
        format!(
            "✨ OpenAPI JSON specification successfully created at '{}'!",
            output_path.display()
        )
        .green()
        .bold()
    );
    Ok(())
}
