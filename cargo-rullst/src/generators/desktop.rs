// src/generators/desktop.rs — Desktop packaging and Omni-channel sub-systems scaffolding.

use crate::generators::is_rullst_project;
use colored::*;
use std::fs;
use std::path::Path;

pub fn scaffold_desktop_system() -> Result<(), Box<dyn std::error::Error>> {
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
        "🖥️ Starting scaffolding of Rullst desktop packaging system (Tauri)..."
            .cyan()
            .bold()
    );

    // 1. Create Directories
    let src_tauri_dir = Path::new("src-tauri");
    let src_dir = src_tauri_dir.join("src");
    let icons_dir = src_tauri_dir.join("icons");

    fs::create_dir_all(&src_tauri_dir)?;
    fs::create_dir_all(&src_dir)?;
    fs::create_dir_all(&icons_dir)?;

    // 2. Write Cargo.toml
    let cargo_toml = r#"[package]
name = "rullst-desktop"
version = "0.1.0"
description = "Rullst Desktop Application"
authors = ["Rullst Developer"]
edition = "2021"

[build-dependencies]
tauri-build = { version = "1.5" }

[dependencies]
tauri = { version = "1.5", features = ["shell-open"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.11", features = ["blocking"] }
"#;
    fs::write(src_tauri_dir.join("Cargo.toml"), cargo_toml)?;

    // 3. Write tauri.conf.json
    let tauri_conf = r#"{
  "build": {
    "beforeDevCommand": "",
    "beforeBuildCommand": "",
    "devPath": "http://localhost:3000",
    "distDir": "http://localhost:3000"
  },
  "package": {
    "productName": "RullstDesktop",
    "version": "0.1.0"
  },
  "tauri": {
    "allowlist": {
      "all": false
    },
    "bundle": {
      "active": true,
      "category": "DeveloperTool",
      "copyright": "",
      "deb": {
        "depends": []
      },
      "externalBin": [],
      "icon": [
        "icons/32x32.png",
        "icons/128x128.png",
        "icons/128x128@2x.png",
        "icons/icon.icns",
        "icons/icon.ico"
      ],
      "identifier": "com.rullst.desktop",
      "longDescription": "",
      "macOS": {
        "entitlements": null,
        "exceptionDomain": "",
        "frameworks": [],
        "providerBundleIdentifier": null,
        "signingIdentity": null
      },
      "resources": [],
      "shortDescription": "",
      "targets": "all",
      "windows": {
        "certificateThumbprint": null,
        "digestAlgorithm": "sha256",
        "timestampUrl": ""
      }
    },
    "security": {
      "csp": null
    },
    "windows": [
      {
        "fullscreen": false,
        "height": 768,
        "resizable": true,
        "title": "Rullst Hyper Desktop",
        "width": 1024
      }
    ]
  }
}
"#;
    fs::write(src_tauri_dir.join("tauri.conf.json"), tauri_conf)?;

    // 4. Write build.rs
    let build_rs = r#"fn main() {
    tauri_build::build();
}
"#;
    fs::write(src_tauri_dir.join("build.rs"), build_rs)?;

    // 5. Write src/main.rs (Process Orchester)
    let main_rs = r#"#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::{Command, Child};
use std::net::TcpStream;
use std::time::Duration;
use std::thread;
use std::sync::{Arc, Mutex};
use tauri::Manager;

fn main() {
    let backend_process = Arc::new(Mutex::new(None::<Child>));
    let backend_clone = Arc::clone(&backend_process);

    thread::spawn(move || {
        println!("🚀 Starting Rullst backend server...");
        
        let mut cmd = if std::path::Path::new("../Cargo.toml").exists() {
            let mut c = Command::new("cargo");
            c.arg("run").current_dir("..");
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
        println!("✅ Rullst server is ready! Launching Tauri interface...");
    } else {
        eprintln!("⚠️ Timeout waiting for port 3000 to open. Attempting window launch anyway...");
    }

    let backend_for_cleanup = Arc::clone(&backend_process);

    tauri::Builder::default()
        .on_window_event(move |event| {
            if let tauri::WindowEvent::Destroyed = event.event() {
                println!("🛑 Tauri window closed. Shutting down Rullst backend...");
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
"#;
    fs::write(src_dir.join("main.rs"), main_rs)?;

    // 6. Generate icons to prevent Tauri compile errors
    // PNG 1x1 transparent
    let png_bytes: &[u8] = &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4,
        0x89, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x44, 0x41, 0x54, 0x78, 0xda, 0x63, 0x60, 0x00, 0x00, 0x00,
        0x02, 0x00, 0x01, 0x73, 0x0d, 0x8b, 0xb4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae,
        0x42, 0x60, 0x82
    ];

    fs::write(icons_dir.join("32x32.png"), png_bytes)?;
    fs::write(icons_dir.join("128x128.png"), png_bytes)?;
    fs::write(icons_dir.join("128x128@2x.png"), png_bytes)?;

    // Construct valid minimal ICO embedding the 1x1 PNG
    let mut ico_bytes = Vec::new();
    ico_bytes.extend_from_slice(&[0x00, 0x00]); // Reserved
    ico_bytes.extend_from_slice(&[0x01, 0x00]); // Type (1 = ICO)
    ico_bytes.extend_from_slice(&[0x01, 0x00]); // Number of images (1)
    
    // Directory entry (16 bytes)
    ico_bytes.push(0x01); // Width (1 pixel)
    ico_bytes.push(0x01); // Height (1 pixel)
    ico_bytes.push(0x00); // Color count
    ico_bytes.push(0x00); // Reserved
    ico_bytes.extend_from_slice(&[0x01, 0x00]); // Color planes (1)
    ico_bytes.extend_from_slice(&[0x20, 0x00]); // Bits per pixel (32)
    
    let png_len = png_bytes.len() as u32;
    ico_bytes.extend_from_slice(&png_len.to_le_bytes()); // Size of image data
    ico_bytes.extend_from_slice(&22u32.to_le_bytes());   // Offset of image data
    
    ico_bytes.extend_from_slice(png_bytes);
    fs::write(icons_dir.join("icon.ico"), &ico_bytes)?;

    // Construct valid minimal ICNS embedding the 1x1 PNG under "ic07" (128x128 size key)
    let mut icns_bytes = Vec::new();
    icns_bytes.extend_from_slice(&[0x69, 0x63, 0x6e, 0x73]); // Magic "icns"
    
    let total_icns_len = (8 + 8 + png_bytes.len()) as u32;
    icns_bytes.extend_from_slice(&total_icns_len.to_be_bytes()); // Total length (big endian)
    
    icns_bytes.extend_from_slice(&[0x69, 0x63, 0x30, 0x37]); // OSType "ic07" (128x128 icon)
    let chunk_len = (8 + png_bytes.len()) as u32;
    icns_bytes.extend_from_slice(&chunk_len.to_be_bytes()); // Chunk length (big endian)
    
    icns_bytes.extend_from_slice(png_bytes);
    fs::write(icons_dir.join("icon.icns"), &icns_bytes)?;

    println!(
        "{}",
        "✅ Rullst Hyper desktop template successfully generated in 'src-tauri/'!"
            .green()
            .bold()
    );

    Ok(())
}

pub fn scaffold_omni_system() -> Result<(), Box<dyn std::error::Error>> {
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
        "📱 Starting scaffolding of Rullst Omni multi-platform frontend (Dioxus)..."
            .cyan()
            .bold()
    );

    // 1. Create Directories
    let omni_dir = Path::new("omni-app");
    let src_dir = omni_dir.join("src");

    fs::create_dir_all(&omni_dir)?;
    fs::create_dir_all(&src_dir)?;

    // 2. Write Cargo.toml
    let cargo_toml = r#"[package]
name = "omni-app"
version = "0.1.0"
authors = ["Rullst Developer"]
edition = "2021"

[dependencies]
dioxus = { version = "0.7", features = ["desktop"] }
reqwest = { version = "0.11", features = ["json"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1", features = ["full"] }
"#;
    fs::write(omni_dir.join("Cargo.toml"), cargo_toml)?;

    // 3. Write src/main.rs
    let main_rs = r##"#![allow(non_snake_case)]
use dioxus::prelude::*;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
struct BackendStatus {
    version: String,
    status: String,
    uptime: String,
}

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut backend_data = use_signal(|| None::<BackendStatus>);

    let mut fetch_status = move |_| {
        spawn(async move {
            let client = reqwest::Client::new();
            match client.get("http://localhost:3000/api/status").send().await {
                Ok(res) => {
                    if let Ok(data) = res.json::<BackendStatus>().await {
                        backend_data.set(Some(data));
                    }
                }
                Err(_) => {
                    backend_data.set(Some(BackendStatus {
                        version: "1.0.5".to_string(),
                        status: "Running (Offline Simulation)".to_string(),
                        uptime: "2h 45m".to_string(),
                    }));
                }
            }
        });
    };

    use_future(move || async move {
        let client = reqwest::Client::new();
        match client.get("http://localhost:3000/api/status").send().await {
            Ok(res) => {
                if let Ok(data) = res.json::<BackendStatus>().await {
                    backend_data.set(Some(data));
                }
            }
            Err(_) => {
                backend_data.set(Some(BackendStatus {
                    version: "1.0.5".to_string(),
                    status: "Running (Offline Simulation)".to_string(),
                    uptime: "2h 45m".to_string(),
                }));
            }
        }
    });

    rsx! {
        style { {include_str!("./style.css")} }
        
        div { class: "app-container",
            div { class: "glow-circle glow-1" }
            div { class: "glow-circle glow-2" }
            
            div { class: "glass-card",
                header { class: "header-container",
                    div { class: "logo-group",
                        span { class: "logo-glow", "R" }
                        h1 { "Rullst "; span { class: "gradient-text", "Omni" } }
                    }
                    span { class: "badge", "v1.0.5 - Free Enterprise" }
                }

                div { class: "main-grid",
                    div { class: "sidebar-panel",
                        h3 { "System Status" }
                        div { class: "status-indicator active",
                            div { class: "ping-dot" }
                            span { "Connected to Dual-Engine Backend" }
                        }
                        
                        div { class: "stats-list",
                            div { class: "stat-item",
                                span { class: "stat-label", "Backend Version:" }
                               span { class: "stat-value", 
                                    if let Some(ref data) = *backend_data.read() {
                                        "{data.version}"
                                    } else {
                                        "Fetching..."
                                    }
                                }
                            }
                            div { class: "stat-item",
                                span { class: "stat-label", "Engine State:" }
                                span { class: "stat-value state-ok", 
                                    if let Some(ref data) = *backend_data.read() {
                                        "{data.status}"
                                    } else {
                                        "Connecting..."
                                    }
                                }
                            }
                            div { class: "stat-item",
                                span { class: "stat-label", "API Uptime:" }
                                span { class: "stat-value", 
                                    if let Some(ref data) = *backend_data.read() {
                                        "{data.uptime}"
                                    } else {
                                        "..."
                                    }
                                }
                            }
                        }

                        button { 
                            class: "primary-btn",
                            onclick: fetch_status,
                            "Refresh Backend Link"
                        }
                    }

                    div { class: "content-panel",
                        h2 { "Multi-Platform Frontend Engine" }
                        p { class: "panel-desc",
                            "Rullst Omni connects your Axum backend to high-fidelity user experiences across iOS, Android, and Desktop using the Dioxus renderer."
                        }

                        div { class: "cards-container",
                            div { class: "feature-card",
                                h4 { "⚡ Rullst Hyper" }
                                p { "Server-side HTMX rendering for extreme lightweight speed and zero Client Wasm overhead." }
                            }
                            div { class: "feature-card highlighted",
                                h4 { "📱 Rullst Omni" }
                                p { "Interactive, cross-compiled native Rust components with instant state reactivity." }
                            }
                        }
                    }
                }

                footer { class: "footer-container",
                    span { "Rullst Framework © 2026" }
                    span { class: "footer-link", "rullst.dev" }
                }
            }
        }
    }
}
"##;
    fs::write(src_dir.join("main.rs"), main_rs)?;

    // 4. Write src/style.css
    let style_css = r#"* {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
    font-family: 'Outfit', -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
}

body, html {
    background-color: #030712;
    color: #f3f4f6;
    overflow: hidden;
    height: 100vh;
    width: 100vw;
}

.app-container {
    position: relative;
    width: 100%;
    height: 100%;
    display: flex;
    justify-content: center;
    align-items: center;
    background: radial-gradient(circle at 50% 50%, #0c1020 0%, #030712 100%);
    overflow: hidden;
}

.glow-circle {
    position: absolute;
    border-radius: 50%;
    filter: blur(100px);
    opacity: 0.3;
    z-index: 1;
    animation: pulse 10s infinite alternate;
}

.glow-1 {
    width: 400px;
    height: 400px;
    background: #6366f1;
    top: -100px;
    left: -100px;
}

.glow-2 {
    width: 450px;
    height: 450px;
    background: #06b6d4;
    bottom: -150px;
    right: -150px;
    animation-delay: 5s;
}

@keyframes pulse {
    0% { transform: scale(1) translate(0, 0); opacity: 0.2; }
    100% { transform: scale(1.2) translate(30px, 30px); opacity: 0.4; }
}

.glass-card {
    position: relative;
    z-index: 10;
    width: 90%;
    max-width: 960px;
    height: 80%;
    max-height: 600px;
    background: rgba(17, 24, 39, 0.65);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 24px;
    display: flex;
    flex-direction: column;
    box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.5), 0 0 40px rgba(99, 102, 241, 0.1);
    overflow: hidden;
}

.header-container {
    padding: 24px 32px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
    display: flex;
    justify-content: space-between;
    align-items: center;
}

.logo-group {
    display: flex;
    align-items: center;
    gap: 12px;
}

.logo-glow {
    width: 38px;
    height: 38px;
    background: linear-gradient(135deg, #6366f1, #06b6d4);
    border-radius: 10px;
    display: flex;
    justify-content: center;
    align-items: center;
    font-weight: 800;
    font-size: 20px;
    color: white;
    box-shadow: 0 0 20px rgba(99, 102, 241, 0.5);
}

h1 {
    font-size: 24px;
    font-weight: 700;
    letter-spacing: -0.5px;
}

.gradient-text {
    background: linear-gradient(90deg, #6366f1, #06b6d4);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
}

.badge {
    padding: 6px 12px;
    border-radius: 9999px;
    background: rgba(99, 102, 241, 0.15);
    border: 1px solid rgba(99, 102, 241, 0.3);
    color: #a5b4fc;
    font-size: 12px;
    font-weight: 600;
}

.main-grid {
    flex: 1;
    display: grid;
    grid-template-columns: 320px 1fr;
    overflow: hidden;
}

.sidebar-panel {
    padding: 32px;
    border-right: 1px solid rgba(255, 255, 255, 0.06);
    background: rgba(10, 15, 30, 0.2);
    display: flex;
    flex-direction: column;
    gap: 24px;
}

.sidebar-panel h3 {
    font-size: 16px;
    font-weight: 600;
    color: #9ca3af;
    text-transform: uppercase;
    letter-spacing: 1px;
}

.status-indicator {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 16px;
    background: rgba(255, 255, 255, 0.03);
    border-radius: 12px;
    border: 1px solid rgba(255, 255, 255, 0.05);
    font-size: 14px;
}

.ping-dot {
    width: 8px;
    height: 8px;
    background-color: #10b981;
    border-radius: 50%;
    box-shadow: 0 0 10px #10b981, 0 0 20px #10b981;
    animation: beacon 1.5s infinite alternate;
}

@keyframes beacon {
    0% { transform: scale(1); opacity: 0.8; }
    100% { transform: scale(1.3); opacity: 1; }
}

.stats-list {
    display: flex;
    flex-direction: column;
    gap: 16px;
}

.stat-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 14px;
}

.stat-label {
    color: #9ca3af;
}

.stat-value {
    font-weight: 600;
    color: #f3f4f6;
}

.state-ok {
    color: #06b6d4;
}

.primary-btn {
    margin-top: auto;
    width: 100%;
    padding: 14px;
    border-radius: 12px;
    border: none;
    background: linear-gradient(90deg, #6366f1, #06b6d4);
    color: white;
    font-weight: 600;
    font-size: 14px;
    cursor: pointer;
    transition: all 0.3s ease;
    box-shadow: 0 4px 15px rgba(99, 102, 241, 0.3);
}

.primary-btn:hover {
    transform: translateY(-2px);
    box-shadow: 0 6px 20px rgba(99, 102, 241, 0.5), 0 0 10px rgba(6, 182, 212, 0.3);
}

.primary-btn:active {
    transform: translateY(0);
}

.content-panel {
    padding: 40px;
    display: flex;
    flex-direction: column;
    gap: 20px;
    overflow-y: auto;
}

h2 {
    font-size: 28px;
    font-weight: 800;
    letter-spacing: -0.5px;
}

.panel-desc {
    color: #9ca3af;
    line-height: 1.6;
    font-size: 15px;
}

.cards-container {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 20px;
    margin-top: 10px;
}

.feature-card {
    padding: 24px;
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid rgba(255, 255, 255, 0.05);
    border-radius: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    transition: all 0.3s ease;
}

.feature-card h4 {
    font-size: 16px;
    font-weight: 700;
}

.feature-card p {
    font-size: 13px;
    color: #9ca3af;
    line-height: 1.5;
}

.feature-card.highlighted {
    background: rgba(99, 102, 241, 0.06);
    border: 1px solid rgba(99, 102, 241, 0.2);
    box-shadow: 0 0 15px rgba(99, 102, 241, 0.05);
}

.feature-card:hover {
    transform: scale(1.02);
    border-color: rgba(99, 102, 241, 0.4);
    box-shadow: 0 10px 20px rgba(0, 0, 0, 0.2), 0 0 15px rgba(99, 102, 241, 0.1);
}

.footer-container {
    padding: 16px 32px;
    border-top: 1px solid rgba(255, 255, 255, 0.06);
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 12px;
    color: #6b7280;
}

.footer-link {
    color: #9ca3af;
    cursor: pointer;
    transition: color 0.2s;
}

.footer-link:hover {
    color: #6366f1;
}
"#;
    fs::write(omni_dir.join("src/style.css"), style_css)?;

    println!(
        "{}",
        "✅ Rullst Omni (Dioxus) template successfully generated in 'omni-app/'!"
            .green()
            .bold()
    );

    Ok(())
}
