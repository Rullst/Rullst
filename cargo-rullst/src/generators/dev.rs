//! Development uses a directly linked executable and supervised process restart.
mod build;
mod process;
mod watcher;

use crate::ui::dash_tui::LogMsg;
use std::{io, path::Path, process::ExitStatus, time::Duration};
use tokio::sync::{mpsc, watch};

#[derive(Clone, Copy, Debug)]
pub(crate) enum DevStatus {
    Starting,
    Ready,
    Unverified,
    Exited(ExitStatus),
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum DevCommand {
    Migrate,
}

#[tokio::main]
pub async fn run_dev_server(is_dash: bool) -> Result<(), Box<dyn std::error::Error>> {
    if !crate::generators::is_rullst_project() {
        return Err(io::Error::other("run this command in a Rullst project root").into());
    }
    let port = configured_port()?;
    let (log_tx, log_rx) = mpsc::channel(512);
    let (status_tx, status_rx) = watch::channel(DevStatus::Starting);
    let (commands, command_rx) = mpsc::channel(1);
    let supervisor = supervise(is_dash, port, log_tx.clone(), status_tx, command_rx);
    tokio::pin!(supervisor);
    if is_dash {
        tokio::select! {
            result = &mut supervisor => result?,
            result = crate::ui::dash_tui::run(log_rx, log_tx, port, true, status_rx, commands) => result?,
            result = tokio::signal::ctrl_c() => result?,
        }
    } else {
        drop(log_rx);
        tokio::select! {
            result = &mut supervisor => result?,
            result = tokio::signal::ctrl_c() => result?,
        }
    }
    // Dropping the supervisor cancels the watcher/build and reaps its owned child.
    Ok(())
}

async fn supervise(
    dashboard: bool,
    port: u16,
    logs: mpsc::Sender<LogMsg>,
    status: watch::Sender<DevStatus>,
    mut commands: mpsc::Receiver<DevCommand>,
) -> io::Result<()> {
    let (mut watcher, mut changes) = watcher::watch_project(Path::new("."))?;
    report(&logs, dashboard, "Building the application...".into());
    let executable = build::compile().await?;
    let mut running = process::Application::prepare(&executable)?;
    if Path::new("src/migrations").is_dir() {
        report(&logs, dashboard, "Running initial db:migrate...".into());
        running.migrate(dashboard, &logs).await?;
    }
    running.start(dashboard, &logs)?;
    status.send_replace(DevStatus::Starting);
    report(&logs, dashboard, "Auto-reload: watching source, assets and configuration; successful builds restart the application.".into());
    report_ready(&mut running, port, dashboard, &logs, &status).await;
    let mut tick = tokio::time::interval(Duration::from_millis(250));
    let mut exit_reported = false;
    loop {
        tokio::select! {
            _ = tick.tick() => {
                if !exit_reported && let Some(exit) = running.try_wait()? {
                    status.send_replace(DevStatus::Exited(exit));
                    report(&logs, dashboard, format!("Application exited ({exit}); fix the error and save to retry."));
                    exit_reported = true;
                }
            }
            Some(DevCommand::Migrate) = commands.recv() => {
                // Owned by this future: dashboard exit cancels the migration
                // and its child group, with the same bounded output as startup.
                let result = if Path::new("src/migrations").is_dir() {
                    running.migrate(dashboard, &logs).await
                } else {
                    Err(io::Error::other("this project has no src/migrations directory"))
                };
                let _ = logs.send(LogMsg::MigrationFinished {
                    success: result.is_ok(),
                    summary: match result {
                        Ok(()) => "Database migration completed using the current executable snapshot.".into(),
                        Err(error) => format!("Database migration failed: {error}"),
                    },
                }).await;
            }
            changed = changes.recv() => {
                if changed.is_none() {
                    return Err(io::Error::other("development file watcher stopped"));
                }
                tokio::time::sleep(Duration::from_millis(150)).await;
                while changes.try_recv().is_ok() {}
                if let Err(error) = watcher::watch_directories(&mut watcher, Path::new(".")) {
                    report(&logs, dashboard, format!("Could not refresh directory watches; current application kept running. Save again to retry: {error}"));
                    continue;
                }
                report(&logs, dashboard, "Change detected; rebuilding before restart...".into());
                let started = std::time::Instant::now();
                let executable = match build::compile().await {
                    Ok(executable) => executable,
                    Err(error) => {
                        report(&logs, dashboard, format!("Build failed; current application kept running.\n{error}"));
                        continue;
                    }
                };
                // Snapshot first: Windows must not lock Cargo's next build output.
                let Some(mut replacement) = prepare_replacement(&executable, dashboard, &logs) else {
                    continue;
                };
                running.stop()?;
                status.send_replace(DevStatus::Starting);
                match replacement.start(dashboard, &logs) {
                    Ok(()) => running = replacement,
                    Err(error) => {
                        running.start(dashboard, &logs)?;
                        report(&logs, dashboard, format!("Replacement could not start; previous binary restarted: {error}"));
                    }
                }
                exit_reported = false;
                report_ready(&mut running, port, dashboard, &logs, &status).await;
                report(&logs, dashboard, format!("Reload attempt finished in {:.0} ms. In-memory state resets; migrations after startup are explicit.", started.elapsed().as_secs_f64() * 1000.0));
            }
        }
    }
}

fn prepare_replacement(
    executable: &Path,
    dashboard: bool,
    logs: &mpsc::Sender<LogMsg>,
) -> Option<process::Application> {
    match process::Application::prepare(executable) {
        Ok(replacement) => Some(replacement),
        Err(error) => {
            report(
                logs,
                dashboard,
                format!(
                    "Could not snapshot the new executable; current application kept running: {error}"
                ),
            );
            None
        }
    }
}

async fn report_ready(
    app: &mut process::Application,
    port: u16,
    dashboard: bool,
    logs: &mpsc::Sender<LogMsg>,
    status: &watch::Sender<DevStatus>,
) {
    match app.wait_ready(port).await {
        Ok(()) => {
            status.send_replace(DevStatus::Ready);
            report(
                logs,
                dashboard,
                format!(
                    "Application generation ready on http://127.0.0.1:{port}; browsers may refresh."
                ),
            )
        }
        Err(error) => {
            status.send_replace(DevStatus::Unverified);
            report(
                logs,
                dashboard,
                format!(
                    "Application readiness was not confirmed: {error}. Check the logs; save to retry."
                ),
            )
        }
    }
}

pub(super) fn report(logs: &mpsc::Sender<LogMsg>, dashboard: bool, message: String) {
    if dashboard {
        let _ = logs.try_send(LogMsg::System(message));
    } else {
        eprintln!("{message}");
    }
}

fn configured_port() -> io::Result<u16> {
    let dotenv: std::collections::HashMap<String, String> = if Path::new(".env").is_file() {
        dotenvy::from_read_iter(std::fs::read(".env")?.as_slice())
            .collect::<Result<_, _>>()
            .map_err(io::Error::other)?
    } else {
        Default::default()
    };
    let config: toml::Value = if Path::new("Rullst.toml").is_file() {
        toml::from_str(&std::fs::read_to_string("Rullst.toml")?).map_err(io::Error::other)?
    } else {
        toml::Value::Table(Default::default())
    };
    resolve_configured_port(std::env::var_os("PORT"), &dotenv, &config)
}

fn resolve_configured_port(
    process_port: Option<std::ffi::OsString>,
    dotenv: &std::collections::HashMap<String, String>,
    config: &toml::Value,
) -> io::Result<u16> {
    let value = process_port
        .map(|value| {
            value
                .into_string()
                .map_err(|_| io::Error::other("PORT is not Unicode"))
        })
        .transpose()?
        .or_else(|| dotenv.get("PORT").cloned())
        .or_else(|| {
            config
                .get("app")?
                .get("port")?
                .as_integer()
                .map(|value| value.to_string())
        });
    match value {
        Some(value) => value
            .parse::<u16>()
            .ok()
            .filter(|value| *value > 0)
            .ok_or_else(|| io::Error::other("PORT must be between 1 and 65535 for auto-reload")),
        None => Ok(3000),
    }
}

#[cfg(test)]
#[path = "dev_tests.rs"]
mod tests;
