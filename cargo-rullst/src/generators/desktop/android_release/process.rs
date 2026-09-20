//! Bounded trusted build/verifier processes; captured tool logs never escape.
use super::ReleaseError;
use std::{
    path::PathBuf,
    process::{Command, Stdio},
    time::Duration,
};
use tokio::io::AsyncReadExt;

pub(super) fn tool(name: &str) -> Result<PathBuf, ReleaseError> {
    let name = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_owned()
    };
    for directory in std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .filter(|path| path.is_absolute())
    {
        let path = directory.join(&name);
        if !path.is_file() {
            continue;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if path.metadata()?.permissions().mode() & 0o111 == 0 {
                continue;
            }
        }
        // Preserve rustup's cargo proxy basename, as in the project updater.
        return Ok(path);
    }
    Err(ReleaseError::Tool)
}

pub(super) fn capture(
    mut command: Command,
    timeout: Duration,
    limit: u64,
) -> Result<Vec<u8>, ReleaseError> {
    if tokio::runtime::Handle::try_current().is_ok() {
        return Err(ReleaseError::Tool);
    }
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    crate::generators::dev::configure_group(&mut command);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async {
        let execution = async {
            let mut command = tokio::process::Command::from(command);
            command.kill_on_drop(true);
            let mut child = crate::generators::dev::BuildChild::new(command.spawn()?)?;
            let stdout = child.child.stdout.take().ok_or(ReleaseError::Tool)?;
            let stderr = child.child.stderr.take().ok_or(ReleaseError::Tool)?;
            let (status, output, _diagnostic) = tokio::try_join!(
                async { child.wait().await.map_err(ReleaseError::from) },
                read(stdout, limit), read(stderr, limit)
            )?;
            if !status.success() { return Err(ReleaseError::Tool); }
            Ok(output)
        };
        tokio::select! {
            result = tokio::time::timeout(timeout, execution) => result.map_err(|_| ReleaseError::Timeout)?,
            signal = tokio::signal::ctrl_c() => { signal?; Err(ReleaseError::Cancelled) }
        }
    })
}

async fn read(
    reader: impl tokio::io::AsyncRead + Unpin,
    limit: u64,
) -> Result<Vec<u8>, ReleaseError> {
    let mut body = Vec::new();
    reader.take(limit + 1).read_to_end(&mut body).await?;
    if body.len() as u64 > limit {
        return Err(ReleaseError::Output);
    }
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "owned process fixture; executed by the bounded capture contract"]
    fn child() {
        match std::env::var("RULLST_ANDROID_PROCESS_FIXTURE")
            .unwrap()
            .as_str()
        {
            "hang" => std::thread::sleep(Duration::from_secs(10)),
            "large" => println!("{}", "x".repeat(70000)),
            "failure" => {
                eprintln!("never-print-fixture-password");
                std::process::exit(9);
            }
            _ => println!("completed"),
        }
    }

    #[test]
    fn output_failure_and_deadline_are_bounded_without_returning_tool_diagnostics() {
        let invoke = |mode, timeout, limit| {
            let mut command = Command::new(std::env::current_exe().unwrap());
            command
                .args([
                    "--exact",
                    "generators::desktop::android_release::process::tests::child",
                    "--ignored",
                    "--nocapture",
                ])
                .env("RULLST_ANDROID_PROCESS_FIXTURE", mode);
            capture(command, timeout, limit)
        };
        let output = invoke("valid", Duration::from_secs(10), 4096).unwrap();
        assert!(String::from_utf8(output).unwrap().contains("completed"));
        assert!(matches!(
            invoke("large", Duration::from_secs(10), 1024),
            Err(ReleaseError::Output)
        ));
        let failure = invoke("failure", Duration::from_secs(10), 4096).unwrap_err();
        assert!(matches!(failure, ReleaseError::Tool));
        assert!(!failure.to_string().contains("never-print-fixture-password"));
        let start = std::time::Instant::now();
        assert!(matches!(
            invoke("hang", Duration::from_millis(200), 4096),
            Err(ReleaseError::Timeout)
        ));
        assert!(start.elapsed() < Duration::from_secs(5));
    }
}
