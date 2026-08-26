// src/generators/desktop/scaffold.rs — Scaffold Omni (Tauri) packaging for desktop & mobile.

use crate::generators::is_rullst_project;
use colored::*;
use std::fs;
use std::path::Path;

#[cfg_attr(mutants, mutants::skip)]
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

    if !has_desktop && !has_android && !has_ios {
        println!("{}", "⚠️ Warning: No platforms selected (remember to press <Space> to select). Defaulting to Desktop."
            .truecolor(255, 165, 0)
            .bold());
        has_desktop = true;
    }
    let _ = has_desktop;

    // Create Directories
    let omni_dir = Path::new("omni-app");
    let src_dir = omni_dir.join("src");
    let icons_dir = omni_dir.join("icons");

    fs::create_dir_all(omni_dir)?;
    fs::create_dir_all(&src_dir)?;
    fs::create_dir_all(&icons_dir)?;

    write_omni_files(omni_dir, &src_dir)?;
    generate_placeholder_icons(&icons_dir)?;
    run_npm_install(omni_dir);

    if has_android {
        init_mobile_target(omni_dir, "android");
    }

    if has_ios {
        if cfg!(target_os = "macos") {
            init_mobile_target(omni_dir, "ios");
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

fn write_omni_files(omni_dir: &Path, src_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // index.html
    fs::write(
        src_dir.join("index.html"),
        r#"<!DOCTYPE html>
<html>
  <head>
    <meta charset="utf-8">
    <script>window.location.replace("http://localhost:3000");</script>
  </head>
  <body style="background-color: #1a1a1a; color: white; display: flex; justify-content: center; align-items: center; height: 100vh; font-family: sans-serif;">
    <h2 style="animation: pulse 1.5s infinite;">Starting Omni Engine...</h2>
    <style>@keyframes pulse { 0% { opacity: 0.5; } 50% { opacity: 1; } 100% { opacity: 0.5; } }</style>
  </body>
</html>"#,
    )?;

    // package.json
    fs::write(
        omni_dir.join("package.json"),
        r#"{
  "name": "rullst-omni",
  "version": "1.0.0",
  "scripts": {
    "tauri": "npx -y @tauri-apps/cli@^2.0.0"
  }
}
"#,
    )?;

    // Cargo.toml
    fs::write(
        omni_dir.join("Cargo.toml"),
        r#"[package]
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
"#,
    )?;

    // tauri.conf.json
    fs::write(
        omni_dir.join("tauri.conf.json"),
        r#"{
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
"#,
    )?;

    // build.rs
    fs::write(
        omni_dir.join("build.rs"),
        "fn main() {\n    tauri_build::build();\n}\n",
    )?;

    // src/lib.rs
    fs::write(
        src_dir.join("lib.rs"),
        r#"
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
                let Some(exe_dir) = std::env::current_exe()
                    .ok()
                    .and_then(|path| path.parent().map(std::path::Path::to_path_buf))
                else {
                    eprintln!("❌ Failed to resolve the Omni executable directory.");
                    return;
                };
                let server_bin = if cfg!(windows) { "server.exe" } else { "server" };
                Command::new(exe_dir.join(server_bin))
            };

            match cmd.spawn() {
                Ok(child) => {
                    let mut lock = match backend_clone.lock() {
                        Ok(lock) => lock,
                        Err(poisoned) => {
                            eprintln!("⚠️ Backend process state was poisoned; recovering ownership.");
                            poisoned.into_inner()
                        }
                    };
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

        if let Err(error) = tauri::Builder::default()
            .on_window_event(move |_window, event| {
                if let tauri::WindowEvent::Destroyed = event {
                    println!("🛑 Omni window closed. Shutting down Rullst backend...");
                    let mut lock = match backend_for_cleanup.lock() {
                        Ok(lock) => lock,
                        Err(poisoned) => {
                            eprintln!("⚠️ Backend process state was poisoned; recovering ownership.");
                            poisoned.into_inner()
                        }
                    };
                    if let Some(mut child) = lock.take() {
                        let _ = child.kill();
                        println!("✅ Rullst backend terminated.");
                    }
                }
            })
            .run(tauri::generate_context!())
        {
            eprintln!("❌ Tauri application failed: {error}");
        }
    }

    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        if let Err(error) = tauri::Builder::default().run(tauri::generate_context!()) {
            eprintln!("❌ Tauri mobile application failed: {error}");
        }
    }
}
"#,
    )?;

    // src/main.rs
    fs::write(
        src_dir.join("main.rs"),
        "#![cfg_attr(not(debug_assertions), windows_subsystem = \"windows\")]\n\nfn main() {\n    rullst_omni::run();\n}\n",
    )?;

    // README.md
    fs::write(
        omni_dir.join("README.md"),
        r#"# Rullst Omni (Tauri-powered Desktop & Mobile App wrapper)

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
"#,
    )?;

    Ok(())
}

fn generate_placeholder_icons(icons_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
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

    Ok(())
}

fn run_npm_install(omni_dir: &Path) {
    let has_npm = if cfg!(windows) {
        std::process::Command::new("npm.cmd")
            .args(["--version"])
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
            let mut c = std::process::Command::new("npm.cmd");
            c.args(["install"]);
            c
        } else {
            let mut c = std::process::Command::new("npm");
            c.arg("install");
            c
        };
        let _ = npm_install.current_dir(omni_dir).status();
    }
}

fn init_mobile_target(omni_dir: &Path, platform: &str) {
    if platform == "android" {
        println!("🤖 Initializing Android support folder inside 'omni-app/'...");
        println!("{}", "💡 Tip: If Omni asks to install Android command line tools or NDK, typing 'y' (yes) is highly recommended!"
            .truecolor(255, 165, 0)
            .bold());
    } else {
        println!("🍎 Initializing iOS support folder inside 'omni-app/'...");
    }

    match super::runner::get_tauri_command(omni_dir) {
        Ok(mut tauri_cmd) => {
            let _ = tauri_cmd
                .arg(platform)
                .arg("init")
                .current_dir(omni_dir)
                .status();
        }
        Err(e) => {
            println!(
                "{}",
                format!(
                    "⚠️ Warning: Could not initialize {} target support: {}",
                    platform, e
                )
                .yellow()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn generated_omni_runtime_is_valid_and_zero_panic() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "rullst-omni-zero-panic-{}-{unique}",
            std::process::id()
        ));
        let src = root.join("src");

        fs::create_dir_all(&src).expect("temporary Omni source directory");
        write_omni_files(&root, &src).expect("Omni scaffold files");
        let generated = fs::read_to_string(src.join("lib.rs")).expect("generated Omni runtime");

        syn::parse_file(&generated).expect("generated Omni runtime must parse as Rust");
        assert!(!generated.contains(".unwrap("));
        assert!(!generated.contains(".expect("));
        assert!(!generated.contains("panic!("));
        assert!(!generated.contains("todo!("));
        assert!(!generated.contains("unimplemented!("));

        let _ = fs::remove_dir_all(root);
    }
}
