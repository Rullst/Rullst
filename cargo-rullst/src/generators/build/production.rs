// src/generators/build/production.rs — Production binary build + static asset pre-compression.

use crate::generators::is_rullst_project;
use colored::*;
use std::fs;
use std::io::Write as _;
use std::path::Path;
use std::process::Command;

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
