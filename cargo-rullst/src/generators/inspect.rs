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
        } else if path.extension().map_or(false, |ext| ext == "rs") {
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
        if path.extension().map_or(false, |ext| ext == "rs")
            && path.file_name().unwrap() != "mod.rs"
        {
            let content = fs::read_to_string(&path)?;
            println!(
                "\n  📦 Model File: {}",
                path.file_name().unwrap().to_string_lossy().cyan()
            );
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
    let schema_file = Path::new("rullst-schema.json");
    if schema_file.exists() {
        let content = fs::read_to_string(schema_file)?;
        println!("{}", "📄 Structural Schema (rullst-schema.json):".bold());
        println!("{}", content);
    } else {
        println!(
            "{}",
            "⚠️ 'rullst-schema.json' not found. Run 'cargo rullst dev' to generate schema."
                .yellow()
        );
    }
    Ok(())
}
