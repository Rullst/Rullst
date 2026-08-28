// src/generators/worker.rs — Background Worker generator.

use crate::generators::is_rullst_project;
use colored::*;
use std::fs;
use std::path::Path;

pub fn worker_to_snake_case(s: &str) -> String {
    let mut base = s.to_string();
    if base.to_lowercase().ends_with("worker") {
        let len = base.len();
        base.truncate(len - 6);
    }

    let mut result = String::new();
    let mut prev_is_lower = false;
    for c in base.chars() {
        if c == '_' || c == '-' {
            result.push('_');
            prev_is_lower = false;
        } else if c.is_uppercase() {
            if prev_is_lower {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
            prev_is_lower = false;
        } else {
            result.push(c);
            prev_is_lower = true;
        }
    }

    result.push_str("_worker");

    // Clean duplicate underscores
    let mut clean_result = String::new();
    let mut prev_is_underscore = false;
    for c in result.chars() {
        if c == '_' {
            if !prev_is_underscore {
                clean_result.push(c);
            }
            prev_is_underscore = true;
        } else {
            clean_result.push(c);
            prev_is_underscore = false;
        }
    }
    clean_result.trim_matches('_').to_string()
}

pub fn create_new_worker(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    if !is_rullst_project() {
        println!(
            "{}",
            "❌ Error: This command must be executed in the root of a valid Rullst project."
                .red()
                .bold()
        );
        std::process::exit(1);
    }

    let snake_name = worker_to_snake_case(name);
    let job_name = snake_name.strip_suffix("_worker").unwrap_or(&snake_name);

    println!(
        "{}",
        format!("🛠️ Generating background worker Rullst: {}...", snake_name)
            .cyan()
            .bold()
    );

    let workers_dir = Path::new("src/workers");
    if !workers_dir.exists() {
        fs::create_dir_all(workers_dir)?;
    }

    let mod_path = workers_dir.join("mod.rs");
    if !mod_path.exists() {
        fs::write(&mod_path, "")?;
    }

    // Add module declaration to mod.rs
    let mut mod_content = fs::read_to_string(&mod_path)?;
    let mod_declaration = format!("pub mod {};", snake_name);
    if !mod_content.contains(&mod_declaration) {
        if !mod_content.is_empty() && !mod_content.ends_with('\n') {
            mod_content.push('\n');
        }
        mod_content.push_str(&mod_declaration);
        mod_content.push('\n');
    }

    // Ensure register_workers function exists in mod.rs
    if !mod_content.contains("pub fn register_workers") {
        mod_content.push_str("\npub fn register_workers(worker: &mut rullst::queue::Worker) {\n");
        mod_content.push_str(&format!("    {}::register(worker);\n", snake_name));
        mod_content.push_str("}\n");
    } else {
        // Inject registration inside register_workers
        let search_str = "pub fn register_workers(worker: &mut rullst::queue::Worker) {";
        if let Some(pos) = mod_content.find(search_str) {
            let insert_pos = pos + search_str.len() + 1;
            mod_content.insert_str(
                insert_pos,
                &format!("    {}::register(worker);\n", snake_name),
            );
        }
    }
    fs::write(&mod_path, mod_content)?;

    let worker_path = workers_dir.join(format!("{}.rs", snake_name));
    if worker_path.exists() {
        println!(
            "{}",
            format!(
                "⚠️ Warning: Worker '{}.rs' already exists. Skipping creation.",
                snake_name
            )
            .yellow()
        );
    } else {
        let template = format!(
            r#"use rullst::queue::{{Worker, QueueError}};
use serde_json::Value;

/// Registra o processador de tarefas deste worker.
pub fn register(worker: &mut Worker) {{
    worker.register("{job_name}", |payload: Value| async move {{
        println!("🚀 [Worker] Processando tarefa '{job_name}' com payload: {{:?}}", payload);
        
        // Escreva a lógica da sua tarefa em segundo plano aqui (ex: enviar e-mail, processar imagem)
        
        Ok(())
    }});
}}
"#,
            job_name = job_name
        );
        fs::write(&worker_path, template)?;
    }

    // Add module declaration to src/main.rs
    let main_path = Path::new("src/main.rs");
    if main_path.exists() {
        let mut main_content = fs::read_to_string(main_path)?;
        if !main_content.contains("pub mod workers;") && !main_content.contains("mod workers;") {
            if main_content.contains("pub mod controllers;") {
                main_content = main_content.replace(
                    "pub mod controllers;",
                    "pub mod controllers;\npub mod workers;",
                );
            } else if main_content.contains("pub mod models;") {
                main_content =
                    main_content.replace("pub mod models;", "pub mod models;\npub mod workers;");
            } else {
                main_content = format!("pub mod workers;\n{}", main_content);
            }
            fs::write(main_path, main_content)?;
            println!(
                "{}",
                "ℹ️ Automatically added 'pub mod workers;' to src/main.rs.".cyan()
            );
        }
    }

    println!(
        "{}",
        format!(
            "✨ Worker '{}' successfully created at '{}'!",
            snake_name,
            worker_path.display()
        )
        .green()
        .bold()
    );
    println!(
        "{}",
        "How to initialize the background Worker in your 'src/main.rs':".cyan()
    );
    println!(
        "{}",
        "  1. Create the queue and initialize the worker:".cyan()
    );
    println!(
        "{}",
        "     let queue = rullst::Queue::sqlite(\"sqlite://rullst.db\").await?;".cyan()
    );
    println!(
        "{}",
        "     let mut worker = rullst::queue::Worker::new(&queue);".cyan()
    );
    println!("{}", "  2. Register your workers:".cyan());
    println!("{}", "     workers::register_workers(&mut worker);".cyan());
    println!("{}", "  3. Start the processing loop:".cyan());
    println!("{}", "     worker.run();".cyan());

    Ok(())
}
