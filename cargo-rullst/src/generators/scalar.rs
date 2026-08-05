//! Scalar Interactive API Documentation Generator (`cargo rullst make:scalar`)

use colored::Colorize;
use std::fs;
use std::path::Path;

/// Scaffolds Scalar API documentation router integration.
pub fn generate_scalar_docs() -> Result<(), Box<dyn std::error::Error>> {
    let target_path = Path::new("src/controllers/docs_controller.rs");

    if let Some(parent) = target_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    let code_content = r###"// src/controllers/docs_controller.rs — Scalar Interactive API Documentation
use axum::Router;
use rullst::scalar::scalar_docs_router;

/// Mounts the interactive Scalar API documentation router at `/docs`.
pub fn router() -> Router {
    scalar_docs_router("/openapi.json")
}
"###;

    fs::write(target_path, code_content)?;

    println!("{}", "📖 Scalar Interactive API Docs Scaffolded Successfully!".green().bold());
    println!("   📁 Controller: {}", "src/controllers/docs_controller.rs".cyan());
    println!("   🌐 Access URL: {}", "http://localhost:3000/docs".bold().yellow());
    println!("   💡 Spec Source: {}", "/openapi.json".bold());

    Ok(())
}
