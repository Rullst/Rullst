//! The "next steps" block printed after `cargo rullst new`: from the new
//! directory to the generated welcome page in three commands.

use super::wizard::summary::shell_word;
use crate::ui::screen::{Line, Tone};
use std::path::Path;

const DEFAULT_PORT: u16 = 3000;

fn valid_port(value: &str) -> Option<u16> {
    value.trim().parse::<u16>().ok().filter(|port| *port > 0)
}

/// The port `cargo rullst dev` serves on: the process `PORT`, then `.env`,
/// then `[app].port` in `Rullst.toml`, then 3000. Invalid values are skipped
/// here; `dev` reports them when it starts.
pub(crate) fn resolve_port(
    process: Option<&str>,
    dotenv: Option<&str>,
    config: Option<i64>,
) -> u16 {
    process
        .and_then(valid_port)
        .or_else(|| dotenv.and_then(valid_port))
        .or_else(|| config.and_then(|port| u16::try_from(port).ok().filter(|port| *port > 0)))
        .unwrap_or(DEFAULT_PORT)
}

/// [`resolve_port`] for the project at `root` and the current environment.
pub(crate) fn app_port(root: &Path) -> u16 {
    let process = std::env::var("PORT").ok();
    let dotenv = std::fs::read(root.join(".env")).ok().and_then(|bytes| {
        dotenvy::from_read_iter(bytes.as_slice())
            .filter_map(Result::ok)
            .find(|(key, _)| key == "PORT")
            .map(|(_, value)| value)
    });
    let config = std::fs::read_to_string(root.join("Rullst.toml"))
        .ok()
        .and_then(|source| toml::from_str::<toml::Value>(&source).ok())
        .and_then(|config| config.get("app")?.get("port")?.as_integer());
    resolve_port(process.as_deref(), dotenv.as_deref(), config)
}

/// The numbered commands from the new project directory to its welcome page.
pub(crate) fn next_steps(destination: &Path, api: bool, migrates: bool, port: u16) -> Vec<Line> {
    let directory = shell_word(&destination.display().to_string());
    let serve = if migrates {
        "builds, applies migrations and serves with live reload"
    } else {
        "builds and serves with live reload"
    };
    let (visit, page) = if api {
        (
            format!("curl http://127.0.0.1:{port}/"),
            "the JSON welcome response",
        )
    } else {
        (
            format!("open http://127.0.0.1:{port}"),
            "your generated welcome page",
        )
    };
    let steps = [
        (format!("cd {directory}"), ""),
        ("cargo rullst dev".to_string(), serve),
        (visit, page),
    ];
    let width = steps
        .iter()
        .map(|(command, _)| command.chars().count())
        .max()
        .unwrap_or(0)
        + 3;
    let mut lines = vec![Line::new(), Line::new().push(Tone::Strong, "Next steps")];
    for (number, (command, purpose)) in steps.into_iter().enumerate() {
        let command = if purpose.is_empty() {
            command
        } else {
            format!("{command:<width$}")
        };
        lines.push(
            Line::new()
                .push(Tone::Accent, format!("  {}  ", number + 1))
                .push(Tone::Value, command)
                .push(Tone::Muted, purpose),
        );
    }
    lines.push(
        Line::new()
            .push(Tone::Muted, "  Prefer a live dashboard? ")
            .push(Tone::Value, "cargo rullst dash")
            .push(Tone::Muted, " · new to Rullst? ")
            .push(Tone::Value, "cargo rullst tour"),
    );
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn port_follows_dev_precedence_and_skips_invalid_values() {
        assert_eq!(resolve_port(None, None, None), 3000);
        assert_eq!(resolve_port(Some("4100"), Some("4200"), Some(4300)), 4100);
        assert_eq!(resolve_port(None, Some(" 4200 "), Some(4300)), 4200);
        assert_eq!(resolve_port(None, None, Some(4300)), 4300);
        assert_eq!(resolve_port(Some("0"), Some("x"), Some(70_000)), 3000);
        assert_eq!(resolve_port(Some("http"), None, Some(8080)), 8080);
    }

    #[test]
    fn generated_configuration_reads_dotenv_and_rullst_toml() {
        let root = tempfile::tempdir().unwrap();
        // The process PORT wins, so only check the files when it is unset.
        if std::env::var_os("PORT").is_none() {
            assert_eq!(app_port(root.path()), 3000);
            std::fs::write(root.path().join("Rullst.toml"), "[app]\nport = 4300\n").unwrap();
            assert_eq!(app_port(root.path()), 4300);
            std::fs::write(root.path().join(".env"), "A=1\nPORT=4200\n").unwrap();
            assert_eq!(app_port(root.path()), 4200);
        }
    }

    #[test]
    fn next_steps_end_on_the_welcome_page_or_the_json_root() {
        let text = |lines: Vec<Line>| lines.iter().map(Line::text).collect::<Vec<_>>();
        let html = text(next_steps(Path::new("my_app"), false, true, 3000));
        assert_eq!(html[1], "Next steps");
        assert_eq!(html[2], "  1  cd my_app");
        assert_eq!(
            html[3],
            format!(
                "  2  {:<29}builds, applies migrations and serves with live reload",
                "cargo rullst dev"
            )
        );
        assert_eq!(
            html[4],
            "  3  open http://127.0.0.1:3000   your generated welcome page"
        );
        assert!(html[5].contains("cargo rullst dash") && html[5].contains("cargo rullst tour"));

        let api = text(next_steps(Path::new("../apps/my api"), true, false, 8080));
        assert_eq!(api[2], "  1  cd '../apps/my api'");
        assert!(api[3].ends_with("builds and serves with live reload"));
        assert!(api[4].starts_with("  3  curl http://127.0.0.1:8080/"));
    }
}
