//! Scalar Interactive API Documentation Generator (`cargo rullst make:scalar`)

use crate::generators::{is_rullst_project, register_mod_ast};
use colored::Colorize;
use std::fs;
use std::io::{Error as IoError, ErrorKind, Write};
use std::path::Path;

fn write_new(path: &Path, contents: &[u8]) -> Result<(), IoError> {
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| {
            if error.kind() == ErrorKind::AlreadyExists {
                IoError::new(
                    ErrorKind::AlreadyExists,
                    format!(
                        "refusing to overwrite existing Scalar controller '{}'",
                        path.display()
                    ),
                )
            } else {
                error
            }
        })?;
    if let Err(error) = output.write_all(contents) {
        drop(output);
        let _ = fs::remove_file(path);
        return Err(error);
    }
    Ok(())
}

/// Scaffolds Scalar API documentation router integration.
pub fn generate_scalar_docs() -> Result<(), Box<dyn std::error::Error>> {
    if !is_rullst_project() {
        return Err(IoError::new(
            ErrorKind::InvalidInput,
            "make:scalar must run in a Rullst project root",
        )
        .into());
    }

    let target_path = Path::new("src/controllers/docs_controller.rs");

    if let Some(parent) = target_path.parent()
        && !parent.exists()
    {
        fs::create_dir_all(parent)?;
    }

    let code_content = r###"// src/controllers/docs_controller.rs — Scalar Interactive API Documentation
use axum::Router;
use rullst::scalar::scalar_docs_router;

/// Mounts the interactive Scalar API documentation router at `/docs`.
pub fn router() -> Router {
    scalar_docs_router("/openapi.json")
}
"###;

    write_new(target_path, code_content.as_bytes())?;
    if let Err(error) = register_mod_ast(Path::new("src/controllers/mod.rs"), "docs_controller") {
        let _ = fs::remove_file(target_path);
        return Err(error);
    }

    println!(
        "{}",
        "📖 Scalar Interactive API Docs Scaffolded Successfully!"
            .green()
            .bold()
    );
    println!(
        "   📁 Controller: {}",
        "src/controllers/docs_controller.rs".cyan()
    );
    println!(
        "   🌐 Mount `docs_controller::router()` before opening {}",
        "http://localhost:3000/docs".bold().yellow()
    );
    println!("   💡 Spec Source: {}", "/openapi.json".bold());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn scalar_controller_output_never_overwrites_an_existing_file() {
        let directory = tempdir().expect("temporary directory");
        let path = directory.path().join("docs_controller.rs");
        write_new(&path, b"first").expect("first write");
        assert_eq!(
            write_new(&path, b"second")
                .expect_err("collision must fail")
                .kind(),
            ErrorKind::AlreadyExists
        );
        assert_eq!(fs::read(path).expect("controller"), b"first");
    }
}
