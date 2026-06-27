// src/generators/desktop.rs — Omni packaging sub-systems scaffolding.

use crate::generators::is_rullst_project;
use colored::*;
use std::fs;
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

pub fn scaffold_omni_system() -> Result<(), Box<dyn std::error::Error>> {
    let theme = dialoguer::theme::ColorfulTheme::default();

    if !is_rullst_project() {
        println!(
            "{}{}",
            "❌ Error: This command must be executed in the root of a valid Rullst project."
                .red()
                .bold(),
            "\nMake sure the current folder contains a 'Cargo.toml' file with a 'rullst' dependency."
                .yellow()
        );
        std::process::exit(1);
    }

    println!(
        "{}",
        "🖥️ Starting scaffolding of Rullst Omni packaging system..."
            .cyan()
            .bold()
    );

    // 1. Select platforms
    let platforms = [
        "Desktop (Windows/Mac/Linux)".to_string(),
        format!(
            "Android {}",
            "(Requires Android Studio SDK)".truecolor(255, 165, 0)
        ),
        format!("iOS {}", "(iPhone/iPad - Requires macOS)".red()),
    ];

    let selections = match dialoguer::MultiSelect::with_theme(&theme)
        .with_prompt(format!("{}", "⚠️ Select target platforms for Rullst Omni (Press <Space> to select, <Enter> to confirm)".truecolor(255, 165, 0).bold()))
        .items(&platforms[..])
        .defaults(&[true, false, false])
        .interact() {
            Ok(sel) => sel,
            Err(_) => {
                println!("{}", "⚠️ Warning: Non-interactive terminal detected. Defaulting to Desktop target.".yellow());
                vec![0]
            }
        };

    let mut has_desktop = false;
    let mut has_android = false;
    let mut has_ios = false;

    for &selection in &selections {
        match selection {
            0 => has_desktop = true,
            1 => has_android = true,
            2 => has_ios = true,
            _ => {}
        }
    }

    // Ensure at least one platform is selected
    if !has_desktop && !has_android && !has_ios {
        println!("{}", "⚠️ Warning: No platforms selected (remember to press <Space> to select). Defaulting to Desktop."
            .truecolor(255, 165, 0)
            .bold());
        has_desktop = true;
    }
    let _ = has_desktop;

    // 2. Create Directories
    let omni_dir = Path::new("omni-app");
    let src_dir = omni_dir.join("src");
    let icons_dir = omni_dir.join("icons");

    fs::create_dir_all(&omni_dir)?;
    fs::create_dir_all(&src_dir)?;
    fs::create_dir_all(&icons_dir)?;

    // 3. Write index.html
    let index_html = r#"<!DOCTYPE html>
<html>
  <head>
    <meta charset="utf-8">
    <script>window.location.replace("http://localhost:3000");</script>
  </head>
  <body style="background-color: #1a1a1a; color: white; display: flex; justify-content: center; align-items: center; height: 100vh; font-family: sans-serif;">
    <h2 style="animation: pulse 1.5s infinite;">Starting Omni Engine...</h2>
    <style>@keyframes pulse { 0% { opacity: 0.5; } 50% { opacity: 1; } 100% { opacity: 0.5; } }</style>
  </body>
</html>"#;
    fs::write(src_dir.join("index.html"), index_html)?;

    // 4. Write package.json
    let package_json = r#"{
  "name": "rullst-omni",
  "version": "1.0.0",
  "scripts": {
    "tauri": "npx -y @tauri-apps/cli@^2.0.0"
  }
}
"#;
    fs::write(omni_dir.join("package.json"), package_json)?;

    // 4. Write Cargo.toml (with Tauri 2.11.2 and Tauri-build 2.6.2)
    let cargo_toml = r#"[package]
name = "rullst-omni"
version = "0.1.0"
description = "Rullst Omni Application"
authors = ["Rullst Developer"]
edition = "2021"

[lib]
name = "rullst_omni"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2.6.2", features = [] }

[dependencies]
tauri = { version = "2.11.2", features = [] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.13", features = ["blocking"] }

[workspace]
"#;
    fs::write(omni_dir.join("Cargo.toml"), cargo_toml)?;

    // 5. Write tauri.conf.json
    let tauri_conf = r#"{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "RullstOmni",
  "version": "0.1.0",
  "identifier": "com.rullst.omni",
  "build": {
    "frontendDist": "src"
  },
  "app": {
    "withGlobalTauri": true,
    "windows": [
      {
        "title": "Rullst Omni",
        "width": 1024,
        "height": 768,
        "resizable": true
      }
    ],
    "security": {
      "csp": null
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ]
  }
}
"#;
    fs::write(omni_dir.join("tauri.conf.json"), tauri_conf)?;

    // 6. Write build.rs
    let build_rs = r#"fn main() {
    tauri_build::build();
}
"#;
    fs::write(omni_dir.join("build.rs"), build_rs)?;

    // 7. Write src/lib.rs and src/main.rs (Process Orchestrator with conditional mobile compilation)
    let lib_rs = r#"
#[cfg(not(any(target_os = "android", target_os = "ios")))]
use std::process::{Command, Child};
#[cfg(not(any(target_os = "android", target_os = "ios")))]
use std::net::TcpStream;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
use std::time::Duration;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
use std::thread;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
use std::sync::{Arc, Mutex};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        let backend_process = Arc::new(Mutex::new(None::<Child>));
        let backend_clone = Arc::clone(&backend_process);

        thread::spawn(move || {
            println!("🚀 Starting Rullst backend server...");
            
            let mut cmd = if std::path::Path::new("../Cargo.toml").exists() {
                let mut c = Command::new("cargo");
                c.arg("run").arg("-q").current_dir("..");
                c
            } else {
                let exe_dir = std::env::current_exe().unwrap().parent().unwrap().to_path_buf();
                let server_bin = if cfg!(windows) { "server.exe" } else { "server" };
                Command::new(exe_dir.join(server_bin))
            };

            match cmd.spawn() {
                Ok(child) => {
                    let mut lock = backend_clone.lock().unwrap();
                    *lock = Some(child);
                }
                Err(e) => {
                    eprintln!("❌ Failed to start Rullst backend: {}", e);
                }
            }
        });

        println!("⏳ Waiting for Rullst server to bind on port 3000...");
        let poll_interval = Duration::from_millis(100);
        let timeout = Duration::from_secs(30);
        let start_time = std::time::Instant::now();
        let mut connected = false;

        while start_time.elapsed() < timeout {
            if TcpStream::connect("127.0.0.1:3000").is_ok() {
                connected = true;
                break;
            }
            thread::sleep(poll_interval);
        }

        if connected {
            println!("✅ Rullst server is ready! Launching Omni interface...");
        } else {
            eprintln!("⚠️ Timeout waiting for port 3000 to open. Attempting window launch anyway...");
        }

        let backend_for_cleanup = Arc::clone(&backend_process);

        tauri::Builder::default()
            .on_window_event(move |_window, event| {
                if let tauri::WindowEvent::Destroyed = event {
                    println!("🛑 Omni window closed. Shutting down Rullst backend...");
                    let mut lock = backend_for_cleanup.lock().unwrap();
                    if let Some(mut child) = lock.take() {
                        let _ = child.kill();
                        println!("✅ Rullst backend terminated.");
                    }
                }
            })
            .run(tauri::generate_context!())
            .expect("error while running tauri application");
    }

    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        // On mobile, the backend server must be running on the host machine.
        // We simply launch standard tauri runtime here.
        tauri::Builder::default()
            .run(tauri::generate_context!())
            .expect("error while running tauri application on mobile");
    }
}
"#;
    fs::write(src_dir.join("lib.rs"), lib_rs)?;

    let main_rs = r#"#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    rullst_omni::run();
}
"#;
    fs::write(src_dir.join("main.rs"), main_rs)?;

    // 8. Write README.md
    let readme_md = r#"# Rullst Omni (Tauri-powered Desktop & Mobile App wrapper)

This directory contains the cross-platform Tauri packaging wrapper for your Rullst application.

## Getting Started

### Desktop (Windows, macOS, Linux)
To run the desktop application:
```bash
cargo rullst omni
# or
cargo rullst omni desktop
```

### Android
To run on an Android emulator or physical device:
1. Make sure you have the Android SDK, NDK, and emulator configured.
2. Ensure the Rullst backend server is running:
   ```bash
   cargo rullst dev
   ```
3. Run the Android client:
   ```bash
   cargo rullst omni android
   ```

> [!IMPORTANT]
> **Android Networking Note:** By default, Android emulators cannot access the host machine's `localhost`.
> You need to update your `tauri.conf.json` or redirects in `index.html` to point to `http://10.0.2.2:3000` (which redirects to your host's localhost:3000) or your computer's local IP address (e.g., `http://192.168.1.50:3000`).

### iOS (macOS required)
To run on an iOS simulator or device:
1. Make sure Xcode is installed.
2. Ensure the Rullst backend server is running:
   ```bash
   cargo rullst dev
   ```
3. Run the iOS client:
   ```bash
   cargo rullst omni ios
   ```
"#;
    fs::write(omni_dir.join("README.md"), readme_md)?;

    // 9. Generate icons to prevent Tauri compile errors
    let png_bytes: &[u8] = &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f,
        0x15, 0xc4, 0x89, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x44, 0x41, 0x54, 0x78, 0xda, 0x63, 0x60,
        0x60, 0x60, 0x60, 0x00, 0x00, 0x00, 0x05, 0x00, 0x01, 0x7a, 0xa8, 0x57, 0x50, 0x00, 0x00,
        0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ];

    fs::write(icons_dir.join("32x32.png"), png_bytes)?;
    fs::write(icons_dir.join("128x128.png"), png_bytes)?;
    fs::write(icons_dir.join("128x128@2x.png"), png_bytes)?;

    // Construct valid ICO
    let mut ico_bytes = Vec::new();
    ico_bytes.extend_from_slice(&[0x00, 0x00]);
    ico_bytes.extend_from_slice(&[0x01, 0x00]);
    ico_bytes.extend_from_slice(&[0x01, 0x00]);
    ico_bytes.push(0x01);
    ico_bytes.push(0x01);
    ico_bytes.push(0x00);
    ico_bytes.push(0x00);
    ico_bytes.extend_from_slice(&[0x01, 0x00]);
    ico_bytes.extend_from_slice(&[0x20, 0x00]);
    let png_len = png_bytes.len() as u32;
    ico_bytes.extend_from_slice(&png_len.to_le_bytes());
    ico_bytes.extend_from_slice(&22u32.to_le_bytes());
    ico_bytes.extend_from_slice(png_bytes);
    fs::write(icons_dir.join("icon.ico"), &ico_bytes)?;

    // Construct valid ICNS
    let mut icns_bytes = Vec::new();
    icns_bytes.extend_from_slice(&[0x69, 0x63, 0x6e, 0x73]);
    let total_icns_len = (8 + 8 + png_bytes.len()) as u32;
    icns_bytes.extend_from_slice(&total_icns_len.to_be_bytes());
    icns_bytes.extend_from_slice(&[0x69, 0x63, 0x30, 0x37]);
    let chunk_len = (8 + png_bytes.len()) as u32;
    icns_bytes.extend_from_slice(&chunk_len.to_be_bytes());
    icns_bytes.extend_from_slice(png_bytes);
    fs::write(icons_dir.join("icon.icns"), &icns_bytes)?;

    // 9.5. Run npm install to populate node_modules with @tauri-apps/cli
    let has_npm = if cfg!(windows) {
        std::process::Command::new("cmd")
            .args(&["/C", "npm --version"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    } else {
        std::process::Command::new("npm")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    };

    if has_npm {
        println!("📦 Installing project dependencies via npm...");
        let mut npm_install = if cfg!(windows) {
            let mut c = std::process::Command::new("cmd");
            c.args(&["/C", "npm", "install"]);
            c
        } else {
            let mut c = std::process::Command::new("npm");
            c.arg("install");
            c
        };
        let _ = npm_install.current_dir(omni_dir).status();
    }

    // 10. Check and resolve Tauri CLI for mobile target auto-initialization
    if has_android {
        println!("🤖 Initializing Android support folder inside 'omni-app/'...");
        println!("{}", "💡 Tip: If Omni asks to install Android command line tools or NDK, typing 'y' (yes) is highly recommended!"
            .truecolor(255, 165, 0)
            .bold());
        match get_tauri_command(omni_dir) {
            Ok(mut tauri_cmd) => {
                let _ = tauri_cmd
                    .arg("android")
                    .arg("init")
                    .current_dir(omni_dir)
                    .status();
            }
            Err(e) => {
                println!(
                    "{}",
                    format!(
                        "⚠️ Warning: Could not initialize Android target support: {}",
                        e
                    )
                    .yellow()
                );
            }
        }
    }

    if has_ios {
        if cfg!(target_os = "macos") {
            println!("🍎 Initializing iOS support folder inside 'omni-app/'...");
            match get_tauri_command(omni_dir) {
                Ok(mut tauri_cmd) => {
                    let _ = tauri_cmd
                        .arg("ios")
                        .arg("init")
                        .current_dir(omni_dir)
                        .status();
                }
                Err(e) => {
                    println!(
                        "{}",
                        format!("⚠️ Warning: Could not initialize iOS target support: {}", e)
                            .yellow()
                    );
                }
            }
        } else {
            println!(
                "{}",
                "⚠️ Warning: iOS initialization requires a macOS host. Skipping iOS setup."
                    .truecolor(255, 165, 0)
                    .bold()
            );
        }
    }

    println!(
        "{}\n\n{}",
        "✅ Rullst Omni template successfully generated in 'omni-app/'!"
            .green()
            .bold(),
        "To start developing, run:".cyan()
    );

    if has_desktop {
        println!("  {}", "cargo rullst omni desktop".white().bold());
    }
    if has_android {
        println!("  {}", "cargo rullst omni android".white().bold());
    }
    if has_ios {
        println!("  {}", "cargo rullst omni ios".white().bold());
    }

    Ok(())
}

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
        "desktop" => {
            let mut child = std::process::Command::new("cargo")
                .arg("run")
                .arg("-q")
                .current_dir(omni_dir)
                .stdout(Stdio::piped())
                .stderr(Stdio::inherit())
                .spawn()
                .expect("Failed to execute cargo run");

            let stdout = child.stdout.take().expect("Failed to open stdout");

            let launched = crate::ui::components::with_spinner(
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

            let status = child.wait().expect("Failed to wait on child");
            if !status.success() {
                std::process::exit(1);
            }
        }
        "android" | "ios" => {
            println!("🚀 Starting Rullst backend server in background...");
            let backend = std::process::Command::new("cargo")
                .arg("run")
                .arg("-q")
                .current_dir(".") // Running in root
                .spawn()
                .expect("Failed to spawn Rullst backend");
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
                    .args(&["reverse", "tcp:3000", "tcp:3000"])
                    .status()
                    .or_else(|_| {
                        std::process::Command::new("adb")
                            .args(&["reverse", "tcp:3000", "tcp:3000"])
                            .status()
                    });
            }

            match get_tauri_command(omni_dir) {
                Ok(mut tauri_cmd) => {
                    tauri_cmd.arg(platform).arg("dev").current_dir(omni_dir);
                    let status = tauri_cmd.status().expect("Failed to run cargo tauri dev");

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
        }
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

    Ok(())
}

fn get_tauri_command(
    _omni_dir: &Path,
) -> Result<std::process::Command, Box<dyn std::error::Error>> {
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

    let has_npx = if cfg!(windows) {
        std::process::Command::new("cmd")
            .args(&["/C", "npx --version"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    } else {
        std::process::Command::new("npx")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    };

    if has_npx {
        let cmd = if cfg!(windows) {
            let mut c = std::process::Command::new("cmd");
            c.args(&["/C", "npx", "--yes", "@tauri-apps/cli"]);
            c
        } else {
            let mut c = std::process::Command::new("npx");
            c.args(&["--yes", "@tauri-apps/cli"]);
            c
        };
        return Ok(cmd);
    }

    // Install tauri-cli globally via cargo install tauri-cli
    println!("{}", "📦 Omni background tools not found. Installing globally via Cargo (this may take a few minutes)..."
        .truecolor(255, 165, 0)
        .bold());

    let installed =
        crate::ui::components::with_spinner("🚀 Installing Omni background tools...", || {
            std::process::Command::new("cargo")
                .args(&["install", "tauri-cli"])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        });

    if installed {
        println!(
            "{}",
            "✅ Omni background tools installed successfully!"
                .green()
                .bold()
        );
        let mut cmd = std::process::Command::new("cargo");
        cmd.arg("tauri");
        Ok(cmd)
    } else {
        Err("Failed to install tauri-cli automatically".into())
    }
}
