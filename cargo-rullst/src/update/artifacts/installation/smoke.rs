use super::{ArtifactError, Manifest, state};
use std::{path::Path, process::Stdio, time::Duration};
use tokio::io::AsyncReadExt;

pub(super) fn verify(directory: &Path, manifest: &Manifest) -> Result<(), ArtifactError> {
    if tokio::runtime::Handle::try_current().is_ok() {
        return Err(ArtifactError::Invalid(
            "installation requires a synchronous CLI context",
        ));
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    for name in state::executable_names(&manifest.target) {
        let program = directory.join(name);
        let (status, output, errors) = runtime.block_on(async {
            let operation = async {
                let mut command = tokio::process::Command::new(program);
                command.arg("--version").current_dir(directory)
                    .env("RULLST_DISABLE_UPDATE_CHECK", "true").env("CARGO_NET_OFFLINE", "true")
                    .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
                crate::generators::dev::configure_group(command.as_std_mut());
                let mut owned = crate::generators::dev::BuildChild::new(command.spawn()?)?;
                let stdout = owned.child.stdout.take().ok_or(ArtifactError::Invalid("version stdout missing"))?;
                let stderr = owned.child.stderr.take().ok_or(ArtifactError::Invalid("version stderr missing"))?;
                tokio::try_join!(async { owned.wait().await.map_err(ArtifactError::Io) }, bounded(stdout), bounded(stderr))
            };
            tokio::select! {
                result = tokio::time::timeout(Duration::from_secs(15), operation) => result.map_err(|_| ArtifactError::Invalid("candidate version check timed out; installation was not committed"))?,
                result = tokio::signal::ctrl_c() => { result?; Err(ArtifactError::Invalid("installation cancelled before commit")) },
            }
        })?;
        let text = std::str::from_utf8(&output)
            .map_err(|_| ArtifactError::Invalid("candidate version is not UTF-8"))?;
        let fields = text.split_whitespace().collect::<Vec<_>>();
        if !status.success()
            || !errors.is_empty()
            || fields.len() != 2
            || !matches!(fields[0], "rullst" | "cargo-rullst")
            || fields[1] != manifest.version
        {
            return Err(ArtifactError::Invalid(
                "candidate did not report the reviewed CLI version",
            ));
        }
    }
    Ok(())
}

async fn bounded(reader: impl tokio::io::AsyncRead + Unpin) -> Result<Vec<u8>, ArtifactError> {
    let mut body = Vec::new();
    reader.take(4097).read_to_end(&mut body).await?;
    if body.len() > 4096 {
        return Err(ArtifactError::Invalid(
            "candidate version output exceeds 4 KiB",
        ));
    }
    Ok(body)
}
