// src/generators/inspect.rs — Inspection tool for routes, models, and macro expansion.
#![cfg_attr(mutants, mutants::skip)]

use colored::*;
use std::fs;
use std::path::Path;

pub fn inspect_project(target: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let target_str = target.unwrap_or("routes");
    println!(
        "{}",
        format!("🔍 Inspecting Rullst target: '{}'...", target_str)
            .cyan()
            .bold()
    );

    match target_str {
        "route" | "routes" => inspect_routes()?,
        "model" | "models" => inspect_models()?,
        "schema" => inspect_schema()?,
        other => {
            println!(
                "{}",
                format!("ℹ️ Custom inspection for '{}':", other).yellow()
            );
            let path = Path::new(other);
            if path.exists() {
                let content = fs::read_to_string(path)?;
                println!("--- {} ---", path.display());
                for (n, line) in content.lines().take(40).enumerate() {
                    println!("{:3} | {}", n + 1, line);
                }
            } else {
                println!(
                    "{}",
                    format!("❌ File or item '{}' not found in workspace.", other).red()
                );
            }
        }
    }

    Ok(())
}

fn inspect_routes() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}",
        "📌 Active Route Table & Macro Inspection:"
            .bold()
            .underline()
    );

    let src_dir = Path::new("src");
    if !src_dir.exists() {
        println!("{}", "⚠️ 'src/' directory not found.".yellow());
        return Ok(());
    }

    let mut found_routes = Vec::new();
    scan_dir_for_routes(src_dir, &mut found_routes)?;

    if found_routes.is_empty() {
        println!(
            "{}",
            "  (No explicit routes! macro calls found in src/)".dimmed()
        );
    } else {
        println!("\n  {:<10} {:<30} {:<30}", "METHOD", "PATH", "HANDLER");
        println!("  {}", "-".repeat(70).dimmed());
        for (method, path, handler) in found_routes {
            println!(
                "  {:<10} {:<30} {:<30}",
                method.green().bold(),
                path.yellow(),
                handler.cyan()
            );
        }
    }

    println!("\n{}", "✨ Route inspection completed.".green());
    Ok(())
}

fn scan_dir_for_routes(
    dir: &Path,
    routes: &mut Vec<(String, String, String)>,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            scan_dir_for_routes(&path, routes)?;
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            let content = fs::read_to_string(&path)?;
            for line in content.lines() {
                let line_trim = line.trim();
                if (line_trim.starts_with("get(")
                    || line_trim.starts_with("post(")
                    || line_trim.starts_with("put(")
                    || line_trim.starts_with("delete("))
                    && line_trim.contains("=>")
                {
                    let parts: Vec<&str> = line_trim.split("=>").collect();
                    if parts.len() == 2 {
                        let left = parts[0].trim();
                        let handler = parts[1].trim().trim_matches(',').trim();

                        let method = if left.starts_with("get") {
                            "GET"
                        } else if left.starts_with("post") {
                            "POST"
                        } else if left.starts_with("put") {
                            "PUT"
                        } else if left.starts_with("delete") {
                            "DELETE"
                        } else {
                            "ALL"
                        };

                        let path = left
                            .find('"')
                            .and_then(|start| {
                                left[start + 1..]
                                    .find('"')
                                    .map(|end| &left[start + 1..start + 1 + end])
                            })
                            .unwrap_or(left);

                        routes.push((method.to_string(), path.to_string(), handler.to_string()));
                    }
                }
            }
        }
    }
    Ok(())
}

fn inspect_models() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}",
        "🗄️ Model & ORM Structural Inspection:".bold().underline()
    );

    let models_dir = Path::new("src/models");
    if !models_dir.exists() {
        println!("{}", "⚠️ 'src/models' directory not found.".yellow());
        return Ok(());
    }

    for entry in fs::read_dir(models_dir)? {
        let entry = entry?;
        let path = entry.path();
        let Some(file_name) = path.file_name() else {
            continue;
        };
        if path.extension().is_some_and(|ext| ext == "rs") && file_name != "mod.rs" {
            let content = fs::read_to_string(&path)?;
            println!("\n  📦 Model File: {}", file_name.to_string_lossy().cyan());
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("pub struct") || trimmed.starts_with("pub enum") {
                    println!("    └─ {}", trimmed.bold());
                } else if trimmed.starts_with("pub ") && trimmed.contains(':') {
                    println!("       ├─ {}", trimmed.dimmed());
                }
            }
        }
    }

    println!("\n{}", "✨ Model inspection completed.".green());
    Ok(())
}

fn inspect_schema() -> Result<(), Box<dyn std::error::Error>> {
    // A project-provided snapshot keeps precedence; Rullst does not write one.
    let schema_file = Path::new("rullst-schema.json");
    if schema_file.exists() {
        let content = fs::read_to_string(schema_file)?;
        println!(
            "{}",
            "📄 Project-provided schema snapshot (rullst-schema.json):".bold()
        );
        println!("{}", content);
        return Ok(());
    }

    let tables = super::schema_diff::extract_tables_from_ast();
    println!(
        "{}",
        "📄 ORM model schema derived from #[derive(Orm)] structs under src/:".bold()
    );
    println!("{}", serde_json::to_string_pretty(&model_schema(&tables))?);
    if tables.is_empty() {
        println!(
            "{}",
            "  (No #[derive(Orm)] structs found under src/)".dimmed()
        );
    }
    Ok(())
}

/// Structural JSON for the statically extracted ORM models, sorted by table.
fn model_schema(tables: &[super::schema_diff::ParsedTable]) -> serde_json::Value {
    let mut tables: Vec<_> = tables.iter().collect();
    tables.sort_by(|left, right| {
        (&left.table_name, &left.struct_name).cmp(&(&right.table_name, &right.struct_name))
    });
    serde_json::json!({
        "source": "src",
        "models": tables
            .iter()
            .map(|table| {
                serde_json::json!({
                    "struct": table.struct_name,
                    "table": table.table_name,
                    "fields": table
                        .fields
                        .iter()
                        .map(|field| {
                            serde_json::json!({
                                "name": field.name,
                                "rust_type": field.rust_type,
                                "optional": field.is_option,
                            })
                        })
                        .collect::<Vec<_>>(),
                })
            })
            .collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use super::super::schema_diff::{ParsedField, ParsedTable};
    use super::*;

    #[test]
    fn model_schema_lists_tables_fields_and_optionality_in_table_order() {
        let table = |table: &str, structure: &str, fields: Vec<ParsedField>| ParsedTable {
            table_name: table.to_string(),
            struct_name: structure.to_string(),
            fields,
        };
        let schema = model_schema(&[
            table(
                "users",
                "User",
                vec![ParsedField {
                    name: "email".to_string(),
                    rust_type: "String".to_string(),
                    is_option: true,
                }],
            ),
            table("audit_events", "AuditEvent", Vec::new()),
        ]);
        assert_eq!(
            schema,
            serde_json::json!({
                "source": "src",
                "models": [
                    {"struct": "AuditEvent", "table": "audit_events", "fields": []},
                    {
                        "struct": "User",
                        "table": "users",
                        "fields": [{"name": "email", "rust_type": "String", "optional": true}]
                    }
                ]
            })
        );
    }
}
