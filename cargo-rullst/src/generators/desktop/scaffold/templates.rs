use std::fs;
use std::path::Path;

pub(super) fn write_omni_files(
    omni_dir: &Path,
    src_dir: &Path,
    backend_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let backend_literal = serde_json::to_string(backend_url)?;

    fs::write(
        src_dir.join("index.html"),
        r#"<!DOCTYPE html>
<html>
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <script defer src="redirect.js"></script>
  </head>
  <body style="background-color: #1a1a1a; color: white; display: flex; justify-content: center; align-items: center; height: 100vh; font-family: sans-serif;">
    <h2 style="animation: pulse 1.5s infinite;">Starting Omni Engine...</h2>
    <style>@keyframes pulse { 0% { opacity: 0.5; } 50% { opacity: 1; } 100% { opacity: 0.5; } }</style>
  </body>
</html>"#,
    )?;
    fs::write(
        src_dir.join("redirect.js"),
        format!("const backendUrl = {backend_literal};\nwindow.location.replace(backendUrl);\n"),
    )?;

    fs::write(
        omni_dir.join("package.json"),
        r#"{
  "name": "rullst-omni",
  "version": "1.0.0",
  "private": true,
  "scripts": {
    "tauri": "tauri"
  },
  "devDependencies": {
    "@tauri-apps/cli": "2.11.4"
  }
}
"#,
    )?;

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
tauri-build = { version = "=2.6.3", features = [] }

[dependencies]
tauri = { version = "=2.11.5", features = [] }

[workspace]
"#,
    )?;

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
    "windows": [
      {
        "title": "Rullst Omni",
        "width": 1024,
        "height": 768,
        "resizable": true
      }
    ],
    "security": {
      "csp": "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' asset: http://asset.localhost blob: data:; connect-src ipc: http://ipc.localhost https: http://localhost:3000 http://127.0.0.1:3000 http://10.0.2.2:3000"
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

    fs::write(
        omni_dir.join("build.rs"),
        "fn main() {\n    tauri_build::build();\n}\n",
    )?;
    fs::write(src_dir.join("lib.rs"), GENERATED_RUNTIME)?;
    fs::write(
        src_dir.join("main.rs"),
        "#![cfg_attr(not(debug_assertions), windows_subsystem = \"windows\")]\n\nfn main() {\n    rullst_omni::run();\n}\n",
    )?;
    fs::write(omni_dir.join("README.md"), GENERATED_README)?;

    Ok(())
}

const GENERATED_RUNTIME: &str = r#"
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
                Err(error) => {
                    eprintln!("❌ Failed to start Rullst backend: {error}");
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
"#;

const GENERATED_README: &str = r#"# Rullst Omni (Tauri packaging shell)

This directory is the generated desktop/Android/iOS packaging shell for a
Rullst web application. The backend URL is fixed in `src/redirect.js` when the
shell is scaffolded. Regenerate or review that file when environments change.

## Run locally

```bash
cargo rullst omni desktop
cargo rullst omni android
cargo rullst omni ios
```

Android requires its SDK/NDK; iOS requires macOS and Xcode. For Android emulator
development, scaffold with `--backend-url http://10.0.2.2:3000`. Distributable
mobile applications should use an HTTPS endpoint reachable from the device.

## Distribution boundary

The generated shell and simulator build are packaging foundations, not proof of
App Store acceptance. Before distributing an iOS app:

1. replace `com.rullst.omni` with an application-owned bundle identifier;
2. set the Apple developer team, signing certificate and provisioning profile;
3. replace the generated placeholder icon and complete product metadata;
4. add the app-owned privacy manifest and required usage descriptions;
5. validate the real HTTPS backend, authentication and data-retention policy;
6. test on supported simulators and physical devices, then use TestFlight;
7. archive, notarize/sign where applicable, and submit through App Store Connect.

Native plugins, offline synchronization, signing credentials, privacy answers,
store publication and review remain application responsibilities.
"#;

pub(super) fn generate_icon_source(icons_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::write(
        icons_dir.join("icon.svg"),
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="1024" height="1024" viewBox="0 0 1024 1024">
  <defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop stop-color="#0f172a"/><stop offset="1" stop-color="#2563eb"/></linearGradient></defs>
  <rect width="1024" height="1024" rx="224" fill="url(#g)"/>
  <path d="M280 760V264h244c142 0 230 70 230 190 0 79-43 139-115 168l137 138H610L493 642h-67v118H280zm146-244h91c58 0 91-21 91-62s-33-62-91-62h-91v124z" fill="#f8fafc"/>
</svg>
"##,
    )?;
    Ok(())
}
