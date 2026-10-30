//! `rullst-studio`: the standalone Studio for remote projects.
//!
//! Serves Studio on `127.0.0.1` without an application process and reads the
//! read-only monitoring endpoint (`/_rullst/monitor/v1`) of the projects you
//! add. Install with `cargo install rullst-studio --version =<release> --features remote --locked`
//! (name the version while v13 is a prerelease).

use rullst_studio::remote::{
    DEFAULT_REMOTE_PORT, RemoteStudioOptions, StudioPaths, run_remote_studio,
};
use std::process::ExitCode;

const USAGE: &str = "Usage: rullst-studio [--port PORT]

Serves Rullst Studio for remote projects on http://127.0.0.1:PORT (default 5556).
Projects are stored in your configuration directory, their tokens encrypted
with a key in your data directory. Studio only ever listens on 127.0.0.1.";

fn parse_port(arguments: &[String]) -> Result<Option<u16>, String> {
    match arguments {
        [] => Ok(None),
        [flag] if flag == "--help" || flag == "-h" => Err(String::new()),
        [flag, value] if flag == "--port" => value
            .parse::<u16>()
            .ok()
            .filter(|port| *port != 0)
            .map(Some)
            .ok_or_else(|| format!("invalid port `{value}`")),
        _ => Err("unexpected arguments".to_string()),
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    // aws-lc-rs is the rustls provider of every Rullst TLS connection; the
    // project poller's HTTPS client reads the process default.
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if arguments
        .first()
        .is_some_and(|argument| argument == "--version")
    {
        println!("rullst-studio {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }
    let port = match parse_port(&arguments) {
        Ok(port) => port.unwrap_or(DEFAULT_REMOTE_PORT),
        Err(problem) if problem.is_empty() => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Err(problem) => {
            eprintln!("rullst-studio: {problem}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    let Some(paths) = StudioPaths::from_env() else {
        eprintln!(
            "rullst-studio: no user configuration directory; set HOME (APPDATA and LOCALAPPDATA on Windows) to an absolute path"
        );
        return ExitCode::FAILURE;
    };
    println!("Rullst Studio (remote projects) on http://127.0.0.1:{port}/studio");
    println!("Project list: {}", paths.projects.display());
    match run_remote_studio(RemoteStudioOptions::new(paths).with_port(port)).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("rullst-studio: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse_port;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn the_port_is_the_only_option() {
        assert_eq!(parse_port(&args(&[])), Ok(None));
        assert_eq!(parse_port(&args(&["--port", "7000"])), Ok(Some(7000)));
        assert_eq!(parse_port(&args(&["--help"])), Err(String::new()));
        assert_eq!(parse_port(&args(&["-h"])), Err(String::new()));
        assert_eq!(
            parse_port(&args(&["--port"])),
            Err("unexpected arguments".to_string())
        );
        assert_eq!(
            parse_port(&args(&["--port", "0"])),
            Err("invalid port `0`".to_string())
        );
        assert_eq!(
            parse_port(&args(&["--port", "x"])),
            Err("invalid port `x`".to_string())
        );
        assert_eq!(
            parse_port(&args(&["--host", "0.0.0.0"])),
            Err("unexpected arguments".to_string())
        );
    }
}
