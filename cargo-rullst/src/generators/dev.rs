//! Development uses a directly linked executable and supervised process restart.
mod build;
mod process;
mod watcher;

pub(crate) use process::{BuildChild, configure_group};

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
    /// Restarts the owned process from its current executable snapshot.
    Restart,
}

/// The single line `dev` adds to its plain output.
const DASH_HINT: &str =
    "Tip: run `cargo rullst dash` for live requests/s, latency, errors and database metrics.";

pub fn run_dev_server(is_dash: bool) -> Result<(), Box<dyn std::error::Error>> {
    run_dev(is_dash, false)
}

/// `run_dev_server`; with `ts_sync`, the TypeScript SDK is regenerated after
/// the initial build and after every successful rebuild.
#[tokio::main]
pub(crate) async fn run_dev(
    is_dash: bool,
    ts_sync: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if !crate::generators::is_rullst_project() {
        return Err(io::Error::other("run this command in a Rullst project root").into());
    }
    let port = configured_port()?;
    let (log_tx, log_rx) = mpsc::channel(512);
    let (status_tx, status_rx) = watch::channel(DevStatus::Starting);
    let (commands, command_rx) = mpsc::channel(1);
    let supervisor = supervise(
        is_dash,
        ts_sync,
        port,
        log_tx.clone(),
        status_tx,
        command_rx,
    );
    tokio::pin!(supervisor);
    let shutdown = shutdown_signal()?;
    if is_dash {
        tokio::select! {
            result = &mut supervisor => result?,
            result = crate::ui::dash_tui::run(log_rx, log_tx, port, true, status_rx, commands) => result?,
            result = shutdown => result?,
        }
    } else {
        drop(log_rx);
        eprintln!("{DASH_HINT}");
        tokio::select! {
            result = &mut supervisor => result?,
            result = shutdown => result?,
        }
    }
    // Dropping the supervisor cancels the watcher/build and reaps its owned child.
    Ok(())
}

/// Resolves on Ctrl+C and, on Unix, on SIGTERM (an IDE stop button, `kill`)
/// or SIGHUP (a closed terminal); on Windows also on console close. Each ends
/// the session through the drop path that stops the application group and
/// removes its snapshot, instead of the default action orphaning them. The
/// handlers are registered before this returns.
fn shutdown_signal() -> io::Result<impl std::future::Future<Output = io::Result<()>>> {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let mut terminate = signal(SignalKind::terminate())?;
        let mut hangup = signal(SignalKind::hangup())?;
        Ok(async move {
            tokio::select! {
                result = tokio::signal::ctrl_c() => result,
                _ = terminate.recv() => Ok(()),
                _ = hangup.recv() => Ok(()),
            }
        })
    }
    #[cfg(windows)]
    {
        let mut close = tokio::signal::windows::ctrl_close()?;
        Ok(async move {
            tokio::select! {
                result = tokio::signal::ctrl_c() => result,
                _ = close.recv() => Ok(()),
            }
        })
    }
    #[cfg(not(any(unix, windows)))]
    {
        Ok(tokio::signal::ctrl_c())
    }
}

async fn supervise(
    dashboard: bool,
    ts_sync: bool,
    port: u16,
    logs: mpsc::Sender<LogMsg>,
    status: watch::Sender<DevStatus>,
    mut commands: mpsc::Receiver<DevCommand>,
) -> io::Result<()> {
    let (mut watcher, mut changes) = watcher::watch_project(Path::new("."))?;
    remove_stale_precompressed_assets(dashboard, &logs);
    report(&logs, dashboard, "Building the application...".into());
    let executable = build::compile().await?;
    sync_typescript(ts_sync, dashboard, &logs);
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
            Some(command) = commands.recv() => match command {
                DevCommand::Migrate => {
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
                DevCommand::Restart => {
                    restart(&mut running, dashboard, &logs, &status)?;
                    exit_reported = false;
                    report_ready(&mut running, port, dashboard, &logs, &status).await;
                }
            },
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
                remove_stale_precompressed_assets(dashboard, &logs);
                report(&logs, dashboard, "Change detected; rebuilding before restart...".into());
                let started = std::time::Instant::now();
                let executable = match build::compile().await {
                    Ok(executable) => executable,
                    Err(error) => {
                        report(&logs, dashboard, format!("Build failed; current application kept running.\n{error}"));
                        continue;
                    }
                };
                sync_typescript(ts_sync, dashboard, &logs);
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

/// Stops the owned process group and starts the same executable snapshot as a
/// new process generation; readiness is verified by the caller.
fn restart(
    running: &mut process::Application,
    dashboard: bool,
    logs: &mpsc::Sender<LogMsg>,
    status: &watch::Sender<DevStatus>,
) -> io::Result<()> {
    report(
        logs,
        dashboard,
        "Restarting the application from the current build...".into(),
    );
    running.stop()?;
    status.send_replace(DevStatus::Starting);
    running.start(dashboard, logs)
}

/// `dev --ts-sync`: regenerates the TypeScript SDK from routes that just
/// compiled. A failure is reported and never stops the supervisor.
fn sync_typescript(enabled: bool, dashboard: bool, logs: &mpsc::Sender<LogMsg>) {
    if enabled {
        report(logs, dashboard, typescript_sync_message(Path::new(".")));
    }
}

fn typescript_sync_message(root: &Path) -> String {
    match crate::generators::ts::sync_ts_sdk(root) {
        Ok(path) => format!("TypeScript SDK synchronized at {}.", path.display()),
        Err(error) => format!("TypeScript SDK sync failed: {error}"),
    }
}

/// Drops `.br`/`.zst` siblings from an earlier `cargo rullst build` that are
/// older than their asset, because the server would keep serving them in
/// place of the edited file. Failures are reported, never fatal.
fn remove_stale_precompressed_assets(dashboard: bool, logs: &mpsc::Sender<LogMsg>) {
    let static_dir = Path::new("static");
    if !static_dir.is_dir() {
        return;
    }
    match crate::generators::build::precompressed::remove_stale_siblings(static_dir) {
        Ok(0) => {}
        Ok(removed) => report(
            logs,
            dashboard,
            format!(
                "Removed {removed} pre-compressed static file(s) older than their source; run `cargo rullst build` to regenerate them."
            ),
        ),
        Err(error) => report(
            logs,
            dashboard,
            format!(
                "Could not remove outdated pre-compressed static files; edited assets may be shadowed: {error}"
            ),
        ),
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
    let dotenv = if Path::new(".env").is_file() {
        parse_dotenv(&std::fs::read(".env")?)?
    } else {
        Default::default()
    };
    let config = if Path::new("Rullst.toml").is_file() {
        parse_rullst_toml(&std::fs::read_to_string("Rullst.toml")?)?
    } else {
        toml::Value::Table(Default::default())
    };
    resolve_configured_port(std::env::var_os("PORT"), &dotenv, &config)
}

/// dotenvy's parse error quotes the unparsed remainder of the file, which can
/// hold secrets, so a failure reports only the 1-based entry number.
pub(crate) fn parse_dotenv(source: &[u8]) -> io::Result<std::collections::HashMap<String, String>> {
    let mut values = std::collections::HashMap::new();
    for (index, entry) in dotenvy::from_read_iter(source).enumerate() {
        let (key, value) = entry.map_err(|error| {
            io::Error::other(match error {
                dotenvy::Error::LineParse(..) => {
                    format!("invalid .env syntax in entry {}", index + 1)
                }
                dotenvy::Error::Io(error) => format!("failed to read .env: {}", error.kind()),
                _ => "invalid .env file".to_owned(),
            })
        })?;
        values.insert(key, value);
    }
    Ok(values)
}

pub(crate) fn parse_rullst_toml(source: &str) -> io::Result<toml::Value> {
    toml::from_str(source).map_err(|error| {
        io::Error::other(format!(
            "Rullst.toml is not valid TOML at {}",
            super::toml_error_position(source, &error)
        ))
    })
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
