use super::ProjectError;
use std::{
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use tokio::io::AsyncReadExt;

pub(super) fn capture(
    program: &'static str,
    args: &[&str],
    root: &Path,
    limit: u64,
) -> Result<Vec<u8>, ProjectError> {
    if tokio::runtime::Handle::try_current().is_ok() {
        return Err(ProjectError::Invalid(
            "project preparation requires a synchronous CLI context",
        ));
    }
    let tool = tool_path(program)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async {
        tokio::time::timeout(Duration::from_secs(30), async {
            let mut child = tokio::process::Command::new(tool).args(args).current_dir(root)
                .env("GIT_OPTIONAL_LOCKS", "0").env("CARGO_NET_OFFLINE", "true")
                .env("RUSTUP_AUTO_INSTALL", "0")
                .env("RULLST_DISABLE_UPDATE_CHECK", "true")
                .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped())
                .kill_on_drop(true).spawn()?;
            let stdout = child.stdout.take().ok_or(ProjectError::Invalid("missing tool output stream"))?;
            let stderr = child.stderr.take().ok_or(ProjectError::Invalid("missing tool error stream"))?;
            let (status, body, _diagnostic) = tokio::try_join!(
                async { child.wait().await.map_err(ProjectError::Io) },
                read(stdout, limit), read(stderr, 64 * 1024))?;
            if !status.success() {
                return Err(ProjectError::Invalid("Git inventory or offline Cargo metadata failed; review the selected project's manifests and local dependencies"));
            }
            Ok(body)
        }).await.map_err(|_| ProjectError::Invalid("project metadata tool timed out"))?
    })
}

async fn read(
    reader: impl tokio::io::AsyncRead + Unpin,
    limit: u64,
) -> Result<Vec<u8>, ProjectError> {
    let mut body = Vec::new();
    reader.take(limit + 1).read_to_end(&mut body).await?;
    if body.len() as u64 > limit {
        return Err(ProjectError::Invalid(
            "project metadata output exceeds its bound",
        ));
    }
    Ok(body)
}

fn tool_path(name: &str) -> Result<PathBuf, ProjectError> {
    let path = std::env::var_os("PATH").unwrap_or_default();
    let filename = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_owned()
    };
    for directory in std::env::split_paths(&path).filter(|entry| entry.is_absolute()) {
        let candidate = directory.join(&filename);
        let Ok(metadata) = candidate.metadata() else {
            continue;
        };
        if !metadata.is_file() {
            continue;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if metadata.permissions().mode() & 0o111 == 0 {
                continue;
            }
        }
        // Keep the proxy basename: resolving cargo's symlink to rustup would
        // change rustup's dispatch. PATH itself remains caller-trusted.
        return Ok(candidate);
    }
    Err(ProjectError::Invalid(
        "Git and Cargo must be installed on absolute trusted PATH entries",
    ))
}
