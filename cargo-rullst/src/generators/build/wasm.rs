// src/generators/build/wasm.rs — Wasm Islands client build system.

use crate::generators::is_rullst_project;
use colored::*;
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn run_build_client(debug: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{} Building Wasm client artifacts...",
        "[2/3]".bold().dimmed()
    );

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
        "\n🏝️  Iniciando a compilação do Rullst Wasm Island Client...\n"
            .cyan()
            .bold()
    );

    let mut cargo_content = fs::read_to_string("Cargo.toml")?;
    inject_lib_crate_type_if_missing(&mut cargo_content)?;

    install_wasm32_target();
    compile_wasm_target(debug)?;

    let (package_name, wasm_file_path) = locate_compiled_wasm(&cargo_content, debug);

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
        format!(
            "  <script type=\"module\">\n    import init from '/static/{}.js';\n    init();\n  </script>",
            package_name
        )
        .cyan()
    );
    Ok(())
}

fn inject_lib_crate_type_if_missing(cargo_content: &mut String) -> Result<(), std::io::Error> {
    if !cargo_content.contains("[lib]") {
        cargo_content.push_str("\n\n[lib]\ncrate-type = [\"cdylib\", \"rlib\"]\n");
        fs::write("Cargo.toml", cargo_content)?;
        println!(
            "{}",
            "ℹ️ Automatically injected [lib] crate-type into your Cargo.toml.".cyan()
        );
    }
    Ok(())
}

fn install_wasm32_target() {
    println!(
        "{}",
        "⚙️ Verificando e instalando target wasm32-unknown-unknown...".yellow()
    );
    let _ = Command::new("rustup")
        .arg("target")
        .arg("add")
        .arg("wasm32-unknown-unknown")
        .status();
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
        println!(
            "{}",
            "❌ Error compiling wasm32-unknown-unknown target.".red()
        );
        std::process::exit(1);
    }
    Ok(())
}

fn locate_compiled_wasm(cargo_content: &str, debug: bool) -> (String, String) {
    let package_name = cargo_content
        .lines()
        .find(|line| line.trim().starts_with("name"))
        .and_then(|line| line.split('=').nth(1))
        .map(|val| {
            val.trim()
                .trim_matches('"')
                .trim_matches('\'')
                .replace('-', "_")
        })
        .unwrap_or_else(|| "app".to_string());

    let profile = if debug { "debug" } else { "release" };
    let mut wasm_file_path = format!(
        "target/wasm32-unknown-unknown/{}/{}.wasm",
        profile, package_name
    );

    if !Path::new(&wasm_file_path).exists() {
        if Path::new("../../target").exists() {
            wasm_file_path = format!(
                "../../target/wasm32-unknown-unknown/{}/{}.wasm",
                profile, package_name
            );
        } else if Path::new("../target").exists() {
            wasm_file_path = format!(
                "../target/wasm32-unknown-unknown/{}/{}.wasm",
                profile, package_name
            );
        }
    }

    if !Path::new(&wasm_file_path).exists() {
        println!(
            "{}",
            format!("❌ Error: Compiled Wasm file not found at '{}'. Rullst also searched in parent directories.", wasm_file_path).red()
        );
        std::process::exit(1);
    }
    (package_name, wasm_file_path)
}

fn ensure_wasm_bindgen_cli() -> Result<(), std::io::Error> {
    println!("{}", "🔍 Checking wasm-bindgen-cli...".yellow());
    let wasm_bindgen_installed = Command::new("wasm-bindgen")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok();

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
            println!(
                "{}",
                "❌ Failed to automatically install wasm-bindgen-cli.".red()
            );
            std::process::exit(1);
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
        println!(
            "{}",
            "❌ Error generating bindings with wasm-bindgen.".red()
        );
        std::process::exit(1);
    }
    Ok(())
}

fn inject_hydration_orchestrator(package_name: &str) -> Result<(), std::io::Error> {
    let js_file_path = format!("static/{}.js", package_name);
    if Path::new(&js_file_path).exists() {
        let mut js_content = fs::read_to_string(&js_file_path)?;

        let orchestrator = format!(
            r#"
// ─── Rullst Wasm Island Hydration Loop 🏝️ ────────────────────────────────────
export function hydrate_all() {{
    import('./{}.js').then((m) => {{
        const islands = document.querySelectorAll('[data-island]');
        for (const island of islands) {{
            const name = island.getAttribute('data-island');
            const props = island.getAttribute('data-props');
            const fn_name = `hydrate_${{name}}`;
            const hydrate_fn = m[fn_name];
            if (hydrate_fn) {{
                try {{
                    hydrate_fn(island, props);
                    console.log(`[Rullst] Hydrated island: ${{name}}`);
                }} catch (e) {{
                    console.error(`[Rullst] Failed to hydrate island ${{name}}:`, e);
                }}
            }} else {{
                console.warn(`[Rullst] No hydration function found for island: ${{name}}`);
            }}
        }}
    }}).catch(e => console.error("[Rullst] Failed to load Wasm ES module:", e));
}}

// Automatically hydrate when ready
if (typeof document !== 'undefined') {{
    if (document.readyState === 'loading') {{
        document.addEventListener('DOMContentLoaded', hydrate_all);
    }} else {{
        hydrate_all();
    }}
}}
"#,
            package_name
        );

        js_content.push_str(&orchestrator);
        fs::write(&js_file_path, js_content)?;
    }
    Ok(())
}
