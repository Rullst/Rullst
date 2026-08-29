// src/generators/build/wasm.rs — Wasm Islands client build system.

use crate::generators::is_rullst_project;
use colored::*;
use std::fs;
use std::io::Error as IoError;
use std::path::Path;
use std::process::Command;
use toml_edit::{Array, DocumentMut, Item, Table, Value};

pub fn run_build_client(debug: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{} Building Wasm client artifacts...",
        "[2/3]".bold().dimmed()
    );

    if !is_rullst_project() {
        return Err(IoError::other(
            "build:client must be executed in the root of a Rullst project",
        )
        .into());
    }

    println!(
        "{}",
        "\n🏝️  Iniciando a compilação do Rullst Wasm Island Client...\n"
            .cyan()
            .bold()
    );

    let mut cargo_content = fs::read_to_string("Cargo.toml")?;
    inject_lib_crate_type_if_missing(&mut cargo_content)?;

    install_wasm32_target()?;
    compile_wasm_target(debug)?;

    let (package_name, wasm_file_path) = locate_compiled_wasm(&cargo_content, debug)?;

    ensure_wasm_bindgen_cli()?;

    let static_dir = Path::new("static");
    if !static_dir.exists() {
        fs::create_dir_all(static_dir)?;
    }

    run_wasm_bindgen(&wasm_file_path)?;
    inject_hydration_orchestrator(&package_name)?;

    println!(
        "{}",
        "✨ Rullst Wasm Islands successfully compiled and generated!"
            .green()
            .bold()
    );
    println!("{}", "How to load in your HTML page:".cyan());
    println!(
        "{}",
        "  <script type=\"module\" src=\"/static/rullst-islands.js\"></script>".cyan()
    );
    Ok(())
}

fn inject_lib_crate_type_if_missing(cargo_content: &mut String) -> Result<(), std::io::Error> {
    let Some(updated) = ensure_wasm_crate_types(cargo_content)? else {
        return Ok(());
    };
    fs::write("Cargo.toml", &updated)?;
    *cargo_content = updated;
    println!(
        "{}",
        "ℹ️ Added the required cdylib crate type to Cargo.toml.".cyan()
    );
    Ok(())
}

fn ensure_wasm_crate_types(cargo_content: &str) -> Result<Option<String>, IoError> {
    let mut document = cargo_content.parse::<DocumentMut>().map_err(|error| {
        IoError::new(
            std::io::ErrorKind::InvalidData,
            format!("Cargo.toml is not valid TOML: {error}"),
        )
    })?;

    if document.get("lib").is_none() {
        document.insert("lib", Item::Table(Table::new()));
    }
    let lib = document
        .get_mut("lib")
        .and_then(Item::as_table_mut)
        .ok_or_else(|| {
            IoError::new(
                std::io::ErrorKind::InvalidData,
                "Cargo.toml [lib] must be a table",
            )
        })?;

    match lib.get_mut("crate-type") {
        Some(item) => {
            let crate_types = item.as_array_mut().ok_or_else(|| {
                IoError::new(
                    std::io::ErrorKind::InvalidData,
                    "Cargo.toml lib.crate-type must be an array",
                )
            })?;
            if crate_types
                .iter()
                .any(|value| value.as_str() == Some("cdylib"))
            {
                return Ok(None);
            }
            crate_types.push("cdylib");
        }
        None => {
            let mut crate_types = Array::new();
            crate_types.push("cdylib");
            crate_types.push("rlib");
            lib.insert("crate-type", Item::Value(Value::Array(crate_types)));
        }
    }

    Ok(Some(document.to_string()))
}

fn install_wasm32_target() -> Result<(), IoError> {
    println!(
        "{}",
        "⚙️ Verificando e instalando target wasm32-unknown-unknown...".yellow()
    );
    let status = Command::new("rustup")
        .arg("target")
        .arg("add")
        .arg("wasm32-unknown-unknown")
        .status()?;
    if !status.success() {
        return Err(IoError::other(
            "rustup failed to install or verify wasm32-unknown-unknown",
        ));
    }
    Ok(())
}

fn compile_wasm_target(debug: bool) -> Result<(), std::io::Error> {
    println!(
        "{}",
        "📦 Compiling frontend components for wasm32-unknown-unknown...".yellow()
    );
    let mut cargo_cmd = Command::new("cargo");
    cargo_cmd
        .arg("build")
        .arg("--target")
        .arg("wasm32-unknown-unknown")
        .arg("--lib");
    if !debug {
        cargo_cmd.arg("--release");
    }
    let build_status = cargo_cmd.status()?;
    if !build_status.success() {
        return Err(IoError::other(
            "cargo failed to compile the wasm32-unknown-unknown library target",
        ));
    }
    Ok(())
}

fn locate_compiled_wasm(cargo_content: &str, debug: bool) -> Result<(String, String), IoError> {
    let package_name = compiled_wasm_stem(cargo_content)?;

    let profile = if debug { "debug" } else { "release" };
    let relative_artifact = format!("wasm32-unknown-unknown/{profile}/{package_name}.wasm");
    let candidates = ["target", "../target", "../../target"]
        .map(|target| Path::new(target).join(&relative_artifact));
    let wasm_file_path = candidates
        .iter()
        .find(|candidate| candidate.is_file())
        .ok_or_else(|| {
            let searched = candidates
                .iter()
                .map(|candidate| format!("`{}`", candidate.display()))
                .collect::<Vec<_>>()
                .join(", ");
            IoError::other(format!(
                "compiled Wasm file was not found; searched {searched}"
            ))
        })?;

    Ok((package_name, wasm_file_path.to_string_lossy().into_owned()))
}

fn compiled_wasm_stem(cargo_content: &str) -> Result<String, IoError> {
    let manifest = toml::from_str::<toml::Value>(cargo_content).map_err(|error| {
        IoError::new(
            std::io::ErrorKind::InvalidData,
            format!("Cargo.toml is not valid TOML: {error}"),
        )
    })?;
    let crate_name = manifest
        .get("lib")
        .and_then(|lib| lib.get("name"))
        .and_then(toml::Value::as_str)
        .or_else(|| {
            manifest
                .get("package")
                .and_then(|package| package.get("name"))
                .and_then(toml::Value::as_str)
        })
        .filter(|name| !name.is_empty())
        .ok_or_else(|| {
            IoError::new(
                std::io::ErrorKind::InvalidData,
                "Cargo.toml must contain a string package.name or lib.name",
            )
        })?;
    Ok(crate_name.replace('-', "_"))
}

fn ensure_wasm_bindgen_cli() -> Result<(), std::io::Error> {
    println!("{}", "🔍 Checking wasm-bindgen-cli...".yellow());
    let wasm_bindgen_installed = Command::new("wasm-bindgen")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success());

    if !wasm_bindgen_installed {
        println!(
            "{}",
            "⚙️ Automatically installing wasm-bindgen-cli... This might take a moment.".yellow()
        );
        let install_status = Command::new("cargo")
            .arg("install")
            .arg("wasm-bindgen-cli")
            .status()?;
        if !install_status.success() {
            return Err(IoError::other("cargo failed to install wasm-bindgen-cli"));
        }
    }
    Ok(())
}

fn run_wasm_bindgen(wasm_file_path: &str) -> Result<(), std::io::Error> {
    println!("{}", "⚡ Running wasm-bindgen bindings...".yellow());
    let bindgen_status = Command::new("wasm-bindgen")
        .arg(wasm_file_path)
        .arg("--out-dir")
        .arg("static")
        .arg("--target")
        .arg("web")
        .arg("--no-typescript")
        .status()?;

    if !bindgen_status.success() {
        return Err(IoError::other(
            "wasm-bindgen failed to generate browser bindings",
        ));
    }
    Ok(())
}

fn inject_hydration_orchestrator(package_name: &str) -> Result<(), std::io::Error> {
    let bindings_path = format!("static/{package_name}.js");
    if !Path::new(&bindings_path).is_file() {
        return Err(IoError::other(format!(
            "wasm-bindgen output `{bindings_path}` was not created"
        )));
    }

    fs::write(
        "static/rullst-islands.js",
        render_hydration_orchestrator(package_name),
    )
}

fn render_hydration_orchestrator(package_name: &str) -> String {
    format!(
        r#"import init, * as bindings from './{package_name}.js';

// ─── Rullst Wasm Island Hydration Loop 🏝️ ────────────────────────────────────
let initialization;

export async function hydrate_all() {{
    initialization ??= init();
    await initialization;

    const islands = document.querySelectorAll('[data-island]');
    for (const island of islands) {{
        const name = island.getAttribute('data-island');
        const props = island.getAttribute('data-props');
        const fn_name = `hydrate_${{name}}`;
        const hydrate_fn = bindings[fn_name];
        if (hydrate_fn) {{
            try {{
                hydrate_fn(island, props);
            }} catch (error) {{
                console.error(`[Rullst] Failed to hydrate island ${{name}}:`, error);
            }}
        }} else {{
            console.warn(`[Rullst] No hydration function found for island: ${{name}}`);
        }}
    }}
}}

// Automatically hydrate when ready
if (typeof document !== 'undefined') {{
    if (document.readyState === 'loading') {{
        document.addEventListener('DOMContentLoaded', () => {{ void hydrate_all(); }});
    }} else {{
        void hydrate_all();
    }}
}}
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hydration_waits_for_wasm_initialization_without_self_importing() {
        let script = render_hydration_orchestrator("demo_app");
        assert!(script.contains("import init, * as bindings from './demo_app.js';"));
        assert!(script.contains("await initialization;"));
        assert!(script.contains("bindings[fn_name]"));
        assert!(!script.contains("import('./demo_app.js')"));
    }

    #[test]
    fn wasm_crate_type_is_merged_into_an_existing_lib_table() {
        let manifest = "[package]\nname = \"demo-app\"\nversion = \"1.0.0\"\n\n[lib]\nname = \"browser_client\"\ncrate-type = [\"rlib\"]\n";
        let updated = ensure_wasm_crate_types(manifest)
            .expect("valid manifest")
            .expect("manifest should change");
        let parsed = toml::from_str::<toml::Value>(&updated).expect("updated manifest");
        let crate_types = parsed["lib"]["crate-type"].as_array().expect("crate types");
        assert!(
            crate_types
                .iter()
                .any(|value| value.as_str() == Some("rlib"))
        );
        assert!(
            crate_types
                .iter()
                .any(|value| value.as_str() == Some("cdylib"))
        );
        assert_eq!(
            compiled_wasm_stem(&updated).expect("lib artifact name"),
            "browser_client"
        );
    }

    #[test]
    fn wasm_manifest_is_unchanged_when_cdylib_is_already_present() {
        let manifest = "[package]\nname = \"demo-app\"\nversion = \"1.0.0\"\n\n[lib]\ncrate-type = [\"cdylib\"]\n";
        assert!(
            ensure_wasm_crate_types(manifest)
                .expect("valid manifest")
                .is_none()
        );
        assert_eq!(
            compiled_wasm_stem(manifest).expect("package artifact name"),
            "demo_app"
        );
    }

    #[test]
    fn malformed_wasm_manifest_is_rejected() {
        assert!(ensure_wasm_crate_types("[package").is_err());
        assert!(compiled_wasm_stem("[package").is_err());
    }
}
