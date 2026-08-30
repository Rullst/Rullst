// src/generators/desktop/runner.rs — Omni app runner for desktop, Android, and iOS.

use crate::ui::spinner::with_spinner;
use colored::*;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::Stdio;

struct ChildGuard(std::process::Child);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[cfg_attr(mutants, mutants::skip)]
pub fn run_omni_app(target: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let omni_dir = Path::new("omni-app");
    if !omni_dir.exists() {
        println!(
            "{}",
            "❌ Error: 'omni-app' directory not found. Please run `cargo rullst make:omni` first."
                .red()
        );
        std::process::exit(1);
    }

    let platform = target.unwrap_or("desktop");

    match platform {
        "desktop" => run_desktop(omni_dir),
        "android" | "ios" => run_mobile(platform, omni_dir),
        _ => {
            println!(
                "{}",
                format!(
                    "❌ Error: Unknown platform '{}'. Supported: desktop, android, ios",
                    platform
                )
                .red()
            );
            std::process::exit(1);
        }
    }
}

fn run_desktop(omni_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut child = std::process::Command::new("cargo")
        .arg("run")
        .arg("-q")
        .current_dir(omni_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;

    let stdout = child.stdout.take().ok_or("Failed to open stdout")?;

    let launched = with_spinner(
        "🚀 Soon the Omni window will automatically open...",
        move || {
            let reader = BufReader::new(stdout);
            let mut ok = false;
            for l in reader.lines().map_while(Result::ok) {
                if l.contains("Launching Omni interface...")
                    || l.contains("Launching Tauri interface...")
                {
                    ok = true;
                    break;
                }
            }
            ok
        },
    );

    if launched {
        println!("{}", "✅ Omni window launched successfully!".green().bold());
    }

    let status = child.wait()?;
    if !status.success() {
        std::process::exit(1);
    }
    Ok(())
}

fn run_mobile(platform: &str, omni_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting Rullst backend server in background...");
    let backend = std::process::Command::new("cargo")
        .arg("run")
        .arg("-q")
        .current_dir(".")
        .spawn()?;
    let backend_guard = ChildGuard(backend);

    println!("⏳ Waiting for backend to bind...");
    for _ in 0..60 {
        if std::net::TcpStream::connect("127.0.0.1:3000").is_ok() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    println!(
        "📱 Starting Omni mobile client ({}) via Omni Engine...",
        platform
    );

    if platform == "android" {
        println!(
            "🔗 Setting up Android USB/Emulator port forwarding (adb reverse tcp:3000 tcp:3000)..."
        );
        let adb_cmd = if cfg!(windows) {
            if let Ok(android_home) = std::env::var("ANDROID_HOME") {
                format!("{}\\platform-tools\\adb.exe", android_home)
            } else {
                "adb".to_string()
            }
        } else {
            "adb".to_string()
        };

        let _ = std::process::Command::new(&adb_cmd)
            .args(["reverse", "tcp:3000", "tcp:3000"])
            .status()
            .or_else(|_| {
                std::process::Command::new("adb")
                    .args(["reverse", "tcp:3000", "tcp:3000"])
                    .status()
            });
    }

    match get_tauri_command(omni_dir) {
        Ok(mut tauri_cmd) => {
            tauri_cmd.arg(platform).arg("dev").current_dir(omni_dir);
            let status = tauri_cmd.status()?;

            drop(backend_guard);
            if !status.success() {
                std::process::exit(1);
            }
        }
        Err(e) => {
            println!(
                "{}",
                format!("❌ Error: Omni CLI is required for mobile target: {}", e).red()
            );
            std::process::exit(1);
        }
    }
    Ok(())
}

pub fn get_tauri_command(
    omni_dir: &Path,
) -> Result<std::process::Command, Box<dyn std::error::Error>> {
    let local_cli = omni_dir
        .join("node_modules")
        .join("@tauri-apps")
        .join("cli")
        .join("package.json");
    if local_cli.is_file() {
        let mut command = if cfg!(windows) {
            std::process::Command::new("npm.cmd")
        } else {
            std::process::Command::new("npm")
        };
        command.args(["exec", "--offline", "--", "tauri"]);
        return Ok(command);
    }

    let has_tauri_cli = std::process::Command::new("cargo")
        .arg("tauri")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if has_tauri_cli {
        let mut cmd = std::process::Command::new("cargo");
        cmd.arg("tauri");
        return Ok(cmd);
    }

    Err(
        "Tauri CLI is unavailable; run `npm install` in omni-app or install a reviewed cargo-tauri version"
            .into(),
    )
}
