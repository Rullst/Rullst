//! `cargo rullst studio --remote`: starts the standalone Studio for remote
//! projects, the `rullst-studio` binary of the `rullst-studio` crate (built
//! with its `remote` feature).

use std::io;
use std::process::Command;

const BINARY: &str = "rullst-studio";

/// Arguments passed to `rullst-studio`.
pub(super) fn remote_args(port: Option<u16>) -> Vec<String> {
    port.map(|port| vec!["--port".to_string(), port.to_string()])
        .unwrap_or_default()
}

/// The error for a binary that is not on `PATH`, or exited unsuccessfully.
pub(super) fn launch_error(outcome: io::Result<std::process::ExitStatus>) -> Result<(), io::Error> {
    match outcome {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(io::Error::other(format!("{BINARY} exited with {status}"))),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "{BINARY} is not installed; install it with `cargo install {BINARY} --features remote --locked`"
            ),
        )),
        Err(error) => Err(error),
    }
}

/// Runs `rullst-studio` in the foreground until it stops.
pub(super) fn run_remote(port: Option<u16>) -> Result<(), io::Error> {
    launch_error(Command::new(BINARY).args(remote_args(port)).status())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_port_is_forwarded_and_a_missing_binary_says_how_to_install_it() {
        assert!(remote_args(None).is_empty());
        assert_eq!(remote_args(Some(7000)), ["--port", "7000"]);
        let missing =
            launch_error(Err(io::Error::new(io::ErrorKind::NotFound, "no such file"))).unwrap_err();
        assert_eq!(missing.kind(), io::ErrorKind::NotFound);
        assert_eq!(
            missing.to_string(),
            "rullst-studio is not installed; install it with `cargo install rullst-studio --features remote --locked`"
        );
        let denied = launch_error(Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "denied",
        )))
        .unwrap_err();
        assert_eq!(denied.kind(), io::ErrorKind::PermissionDenied);
    }
}
