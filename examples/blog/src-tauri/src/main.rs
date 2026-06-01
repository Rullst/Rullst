#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

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
