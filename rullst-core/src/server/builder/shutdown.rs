//! OS-signal shutdown and lifecycle readiness/drain transitions for the server.

use super::ServerError;
use crate::lifecycle::ApplicationLifecycle;

/// Listens for OS termination signals (SIGINT / SIGTERM / Ctrl+C) to drain in-flight requests cleanly.
#[cfg_attr(mutants, mutants::skip)]
pub async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut stream) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            stream.recv().await;
        } else {
            std::future::pending::<()>().await;
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            crate::server::console::stdout_line(format_args!("\n🛑 [Rullst Shutdown] Received SIGINT (Ctrl+C). Draining in-flight requests..."));
        },
        _ = terminate => {
            crate::server::console::stdout_line(format_args!("\n🛑 [Rullst Shutdown] Received SIGTERM. Draining in-flight requests..."));
        },
    }
}

pub(super) async fn shutdown_with_lifecycle<F>(shutdown: F, lifecycle: Option<ApplicationLifecycle>)
where
    F: std::future::Future<Output = ()>,
{
    shutdown.await;
    if let Some(lifecycle) = lifecycle {
        let _ = lifecycle.begin_draining();
    }
}

pub(super) fn mark_lifecycle_ready(
    lifecycle: Option<&ApplicationLifecycle>,
) -> Result<(), ServerError> {
    match lifecycle {
        Some(lifecycle) => lifecycle.mark_ready().map_err(ServerError::from),
        None => Ok(()),
    }
}

pub(super) fn mark_lifecycle_stopped(lifecycle: Option<&ApplicationLifecycle>) {
    if let Some(lifecycle) = lifecycle {
        lifecycle.mark_stopped();
    }
}
