//! `cargo rullst dev` in a project set up by `cargo rullst add tailwind`:
//! one build before the application starts, then `tailwindcss --watch`
//! for as long as the session runs. The CLI is the pinned one, verified
//! before it starts; a failure is reported and the server runs without it.

use super::report;
use crate::generators::frontend::{self, HttpFetcher, tailwind};
use crate::ui::dash_tui::LogMsg;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Stdio};
use tokio::sync::mpsc;

/// The verified CLI after the first build.
pub(super) struct Prepared {
    binary: PathBuf,
}

/// The running `--watch` process; dropping it stops it.
pub(super) struct Watch {
    child: Child,
    /// Closing it ends `--watch`, also if this process dies.
    stdin: Option<ChildStdin>,
}

impl Drop for Watch {
    fn drop(&mut self) {
        drop(self.stdin.take());
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Verifies (downloading when missing) the CLI and runs one build, when the
/// project uses the managed Tailwind setup.
#[cfg_attr(mutants, mutants::skip)]
pub(super) async fn prepare(dashboard: bool, logs: &mpsc::Sender<LogMsg>) -> Option<Prepared> {
    if !frontend::has_managed_tailwind(Path::new(".")) {
        return None;
    }
    report(
        logs,
        dashboard,
        format!(
            "Tailwind CSS: building {} with tailwindcss v{}...",
            frontend::TAILWIND_OUTPUT,
            frontend::tailwind_release::VERSION
        ),
    );
    let prepared = tokio::task::spawn_blocking(|| {
        let verified = tailwind::ensure_host(&HttpFetcher).map_err(|error| error.to_string())?;
        tailwind::run_build(&verified.path, Path::new("."), false)
            .map_err(|error| error.to_string())?;
        Ok::<_, String>(verified.path)
    })
    .await
    .unwrap_or_else(|_| Err("the build task stopped".to_owned()));
    match prepared {
        Ok(binary) => Some(Prepared { binary }),
        Err(error) => {
            report(
                logs,
                dashboard,
                format!(
                    "Tailwind CSS is not rebuilding {}: {error}",
                    frontend::TAILWIND_OUTPUT
                ),
            );
            None
        }
    }
}

impl Prepared {
    /// Starts `tailwindcss --watch`; its output goes to the dev log.
    #[cfg_attr(mutants, mutants::skip)]
    pub(super) fn watch(self, dashboard: bool, logs: &mpsc::Sender<LogMsg>) -> Option<Watch> {
        // Verified again right before it starts.
        let checked =
            tailwind::host_asset().and_then(|asset| tailwind::verify(&self.binary, asset));
        let spawned = checked.map_err(|error| error.to_string()).and_then(|()| {
            let mut command = tailwind::watch_command(&self.binary, Path::new("."));
            command.stdout(Stdio::null()).stderr(Stdio::piped());
            tailwind::spawn(&mut command).map_err(|error| error.to_string())
        });
        let mut child = match spawned {
            Ok(child) => child,
            Err(error) => {
                report(
                    logs,
                    dashboard,
                    format!("Tailwind CSS watch could not start: {error}"),
                );
                return None;
            }
        };
        if let Some(stderr) = child.stderr.take() {
            let logs = logs.clone();
            std::thread::spawn(move || {
                for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                    let line = line.trim();
                    if !line.is_empty() {
                        report(&logs, dashboard, format!("tailwindcss: {line}"));
                    }
                }
            });
        }
        report(
            logs,
            dashboard,
            format!(
                "Tailwind CSS: watching {} for class changes.",
                frontend::TAILWIND_INPUT
            ),
        );
        let stdin = child.stdin.take();
        Some(Watch { child, stdin })
    }
}
