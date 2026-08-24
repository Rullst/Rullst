use crate::generators::{
    is_rullst_project, is_valid_rust_identifier, model_to_pascal_case, model_to_snake_case,
    register_mod_ast,
};
use colored::*;
use std::fs;
use std::io::{Error as IoError, ErrorKind};
use std::path::Path;

const ISLAND_TEMPLATE: &str = include_str!("island.rs.template");

fn has_dependency(manifest: &str, dependency: &str) -> bool {
    manifest.lines().any(|line| {
        let declaration = line.split_once('#').map_or(line, |(value, _)| value).trim();
        declaration
            .split_once('=')
            .is_some_and(|(key, _)| key.trim().trim_matches(['\'', '"']) == dependency)
    })
}

fn insert_after_table_header(
    manifest: &mut String,
    header: &str,
    declaration: &str,
) -> Result<(), IoError> {
    let Some(start) = manifest.find(header) else {
        return Err(IoError::new(
            ErrorKind::InvalidData,
            format!("Cargo.toml does not contain {header}"),
        ));
    };
    manifest.insert_str(start + header.len(), &format!("\n{declaration}"));
    Ok(())
}

fn ensure_island_dependencies(manifest: &str) -> Result<String, IoError> {
    let mut updated = manifest.to_string();
    for (name, declaration) in [
        (
            "serde",
            "serde = { version = \"1\", features = [\"derive\"] }",
        ),
        ("serde_json", "serde_json = \"1\""),
    ] {
        if !has_dependency(&updated, name) {
            insert_after_table_header(&mut updated, "[dependencies]", declaration)?;
        }
    }

    let target_header = "[target.'cfg(target_arch = \"wasm32\")'.dependencies]";
    if !updated.contains(target_header) {
        if !updated.ends_with('\n') {
            updated.push('\n');
        }
        updated.push_str(&format!("\n{target_header}\n"));
    }
    if !has_dependency(&updated, "wasm-bindgen") {
        insert_after_table_header(&mut updated, target_header, "wasm-bindgen = \"0.2\"")?;
    }
    if !has_dependency(&updated, "web-sys") {
        insert_after_table_header(
            &mut updated,
            target_header,
            "web-sys = { version = \"0.3\", features = [\"Document\", \"Element\", \"EventTarget\", \"Window\"] }",
        )?;
    }

    Ok(updated)
}

pub(crate) fn render_island(name: &str) -> (String, String, String) {
    let module_name = model_to_snake_case(name);
    let type_name = model_to_pascal_case(name);
    let source = ISLAND_TEMPLATE
        .replace("__MODULE_NAME__", &module_name)
        .replace("__TYPE_NAME__", &type_name);
    (module_name, type_name, source)
}

pub fn create_new_island(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    if !is_rullst_project() {
        println!(
            "{}",
            "❌ Error: This command must be executed in the root of a valid Rullst project."
                .red()
                .bold()
        );
        std::process::exit(1);
    }

    let (module_name, type_name, source) = render_island(name);
    if !is_valid_rust_identifier(&module_name) || !is_valid_rust_identifier(&type_name) {
        return Err(IoError::new(
            ErrorKind::InvalidInput,
            "island name must produce valid non-keyword Rust identifiers",
        )
        .into());
    }

    println!(
        "{}",
        format!("🏝️ Generating Wasm Island: {}...", type_name)
            .cyan()
            .bold()
    );

    let islands_dir = Path::new("src/islands");
    if !islands_dir.exists() {
        fs::create_dir_all(islands_dir)?;
    }

    register_mod_ast(&islands_dir.join("mod.rs"), &module_name)?;

    let lib_path = Path::new("src/lib.rs");
    if lib_path.exists() {
        register_mod_ast(lib_path, "islands")?;
    } else {
        fs::write(lib_path, "pub mod islands;\n")?;
    }

    let cargo_toml_path = Path::new("Cargo.toml");
    let manifest = fs::read_to_string(cargo_toml_path)?;
    let updated_manifest = ensure_island_dependencies(&manifest)?;
    if updated_manifest != manifest {
        fs::write(cargo_toml_path, updated_manifest)?;
    }

    let island_path = islands_dir.join(format!("{}.rs", module_name));
    if island_path.exists() {
        println!(
            "{}",
            format!(
                "⚠️ Warning: Island '{}.rs' already exists. Skipping file creation.",
                module_name
            )
            .yellow()
        );
    } else {
        fs::write(&island_path, source)?;
        println!(
            "{}",
            format!(
                "✅ Island '{}' created successfully at: {:?}",
                type_name, island_path
            )
            .green()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn island_names_do_not_reuse_controller_suffixes() {
        let (module_name, type_name, source) = render_island("InteractiveChart");
        assert_eq!(module_name, "interactive_chart");
        assert_eq!(type_name, "InteractiveChart");
        assert!(!source.contains("_controller"));
        assert!(!source.contains("Controller"));
        assert!(!source.contains("rullst::view"));
        assert!(!source.contains(".unwrap("));
        assert!(!source.contains(".expect("));
        assert!(!source.contains("panic!("));
        syn::parse_file(&source).expect("generated island must parse as Rust");
    }

    #[test]
    fn island_dependencies_are_injected_once() {
        let original = "[package]\nname = \"demo\"\n\n[dependencies]\nrullst = \"12\"\n";
        let updated = ensure_island_dependencies(original).expect("dependencies must be added");
        let repeated = ensure_island_dependencies(&updated).expect("injection must be idempotent");
        assert_eq!(updated, repeated);
        assert_eq!(updated.matches("wasm-bindgen =").count(), 1);
        assert_eq!(updated.matches("web-sys =").count(), 1);
        assert_eq!(updated.matches("serde_json =").count(), 1);
    }
}
