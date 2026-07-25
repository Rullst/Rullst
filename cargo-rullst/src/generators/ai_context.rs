use colored::*;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

pub fn generate_ai_context(base_path: Option<&Path>) -> Result<(), Box<dyn std::error::Error>> {
    let base = base_path.unwrap_or_else(|| Path::new("."));

    println!(
        "{}",
        "🤖 Generating AI Context (.llms.txt)...".bright_cyan()
    );

    let mut context = String::new();
    context.push_str("# Rullst Project Context\n\n");
    context.push_str("This file provides context for LLMs (like Cursor, Claude, Windsurf, Gemini) about this Rullst application.\n\n");

    // Read Cargo.toml
    let cargo_toml_path = base.join("Cargo.toml");
    if cargo_toml_path.exists() {
        context.push_str("## Dependencies (Cargo.toml)\n```toml\n");
        if let Ok(content) = fs::read_to_string(&cargo_toml_path) {
            context.push_str(&content);
        }
        context.push_str("\n```\n\n");
    }

    // Function to read all files in a directory and append
    let mut append_dir = |dir: &str, title: &str| {
        let full_dir = base.join(dir);
        if full_dir.exists() {
            context.push_str(&format!("## {}\n\n", title));
            for entry in WalkDir::new(&full_dir).into_iter().filter_map(|e| e.ok()) {
                if entry.path().is_file()
                    && entry.path().extension().map_or(false, |e| e == "rs")
                {
                    if let Ok(content) = fs::read_to_string(entry.path()) {
                        let rel_path = entry.path().display().to_string().replace("\\", "/");
                        context.push_str(&format!(
                            "### {}\n```rust\n{}\n```\n\n",
                            rel_path, content
                        ));
                    }
                }
            }
        }
    };

    append_dir("src/models", "Models");
    append_dir("src/controllers", "Controllers");
    append_dir("src/middlewares", "Middlewares");
    append_dir("src/workers", "Background Workers");

    let output_path = base.join(".llms.txt");
    fs::write(&output_path, context)?;
    println!(
        "{}",
        "✅ AI Context generated successfully at .llms.txt".green()
    );

    Ok(())
}
