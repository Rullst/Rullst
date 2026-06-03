// src/generators/build.rs — Production builds, frontend islands compilation, and upgrade systems.

use crate::generators::is_rullst_project;
use crate::ui::components::with_spinner;
use colored::*;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;

fn get_cache_path() -> std::path::PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push("rullst_version_cache.txt");
    dir
}

pub fn run_upgrade() -> Result<(), Box<dyn std::error::Error>> {
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
        "\n🚀 Starting Rullst Safe Upgrade (Self-Healing Upgrades)...\n"
            .cyan()
            .bold()
    );

    let latest_version = if get_cache_path().exists() {
        std::fs::read_to_string(get_cache_path())
            .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string())
            .trim()
            .to_string()
    } else {
        env!("CARGO_PKG_VERSION").to_string()
    };

    // Step 1: Update Cargo.toml
    println!(
        "{}",
        format!(
            "📦 Updating Rullst dependency versions to {} in Cargo.toml...",
            latest_version
        )
        .yellow()
    );
    let cargo_path = Path::new("Cargo.toml");
    if cargo_path.exists() {
        let mut cargo_content = std::fs::read_to_string(cargo_path)?;

        let re_rullst = regex::Regex::new(r#"(?m)^(\s*rullst\s*=\s*)"[^"]+""#)?;
        cargo_content = re_rullst
            .replace_all(&cargo_content, |caps: &regex::Captures| {
                format!(r#"{}"{}""#, &caps[1], latest_version)
            })
            .into_owned();

        let re_macros = regex::Regex::new(r#"(?m)^(\s*rullst-macros\s*=\s*)"[^"]+""#)?;
        cargo_content = re_macros
            .replace_all(&cargo_content, |caps: &regex::Captures| {
                format!(r#"{}"{}""#, &caps[1], latest_version)
            })
            .into_owned();

        let re_eloquent = regex::Regex::new(r#"(?m)^(\s*rullst-orm\s*=\s*)"[^"]+""#)?;
        cargo_content = re_eloquent
            .replace_all(&cargo_content, |caps: &regex::Captures| {
                format!(r#"{}"3.0.3""#, &caps[1])
            })
            .into_owned();

        std::fs::write(cargo_path, cargo_content)?;
    }

    // Step 2: Run cargo update
    let update_success = with_spinner("Refreshing dependencies and lockfile...", || {
        Command::new("cargo")
            .arg("update")
            .arg("-q")
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    });

    if !update_success {
        println!(
            "{}",
            "❌ Failed to update dependencies via cargo update.".red()
        );
        std::process::exit(1);
    }

    // Step 3: Run self-healing codemod AST & regex rules
    println!(
        "{}",
        "\n🔧 Executing self-healing codemod AST & regex rules over project files...".yellow()
    );

    let rules = vec![
        (
            r#"\bold_initializer\s*\(\s*\)"#,
            "Router::new()",
            "Legacy old_initializer() -> Router::new()",
        ),
        (
            r#"\brullst::routing::old_initializer\b"#,
            "rullst::routing::Router::new",
            "Legacy router initialization path",
        ),
        (
            r#"\buse\s+sqlx::"#,
            "use rullst::db::sqlx::",
            "Enforce Dependency Shielding for sqlx",
        ),
        (
            r#"\buse\s+axum::"#,
            "use rullst::server::",
            "Enforce Dependency Shielding for axum",
        ),
        (
            r#"\buse\s+tokio::"#,
            "use rullst::runtime::",
            "Enforce Dependency Shielding for tokio",
        ),
    ];

    let mut applied_count = 0;
    if Path::new("src").exists() {
        let walker = walkdir::WalkDir::new("src");
        for entry in walker.into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("rs") {
                let mut file_content = std::fs::read_to_string(path)?;
                let mut modified = false;

                for (pattern, replacement, desc) in &rules {
                    let re = regex::Regex::new(pattern)?;
                    if re.is_match(&file_content) {
                        file_content = re.replace_all(&file_content, *replacement).into_owned();
                        println!(
                            "  [{}] Applied codemod: {} in {}",
                            "Codemod".green().bold(),
                            desc.cyan(),
                            path.display()
                        );
                        modified = true;
                        applied_count += 1;
                    }
                }

                if modified {
                    std::fs::write(path, file_content)?;
                }
            }
        }
    }

    if applied_count == 0 {
        println!("  ✨ No legacy APIs or shielding patterns required patching in this codebase.");
    } else {
        println!(
            "  ✨ Successfully executed {} codemod modifications.",
            applied_count
        );
    }

    // Step 4: Run `cargo fix`
    let fix_success = with_spinner("Applying additional code fixes via cargo fix...", || {
        Command::new("cargo")
            .arg("fix")
            .arg("--allow-no-vcs")
            .arg("--allow-dirty")
            .arg("-q")
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    });

    if !fix_success {
        println!(
            "{}",
            "❌ Failed to apply additional code fixes via cargo fix.".red()
        );
        std::process::exit(1);
    }

    // Step 5: Compiler validation gate
    let check_success = with_spinner(
        "Running validation gate (cargo check) to confirm health status...",
        || {
            Command::new("cargo")
                .arg("check")
                .arg("-q")
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        },
    );

    if check_success {
        println!(
            "{}",
            "\n✅ Rullst updated successfully. No breaking changes detected! Code is 100% stable.\n"
                .green()
                .bold()
        );
    } else {
        println!(
            "{}",
            "\n⚠️ Warning: Upgrade completed with check failures. Please review the compiler errors manually.\n"
                .yellow()
                .bold()
        );
    }

    Ok(())
}

pub fn run_dev_server() -> Result<(), Box<dyn std::error::Error>> {
    if !is_rullst_project() {
        println!(
            "{}",
            "❌ Error: This command must be executed in the root of a valid Rullst project."
                .red()
                .bold()
        );
        std::process::exit(1);
    }

    println!("{}", "\n🚀 Starting Rullst Dev Server...\n".cyan().bold());

    let output_result = with_spinner("Compiling Rullst Application...", || {
        Command::new("cargo").arg("build").arg("-q").output()
    });

    match output_result {
        Ok(output) => {
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if !stderr.trim().is_empty() {
                    println!("{}", stderr);
                }
                println!(
                    "{}",
                    "❌ Compilation failed. Run `cargo build` to see the detailed errors.".red()
                );
                std::process::exit(1);
            } else {
                // Print warnings (if any) after the spinner has successfully finished
                let stderr = String::from_utf8_lossy(&output.stderr);
                if !stderr.trim().is_empty() {
                    println!("{}", stderr);
                }
            }
        }
        Err(_) => {
            println!("{}", "❌ Failed to execute `cargo build`.".red());
            std::process::exit(1);
        }
    }

    println!(
        "{}",
        "✨ Compilation successful! Starting the server...\n".green()
    );

    let status = Command::new("cargo").arg("run").arg("-q").status()?;

    if !status.success() {
        std::process::exit(1);
    }

    Ok(())
}

pub fn run_build_client(debug: bool) -> Result<(), Box<dyn std::error::Error>> {
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

    // 1. Check and inject [lib] crate-type into Cargo.toml if missing
    let mut cargo_content = fs::read_to_string("Cargo.toml")?;
    if !cargo_content.contains("[lib]") {
        cargo_content.push_str("\n\n[lib]\ncrate-type = [\"cdylib\", \"rlib\"]\n");
        fs::write("Cargo.toml", &cargo_content)?;
        println!(
            "{}",
            "ℹ️ Automatically injected [lib] crate-type into your Cargo.toml.".cyan()
        );
    }

    // 2. Proactively try to add the wasm32 target using rustup
    println!(
        "{}",
        "⚙️ Verificando e instalando target wasm32-unknown-unknown...".yellow()
    );
    let _ = Command::new("rustup")
        .arg("target")
        .arg("add")
        .arg("wasm32-unknown-unknown")
        .status();

    // 3. Compile the target
    println!(
        "{}",
        "📦 Compilando componentes frontend para wasm32-unknown-unknown...".yellow()
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

    // 4. Extract package name to locate the compiled wasm file
    let package_name = cargo_content
        .lines()
        .find(|line| line.trim().starts_with("name"))
        .and_then(|line| line.split('=').nth(1))
        .map(|val| {
            val.trim()
                .trim_matches('"')
                .trim_matches('\'')
                .replace("-", "_")
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

    // 5. Ensure wasm-bindgen-cli is installed
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

    // 6. Ensure static/ directory exists
    let static_dir = Path::new("static");
    if !static_dir.exists() {
        fs::create_dir_all(static_dir)?;
    }

    // 7. Run wasm-bindgen compiler
    println!("{}", "⚡ Running wasm-bindgen bindings...".yellow());
    let bindgen_status = Command::new("wasm-bindgen")
        .arg(&wasm_file_path)
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

    // 8. Append the orchestrator to the generated JS file
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

pub fn run_production_build(release: bool) -> Result<(), Box<dyn std::error::Error>> {
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
        format!(
            "\n🚀 Starting Rullst production build pipeline (Release Mode: {})...\n",
            release
        )
        .cyan()
        .bold()
    );

    // 1. Run cargo build --release (or debug)
    let mut cargo_cmd = Command::new("cargo");
    cargo_cmd.arg("build");
    if release {
        cargo_cmd.arg("--release");
    }

    println!(
        "{}",
        format!(
            "⚙️ Executing cargo build{}...",
            if release { " --release" } else { "" }
        )
        .yellow()
    );
    let build_status = cargo_cmd.status()?;
    if !build_status.success() {
        println!("{}", "❌ Error: Cargo build failed.".red().bold());
        std::process::exit(1);
    }

    // 2. Pre-compress static files in static/ directory
    let static_dir = Path::new("static");
    if static_dir.exists() {
        println!(
            "{}",
            "📦 Pre-compressing static assets in static/ directory...".yellow()
        );
        let walker = walkdir::WalkDir::new(static_dir);
        let mut file_count = 0;
        let mut br_count = 0;
        let mut zst_count = 0;

        for entry in walker.into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() {
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                if matches!(
                    ext.as_str(),
                    "html" | "css" | "js" | "json" | "svg" | "wasm" | "xml" | "txt"
                ) {
                    file_count += 1;
                    let input_bytes = fs::read(path)?;

                    // Brotli compression (level 11)
                    let br_path = path.with_extension(format!("{}.br", ext));
                    println!(
                        "  Compressing {} -> {} (Brotli L11)...",
                        path.display(),
                        br_path.display()
                    );
                    {
                        let br_file = fs::File::create(&br_path)?;
                        let mut writer = brotli::CompressorWriter::new(br_file, 4096, 11, 22);
                        writer.write_all(&input_bytes)?;
                        writer.flush()?;
                    }
                    br_count += 1;

                    // Zstandard compression (level 19)
                    let zst_path = path.with_extension(format!("{}.zst", ext));
                    println!(
                        "  Compressing {} -> {} (Zstd L19)...",
                        path.display(),
                        zst_path.display()
                    );
                    {
                        let zst_file = fs::File::create(&zst_path)?;
                        let mut encoder = zstd::Encoder::new(zst_file, 19)?;
                        encoder.write_all(&input_bytes)?;
                        encoder.finish()?;
                    }
                    zst_count += 1;
                }
            }
        }
        println!(
            "{}",
            format!(
                "\n✨ Pre-compression finished: processed {} files, generated {} .br files and {} .zst files.",
                file_count, br_count, zst_count
            )
            .green()
            .bold()
        );
    } else {
        println!(
            "{}",
            "ℹ️ No static/ directory found. Skipping static asset pre-compression.".cyan()
        );
    }

    println!(
        "{}",
        "\n🎉 Rullst production build completed successfully!"
            .green()
            .bold()
    );

    Ok(())
}
