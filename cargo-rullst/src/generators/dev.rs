#![cfg_attr(mutants, mutants::skip)]
use crate::generators::is_rullst_project;
use colored::*;
use notify::{RecursiveMode, Watcher};
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::process::Stdio;

use tokio::sync::mpsc;
use tokio::time::{Duration, sleep};

#[path = "dev_support.rs"]
mod support;
use support::{
    bounded_diagnostic, did_logic_change, generate_reload_token, hot_reload_profile_available,
    is_actionable_event, report_watcher_message,
};

const DASHBOARD_LOG_CAPACITY: usize = 512;
const RELOAD_TOKEN_HEADER: &str = "x-rullst-hmr-token";

#[derive(Debug, thiserror::Error)]
enum DevServerError {
    #[error("this command must be executed in the root of a valid Rullst project")]
    NotRullstProject,
    #[error("failed to invoke the initial Cargo build: {0}")]
    BuildInvocation(#[source] std::io::Error),
    #[error("the initial application build failed; run `cargo build` for the complete diagnostic")]
    BuildFailed,
    #[error("failed to invoke the initial database migration: {0}")]
    MigrationInvocation(#[source] std::io::Error),
    #[error("the initial database migration failed with status {0}")]
    MigrationFailed(std::process::ExitStatus),
}

use crate::ui::dash_tui::LogMsg;

#[tokio::main]
pub async fn run_dev_server(is_dash: bool) -> Result<(), Box<dyn std::error::Error>> {
    if !is_rullst_project() {
        return Err(DevServerError::NotRullstProject.into());
    }

    println!(
        "{}\n",
        "🚀 Starting the Rullst development server...".cyan().bold()
    );

    build_and_migrate()?;

    let hot_reload_enabled = hot_reload_profile_available(Path::new("."));

    let (log_tx, log_rx) = mpsc::channel::<LogMsg>(DASHBOARD_LOG_CAPACITY);

    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(3000);
    if hot_reload_enabled {
        let msg =
            format!("📡 Authenticated hot reload active on http://127.0.0.1:{port}/_rullst_hmr");
        if is_dash {
            let _ = log_tx.send(LogMsg::System(msg)).await;
        } else {
            println!("{msg}");
        }
    }

    let msg2 = "📦 Booting Rullst application...".yellow().to_string();
    if is_dash {
        let _ = log_tx.send(LogMsg::System(msg2)).await;
    } else {
        println!("{}", msg2);
    }

    let reload_token = generate_reload_token();
    let mut cmd = Command::new("cargo");
    cmd.arg("run").arg("-q");
    if hot_reload_enabled {
        cmd.env("HOT_RELOAD", "1");
        cmd.env("RULLST_HMR_TOKEN", &reload_token);
    }
    if is_dash {
        configure_dashboard_process_group(&mut cmd);
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    }

    let mut app_child = cmd.spawn()?;

    if is_dash {
        let stdout = app_child
            .stdout
            .take()
            .ok_or_else(|| std::io::Error::other("cargo stdout was not piped for the dashboard"))?;
        let stderr = app_child
            .stderr
            .take()
            .ok_or_else(|| std::io::Error::other("cargo stderr was not piped for the dashboard"))?;
        let tx1 = log_tx.clone();
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                if tx1.blocking_send(LogMsg::AppStdout(line)).is_err() {
                    break;
                }
            }
        });
        let tx2 = log_tx.clone();
        std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                if tx2.blocking_send(LogMsg::AppStderr(line)).is_err() {
                    break;
                }
            }
        });
    }

    let log_tx_watcher = log_tx.clone();
    let reload_token_watcher = reload_token;

    let watcher_task = tokio::spawn(async move {
        if !hot_reload_enabled {
            report_watcher_message(
                &log_tx_watcher,
                is_dash,
                "Hot swapping is disabled for this scaffold; regenerate with `--hot-reload` to add the explicit cdylib boundary."
                    .to_string(),
                false,
            )
            .await;
            return;
        }
        let (notify_tx, mut notify_rx) = tokio::sync::mpsc::channel(100);
        let mut watcher =
            match notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
                if let Ok(event) = res {
                    if !is_actionable_event(event.kind) {
                        return;
                    }
                    let _ = notify_tx.blocking_send(event);
                }
            }) {
                Ok(watcher) => watcher,
                Err(error) => {
                    let message = format!("❌ Failed to initialize the file watcher: {error}");
                    if is_dash {
                        let _ = log_tx_watcher.send(LogMsg::System(message)).await;
                    } else {
                        eprintln!("{message}");
                    }
                    return;
                }
            };

        for (path, mode) in [
            (Path::new("src"), RecursiveMode::Recursive),
            (Path::new("Cargo.toml"), RecursiveMode::NonRecursive),
        ] {
            if let Err(error) = watcher.watch(path, mode) {
                let message = format!("❌ Failed to watch {}: {error}", path.display());
                if is_dash {
                    let _ = log_tx_watcher.send(LogMsg::System(message)).await;
                } else {
                    eprintln!("{message}");
                }
                return;
            }
        }

        let m = "✨ Watching for file changes... (Press Ctrl+C to stop)"
            .green()
            .to_string();
        if is_dash {
            let _ = log_tx_watcher.send(LogMsg::System(m)).await;
        } else {
            println!("{}", m);
        }

        let mut file_cache: HashMap<PathBuf, String> = HashMap::new();

        for entry in walkdir::WalkDir::new("src")
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.path().is_file()
                && entry.path().extension().and_then(|e| e.to_str()) == Some("rs")
                && let Ok(content) = fs::read_to_string(entry.path())
            {
                file_cache.insert(entry.path().to_path_buf(), content);
            }
        }

        while let Some(event) = notify_rx.recv().await {
            let mut changed_paths = event.paths;
            sleep(Duration::from_millis(150)).await;
            while let Ok(coalesced) = notify_rx.try_recv() {
                changed_paths.extend(coalesced.paths);
            }
            changed_paths.sort_unstable();
            changed_paths.dedup();

            let mut logic_changed = false;
            let mut html_changed = false;

            for path in changed_paths {
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
                    } else if file_cache.remove(&path).is_some() {
                        logic_changed = true;
                    }
                } else if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                    logic_changed = true;
                }
            }

            if html_changed || logic_changed {
                let change_kind = if logic_changed {
                    "Rust logic or manifest"
                } else {
                    "server-rendered view"
                };
                report_watcher_message(
                    &log_tx_watcher,
                    is_dash,
                    format!("🔄 {change_kind} change detected; rebuilding the hot library..."),
                    false,
                )
                .await;

                let started = std::time::Instant::now();
                let output = tokio::process::Command::new("cargo")
                    .arg("build")
                    .arg("--lib")
                    .arg("-q")
                    .output()
                    .await;

                match output {
                    Ok(output) if output.status.success() => {
                        let client = reqwest::Client::new();
                        let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
                        let url =
                            format!("http://127.0.0.1:{}/_rullst/internal/reload_dylib", port);
                        match client
                            .post(&url)
                            .header(RELOAD_TOKEN_HEADER, &reload_token_watcher)
                            .timeout(Duration::from_secs(5))
                            .send()
                            .await
                        {
                            Ok(response) if response.status().is_success() => {
                                report_watcher_message(
                                    &log_tx_watcher,
                                    is_dash,
                                    format!(
                                        "✅ Hot library swapped in {:.0} ms; refreshing connected browsers.",
                                        started.elapsed().as_secs_f64() * 1_000.0
                                    ),
                                    false,
                                )
                                .await;
                            }
                            Ok(response) => {
                                let status = response.status();
                                report_watcher_message(
                                    &log_tx_watcher,
                                    is_dash,
                                    format!(
                                        "Hot-swap endpoint rejected the new library ({status})."
                                    ),
                                    true,
                                )
                                .await;
                            }
                            Err(error) => {
                                report_watcher_message(
                                    &log_tx_watcher,
                                    is_dash,
                                    format!("Hot-swap request failed: {error}"),
                                    true,
                                )
                                .await;
                            }
                        }
                    }
                    Ok(output) => {
                        let diagnostic = bounded_diagnostic(&output.stderr);
                        report_watcher_message(
                            &log_tx_watcher,
                            is_dash,
                            format!("Hot-reload build failed; the running application was kept unchanged.\n{diagnostic}"),
                            true,
                        )
                        .await;
                    }
                    Err(error) => {
                        report_watcher_message(
                            &log_tx_watcher,
                            is_dash,
                            format!("Could not invoke the hot-reload build: {error}"),
                            true,
                        )
                        .await;
                    }
                }
            }
        }
    });

    if is_dash {
        let dashboard_result = crate::ui::dash_tui::run(
            log_rx,
            log_tx.clone(),
            port,
            hot_reload_enabled,
            &mut app_child,
        )
        .await;
        stop_child(&mut app_child);
        watcher_task.abort();
        dashboard_result?;
    } else {
        let _ = app_child.wait()?;
        watcher_task.abort();
    }

    Ok(())
}

fn stop_child(child: &mut std::process::Child) {
    if child.try_wait().ok().flatten().is_none() && !terminate_dashboard_process_group(child.id()) {
        let _ = child.kill();
    }
    let _ = child.wait();
}

fn configure_dashboard_process_group(command: &mut Command) {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        command.creation_flags(CREATE_NEW_PROCESS_GROUP);
    }
}

fn terminate_dashboard_process_group(process_id: u32) -> bool {
    #[cfg(unix)]
    {
        return Command::new("kill")
            .args(["-TERM", "--", &format!("-{process_id}")])
            .status()
            .is_ok_and(|status| status.success());
    }
    #[cfg(windows)]
    {
        return Command::new("taskkill")
            .args(["/PID", &process_id.to_string(), "/T", "/F"])
            .status()
            .is_ok_and(|status| status.success());
    }
    #[allow(unreachable_code)]
    false
}

fn build_and_migrate() -> Result<(), DevServerError> {
    let output_result =
        crate::ui::components::with_spinner("Compiling Rullst Application...", || {
            Command::new("cargo").arg("build").arg("-q").output()
        });

    let output = output_result.map_err(DevServerError::BuildInvocation)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.trim().is_empty() {
            eprintln!("{stderr}");
        }
        return Err(DevServerError::BuildFailed);
    }

    if std::path::Path::new("src/migrations").exists() {
        println!("{}", "📦 Executing pending database migrations...".yellow());
        let status = Command::new("cargo")
            .arg("run")
            .arg("-q")
            .arg("--")
            .arg("db:migrate")
            .status()
            .map_err(DevServerError::MigrationInvocation)?;
        if !status.success() {
            return Err(DevServerError::MigrationFailed(status));
        }
    }
    Ok(())
}
