#![cfg_attr(mutants, mutants::skip)]
use crate::generators::is_rullst_project;
use axum::{
    Router,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
    routing::get,
};
use colored::*;
use notify::{RecursiveMode, Watcher};
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::process::Stdio;

use quote::quote;
use syn::{Macro, visit_mut::VisitMut};
use tokio::sync::{broadcast, mpsc};
use tokio::time::{Duration, sleep};

struct HtmlStripper;

impl VisitMut for HtmlStripper {
    fn visit_macro_mut(&mut self, m: &mut Macro) {
        if m.path.is_ident("html") {
            m.tokens = proc_macro2::TokenStream::new();
        }
        syn::visit_mut::visit_macro_mut(self, m);
    }
}

fn did_logic_change(old_src: &str, new_src: &str) -> bool {
    let mut old_ast = match syn::parse_file(old_src) {
        Ok(ast) => ast,
        Err(_) => return true,
    };
    let mut new_ast = match syn::parse_file(new_src) {
        Ok(ast) => ast,
        Err(_) => return true,
    };

    let mut stripper = HtmlStripper;
    stripper.visit_file_mut(&mut old_ast);
    stripper.visit_file_mut(&mut new_ast);

    let old_stripped = quote!(#old_ast).to_string();
    let new_stripped = quote!(#new_ast).to_string();

    old_stripped != new_stripped
}

#[derive(Clone)]
struct AppState {
    tx: broadcast::Sender<String>,
}

#[derive(Debug)]
pub enum LogMsg {
    AppStdout(String),
    AppStderr(String),
    System(String),
}

#[tokio::main]
pub async fn run_dev_server(is_dash: bool) -> Result<(), Box<dyn std::error::Error>> {
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
        "{}\n",
        "🚀 Starting Rullst Hybrid Hot-Reload Server (AST + Dylib)..."
            .cyan()
            .bold()
    );

    build_and_migrate();

    let (tx, _rx) = broadcast::channel(100);
    let app_state = AppState { tx: tx.clone() };

    let (log_tx, mut log_rx) = mpsc::unbounded_channel::<LogMsg>();

    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(3000);
    let ws_port = port + 1;

    let ws_app = Router::new()
        .route("/_rullst_hmr", get(ws_handler))
        .with_state(app_state.clone());

    let ws_listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", ws_port)).await?;
    let msg = format!(
        "📡 Rullst HMR WebSocket listening on ws://127.0.0.1:{}/_rullst_hmr",
        ws_port
    );
    if is_dash {
        let _ = log_tx.send(LogMsg::System(msg));
    } else {
        println!("{}", msg);
    }

    tokio::spawn(async move {
        let _ = axum::serve(ws_listener, ws_app).await;
    });

    let msg2 = "📦 Booting Rullst application...".yellow().to_string();
    if is_dash {
        let _ = log_tx.send(LogMsg::System(msg2));
    } else {
        println!("{}", msg2);
    }

    let mut cmd = Command::new("cargo");
    cmd.arg("run").arg("-q");
    if is_dash {
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    }

    let mut app_child = cmd.spawn()?;

    if is_dash {
        let stdout = app_child.stdout.take().unwrap();
        let stderr = app_child.stderr.take().unwrap();
        let tx1 = log_tx.clone();
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                let _ = tx1.send(LogMsg::AppStdout(line));
            }
        });
        let tx2 = log_tx.clone();
        std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                let _ = tx2.send(LogMsg::AppStderr(line));
            }
        });
    }

    let log_tx_watcher = log_tx.clone();

    let watcher_task = tokio::spawn(async move {
        let (notify_tx, mut notify_rx) = tokio::sync::mpsc::channel(100);
        let mut watcher =
            notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
                if let Ok(event) = res {
                    let _ = notify_tx.blocking_send(event);
                }
            })
            .unwrap();

        watcher
            .watch(Path::new("src"), RecursiveMode::Recursive)
            .unwrap();
        watcher
            .watch(Path::new("Cargo.toml"), RecursiveMode::NonRecursive)
            .unwrap();

        let m = "✨ Watching for file changes... (Press Ctrl+C to stop)"
            .green()
            .to_string();
        if is_dash {
            let _ = log_tx_watcher.send(LogMsg::System(m));
        } else {
            println!("{}", m);
        }

        let mut last_build = std::time::Instant::now();
        let mut file_cache: HashMap<PathBuf, String> = HashMap::new();

        for entry in walkdir::WalkDir::new("src")
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.path().is_file()
                && entry.path().extension().and_then(|e| e.to_str()) == Some("rs")
            {
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    file_cache.insert(entry.path().to_path_buf(), content);
                }
            }
        }

        while let Some(event) = notify_rx.recv().await {
            sleep(Duration::from_millis(150)).await;
            while notify_rx.try_recv().is_ok() {}

            if last_build.elapsed() < Duration::from_millis(500) {
                continue;
            }

            let mut logic_changed = false;
            let mut html_changed = false;

            for path in event.paths {
                if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                    if let Ok(new_content) = fs::read_to_string(&path) {
                        if let Some(old_content) = file_cache.get(&path) {
                            if old_content != &new_content {
                                if did_logic_change(old_content, &new_content) {
                                    logic_changed = true;
                                } else {
                                    html_changed = true;
                                }
                                file_cache.insert(path.clone(), new_content);
                            }
                        } else {
                            logic_changed = true;
                            file_cache.insert(path.clone(), new_content);
                        }
                    }
                } else if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                    logic_changed = true;
                }
            }

            if html_changed && !logic_changed {
                let m = "🎨 UI change detected. Sending HTML fragment via WebSocket..."
                    .magenta()
                    .to_string();
                if is_dash {
                    let _ = log_tx_watcher.send(LogMsg::System(m));
                } else {
                    println!("{}", m);
                }
                let _ = tx.send(r#"{"type": "UI_UPDATE"}"#.to_string());
            }

            if logic_changed {
                let m = "🔄 File change detected. Recompiling library for Hot-Swap..."
                    .yellow()
                    .to_string();
                if is_dash {
                    let _ = log_tx_watcher.send(LogMsg::System(m));
                } else {
                    println!("{}", m);
                }

                let status = Command::new("cargo")
                    .arg("build")
                    .arg("--lib")
                    .arg("-q")
                    .status();

                if let Ok(status) = status {
                    if status.success() {
                        let client = reqwest::Client::new();
                        let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
                        let url =
                            format!("http://127.0.0.1:{}/_rullst/internal/reload_dylib", port);
                        match client.post(&url).send().await {
                            Ok(_) => {
                                let m = "✅ Hot-Swap executed successfully. App updated!"
                                    .green()
                                    .to_string();
                                if is_dash {
                                    let _ = log_tx_watcher.send(LogMsg::System(m));
                                } else {
                                    println!("{}", m);
                                }
                            }
                            Err(_) => {
                                let m =
                                    "⚠️ Failed to trigger hot-swap webhook. Is the app running?"
                                        .red()
                                        .to_string();
                                if is_dash {
                                    let _ = log_tx_watcher.send(LogMsg::System(m));
                                } else {
                                    println!("{}", m);
                                }
                            }
                        }
                    } else {
                        let m = "❌ Build failed. Please fix errors to Hot-Swap."
                            .red()
                            .to_string();
                        if is_dash {
                            let _ = log_tx_watcher.send(LogMsg::System(m));
                        } else {
                            println!("{}", m);
                        }
                    }
                }
            }

            last_build = std::time::Instant::now();
        }
    });

    if is_dash {
        crate::ui::dash_tui::run(log_rx, port).await?;
        let _ = app_child.kill();
        watcher_task.abort();
    } else {
        while let Some(msg) = log_rx.recv().await {
            match msg {
                LogMsg::System(s) => println!("{}", s),
                LogMsg::AppStdout(s) => println!("{}", s),
                LogMsg::AppStderr(s) => eprintln!("{}", s),
            }
        }
        let _ = app_child.kill();
    }

    Ok(())
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    let mut rx = state.tx.subscribe();
    while let Ok(msg) = rx.recv().await {
        if socket.send(Message::Text(msg.into())).await.is_err() {
            break;
        }
    }
}

fn build_and_migrate() {
    let output_result =
        crate::ui::components::with_spinner("Compiling Rullst Application...", || {
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
                    "❌ Compilation failed. Run `cargo build` to see errors.".red()
                );
                std::process::exit(1);
            }
        }
        Err(_) => {
            println!("{}", "❌ Failed to execute `cargo build`.".red());
            std::process::exit(1);
        }
    }

    if std::path::Path::new("src/migrations").exists() {
        println!("{}", "📦 Executing pending database migrations...".yellow());
        let _ = Command::new("cargo")
            .arg("run")
            .arg("-q")
            .arg("--")
            .arg("db:migrate")
            .status();
    }
}
