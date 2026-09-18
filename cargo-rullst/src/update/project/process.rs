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
    capture_with_status(program, args, root, limit, false)
}

fn capture_with_status(
    program: &'static str,
    args: &[&str],
    root: &Path,
    limit: u64,
    diff: bool,
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
            if !status.success() && !(diff && status.code() == Some(1)) {
                return Err(ProjectError::Invalid("Git inventory or offline Cargo metadata failed; review the selected project's manifests and local dependencies"));
            }
            Ok(body)
        }).await.map_err(|_| ProjectError::Invalid("project metadata tool timed out"))?
    })
}

pub(super) fn diff(before: &Path, candidate: &Path) -> Result<String, ProjectError> {
    let before = before.canonicalize()?;
    let candidate = candidate.canonicalize()?;
    let root = before
        .parent()
        .and_then(Path::parent)
        .ok_or(ProjectError::Invalid(
            "review source has no private cache parent",
        ))?;
    // Git for Windows does not consistently accept verbatim \\?\ paths in
    // no-index arguments. Use bounded relative paths within the same private
    // cache; canonical source identity is still validated by the caller.
    let relative = |path: &Path| -> Result<String, ProjectError> {
        Ok(path
            .strip_prefix(root)
            .map_err(|_| ProjectError::Invalid("review paths must share the private cache"))?
            .to_str()
            .ok_or(ProjectError::Invalid("review paths must be UTF-8"))?
            .replace('\\', "/"))
    };
    let before_argument = relative(&before)?;
    let candidate_argument = relative(&candidate)?;
    let output = capture_with_status(
        "git",
        &[
            "--no-pager",
            "-c",
            "core.fsmonitor=false",
            "diff",
            "--no-index",
            "--text",
            "--no-renames",
            "--no-ext-diff",
            "--no-textconv",
            "--no-color",
            "--",
            &before_argument,
            &candidate_argument,
        ],
        root,
        8 * 1024 * 1024,
        true,
    )?;
    String::from_utf8(output).map_err(|_| ProjectError::Invalid("review diff is not UTF-8"))
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

pub(super) fn execute(
    program: &'static str,
    args: &[String],
    root: &Path,
    target: &Path,
    timeout: Duration,
    offline: bool,
) -> Result<(bool, Vec<u8>, Vec<u8>), ProjectError> {
    if tokio::runtime::Handle::try_current().is_ok() {
        return Err(ProjectError::Invalid(
            "project verification requires a synchronous CLI context",
        ));
    }
    let tool = tool_path(program)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async {
        let execution = async {
            let mut command = tokio::process::Command::new(tool);
            command.args(args).current_dir(root).env("CARGO_TARGET_DIR", target)
                .env("CARGO_NET_OFFLINE", if offline { "true" } else { "false" })
                .env("RUSTUP_AUTO_INSTALL", "0").env("RULLST_DISABLE_UPDATE_CHECK", "true")
                .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
            crate::generators::dev::configure_group(command.as_std_mut());
            let mut owned = crate::generators::dev::BuildChild::new(command.spawn()?)?;
            let stdout = owned.child.stdout.take().ok_or(ProjectError::Invalid("missing Cargo output"))?;
            let stderr = owned.child.stderr.take().ok_or(ProjectError::Invalid("missing Cargo diagnostics"))?;
            let (status, output, diagnostics) = tokio::try_join!(
                async { owned.wait().await.map_err(ProjectError::Io) },
                read(stdout, 8 * 1024 * 1024), read(stderr, 8 * 1024 * 1024))?;
            Ok((status.success(), output, diagnostics))
        };
        tokio::select! {
            result = tokio::time::timeout(timeout, execution) => result.map_err(|_| ProjectError::Invalid("project verification timed out; acceptance was not recorded"))?,
            result = tokio::signal::ctrl_c() => { result?; Err(ProjectError::Invalid("project verification cancelled; acceptance was not recorded")) },
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn diff_uses_paths_relative_to_the_private_cache_on_every_platform() {
        let directory = tempfile::Builder::new()
            .prefix("rullst review ")
            .tempdir()
            .unwrap();
        let base = directory.path().canonicalize().unwrap();
        let before = base.join("project-one/before");
        let candidate = base.join("project-two/candidate");
        std::fs::create_dir_all(&before).unwrap();
        std::fs::create_dir_all(&candidate).unwrap();
        std::fs::write(before.join("Cargo.toml"), "old\n").unwrap();
        std::fs::write(candidate.join("Cargo.toml"), "new\n").unwrap();
        let patch = diff(&before, &candidate).unwrap();
        assert!(patch.contains("project-one/before/Cargo.toml"), "{patch}");
        assert!(
            patch.contains("project-two/candidate/Cargo.toml"),
            "{patch}"
        );
        assert!(patch.contains("-old"));
        assert!(patch.contains("+new"));
        assert!(!patch.contains(base.to_str().unwrap()));
    }
}
