//! Scalar Interactive API Documentation Generator (`cargo rullst make:scalar`)

use crate::generators::{is_rullst_project, register_mod_ast};
use colored::Colorize;
use std::fs;
use std::io::{Error as IoError, ErrorKind, Write};
use std::path::Path;

/// The controller emitted by `make:scalar`. Axum is reached through the
/// `rullst::web` re-export because generated projects do not depend on it.
const SCALAR_CONTROLLER: &str = r###"// src/controllers/docs_controller.rs — Scalar Interactive API Documentation
use rullst::scalar::scalar_docs_router;
use rullst::web::axum::Router;

/// Mounts the interactive Scalar API documentation router at `/docs`.
pub fn router() -> Router {
    scalar_docs_router("/openapi.json")
}
"###;

/// Returns the exact controller source emitted by `make:scalar`.
///
/// Public so scaffold smoke tests can compile the generated module.
#[doc(hidden)]
pub fn scalar_controller_source() -> &'static str {
    SCALAR_CONTROLLER
}

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

    write_new(target_path, SCALAR_CONTROLLER.as_bytes())?;
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
    fn scalar_controller_reaches_axum_through_the_rullst_facade() {
        let source = scalar_controller_source();
        assert!(source.contains("use rullst::web::axum::Router;"));
        assert!(!source.contains("use axum::"));
        syn::parse_file(source).expect("generated Scalar controller should parse");
    }

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
