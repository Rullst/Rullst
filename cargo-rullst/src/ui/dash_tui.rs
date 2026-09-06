mod render;
#[cfg(test)]
mod render_tests;
mod state;
mod terminal;

use crate::generators::dev::{DevCommand, DevStatus};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{Terminal, backend::CrosstermBackend};
use state::{App, LogLevel, ServerStatus};
use std::{
    fs::File,
    io::{self, IsTerminal, Read},
    process::{Command, ExitStatus, Stdio},
    time::Duration,
};
use tokio::sync::mpsc::{Receiver, Sender};

const ENV_READ_LIMIT: u64 = 64 * 1_024;

#[derive(Debug)]
pub enum LogMsg {
    AppStdout(String),
    AppStderr(String),
    System(String),
    MigrationFinished { success: bool, summary: String },
    StudioProbe { available: bool },
}

pub(crate) async fn run(
    mut log_rx: Receiver<LogMsg>,
    log_tx: Sender<LogMsg>,
    port: u16,
    hmr_enabled: bool,
    mut process_status: tokio::sync::watch::Receiver<DevStatus>,
    commands: Sender<DevCommand>,
) -> Result<(), Box<dyn std::error::Error>> {
    if !io::stdout().is_terminal() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "cargo rullst dash requires an interactive terminal; use `cargo rullst dev` in non-interactive environments",
        )
        .into());
    }

    let _terminal_guard = terminal::TerminalGuard::enter()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    let (key_tx, mut key_rx) = tokio::sync::mpsc::channel(32);
    std::thread::spawn(move || {
        loop {
            if key_tx.is_closed() {
                break;
            }
            if let Ok(true) = crossterm::event::poll(Duration::from_millis(150))
                && let Ok(event) = crossterm::event::read()
                && key_tx.blocking_send(event).is_err()
            {
                break;
            }
        }
    });

    let colors_enabled = std::env::var_os("NO_COLOR").is_none();
    let animations_enabled = !reduced_motion_requested();
    let mut app = App::new(
        port,
        hmr_enabled,
        detect_database_profile(),
        colors_enabled,
        animations_enabled,
    );
    app.push_system("Dashboard ready; waiting for the application port.".to_string());
    app.push_system(if hmr_enabled {
        "Auto-reload rebuilds and restarts the owned application process.".to_string()
    } else {
        "Hot reload is disabled for this project profile.".to_string()
    });

    let tick_duration = if animations_enabled {
        Duration::from_millis(120)
    } else {
        Duration::from_secs(1)
    };
    let mut ticker = tokio::time::interval(tick_duration);
    app.server_status = supervisor_status(*process_status.borrow_and_update());

    loop {
        terminal.draw(|frame| render::ui(frame, &app))?;

        tokio::select! {
            _ = ticker.tick() => {
                app.tick_count = app.tick_count.wrapping_add(1);
            }
            changed = process_status.changed() => {
                if changed.is_err() { break; }
                app.server_status = supervisor_status(*process_status.borrow_and_update());
            }
            message = log_rx.recv() => {
                if let Some(message) = message {
                    handle_log_message(&mut app, message);
                }
            }
            event = key_rx.recv() => {
                let Some(Event::Key(key)) = event else {
                    continue;
                };
                if key.kind == KeyEventKind::Press
                    && handle_key(key, &mut app, &log_tx, &commands)
                {
                    break;
                }
            }
        }
    }

    terminal.show_cursor()?;
    Ok(())
}

fn handle_log_message(app: &mut App, message: LogMsg) {
    match message {
        LogMsg::AppStdout(line) => app.push_app(LogLevel::Info, strip_ansi(&line)),
        LogMsg::AppStderr(line) => {
            let line = strip_ansi(&line);
            let lower = line.to_ascii_lowercase();
            let level = if lower.contains("warning:")
                || line.trim_start().starts_with('|')
                || line.trim_start().starts_with('=')
            {
                LogLevel::Warning
            } else {
                LogLevel::Error
            };
            app.push_app(level, line);
        }
        LogMsg::System(line) => app.push_system(strip_ansi(&line)),
        LogMsg::MigrationFinished { success, summary } => {
            app.migration_running = false;
            app.push_system(summary);
            if !success {
                app.push_app(
                    LogLevel::Error,
                    "Database migration did not complete successfully.".to_string(),
                );
            }
        }
        LogMsg::StudioProbe { available } => {
            app.studio_probe_running = false;
            if available {
                app.push_system("Studio verified on 127.0.0.1:5555; opening it.".to_string());
            } else {
                app.push_system(
                    "Studio is unavailable on 127.0.0.1:5555; check the application logs."
                        .to_string(),
                );
            }
        }
    }
}

fn handle_key(
    key: KeyEvent,
    app: &mut App,
    log_tx: &Sender<LogMsg>,
    commands: &Sender<DevCommand>,
) -> bool {
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return true;
    }
    if app.search_editing {
        match key.code {
            KeyCode::Enter | KeyCode::Esc => app.search_editing = false,
            KeyCode::Backspace => app.backspace_search(),
            KeyCode::Char(character) => app.append_search(character),
            _ => {}
        }
        return false;
    }

    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => return true,
        KeyCode::Char('o') => {
            open_browser(format!("http://127.0.0.1:{}", app.port), log_tx.clone())
        }
        KeyCode::Char('s') if !app.studio_probe_running => {
            app.studio_probe_running = true;
            app.push_system("Checking the local Studio endpoint...".to_string());
            spawn_studio_probe(log_tx.clone());
        }
        KeyCode::Char('d') => open_api_docs(app, log_tx.clone()),
        KeyCode::Char('m') if !app.migration_running => {
            if commands.try_send(DevCommand::Migrate).is_ok() {
                app.migration_running = true;
                app.push_system(
                    "Queued db:migrate for the owned application snapshot...".to_string(),
                );
            } else {
                app.push_system(
                    "Migration could not be queued; the supervisor is busy or stopped.".to_string(),
                );
            }
        }
        KeyCode::Char('c') => app.clear_logs(),
        KeyCode::Char('f') => {
            app.filter = app.filter.next();
            app.app_scroll_from_bottom = 0;
        }
        KeyCode::Char('/') => app.search_editing = true,
        KeyCode::Tab => app.focus = app.focus.next(),
        KeyCode::Up => app.scroll_older(1),
        KeyCode::Down => app.scroll_newer(1),
        KeyCode::PageUp => app.scroll_older(10),
        KeyCode::PageDown => app.scroll_newer(10),
        KeyCode::End => app.scroll_to_latest(),
        _ => {}
    }
    false
}

fn exit_status(status: ExitStatus) -> ServerStatus {
    ServerStatus::Exited {
        success: status.success(),
        code: status.code(),
    }
}

fn supervisor_status(status: DevStatus) -> ServerStatus {
    match status {
        DevStatus::Starting => ServerStatus::Starting,
        DevStatus::Ready => ServerStatus::Ready,
        DevStatus::Unverified => ServerStatus::Unverified,
        DevStatus::Exited(status) => exit_status(status),
    }
}

async fn probe_port(port: u16) -> bool {
    tokio::time::timeout(
        Duration::from_millis(120),
        tokio::net::TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, port)),
    )
    .await
    .is_ok_and(|connection| connection.is_ok())
}

fn spawn_studio_probe(log_tx: Sender<LogMsg>) {
    tokio::spawn(async move {
        let available = probe_port(5_555).await;
        if available {
            open_browser("http://127.0.0.1:5555".to_string(), log_tx.clone());
        }
        let _ = log_tx.send(LogMsg::StudioProbe { available }).await;
    });
}

fn open_api_docs(app: &mut App, log_tx: Sender<LogMsg>) {
    let spec_exists = std::path::Path::new("openapi.json").is_file();
    let controller_exists = std::path::Path::new("src/controllers/docs_controller.rs").is_file();
    if spec_exists && controller_exists {
        app.action_notice = Some("Opening API docs at /docs...".to_string());
        open_browser(format!("http://127.0.0.1:{}/docs", app.port), log_tx);
    } else {
        let notice = "API docs unavailable: start with `cargo rullst make:scalar`, then mount the router and provide openapi.json."
            .to_string();
        app.action_notice = Some(notice.clone());
        app.push_system(notice);
    }
}

fn open_browser(url: String, log_tx: Sender<LogMsg>) {
    std::thread::spawn(move || {
        let result = browser_command(&url).and_then(|mut command| command.status());
        let message = match result {
            Ok(status) if status.success() => format!("Opened {url}"),
            Ok(status) => format!("Browser opener exited with status {status} for {url}"),
            Err(error) => format!("Could not open {url}: {error}"),
        };
        let _ = log_tx.blocking_send(LogMsg::System(message));
    });
}

fn browser_command(url: &str) -> io::Result<Command> {
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("cmd");
        command.args(["/c", "start", "", url]);
        command
    };
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("open");
        command.arg(url);
        command
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut command = Command::new("xdg-open");
        command.arg(url);
        command
    };
    #[cfg(not(any(target_os = "windows", target_os = "macos", unix)))]
    return Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "opening a browser is unsupported on this platform",
    ));

    command.stdout(Stdio::null()).stderr(Stdio::null());
    Ok(command)
}

fn detect_database_profile() -> String {
    let mut contents = String::new();
    let Ok(file) = File::open(".env") else {
        return "not configured (.env missing)".to_string();
    };
    if file
        .take(ENV_READ_LIMIT)
        .read_to_string(&mut contents)
        .is_err()
    {
        return "configuration unreadable".to_string();
    }

    database_profile_from_env(&contents)
}

fn database_profile_from_env(contents: &str) -> String {
    for line in contents.lines() {
        let Some((name, raw_value)) = line.trim().split_once('=') else {
            continue;
        };
        if name.trim() != "DATABASE_URL" {
            continue;
        }
        let value = raw_value.trim().trim_matches(['"', '\'']);
        let provider = if value.starts_with("postgres://") || value.starts_with("postgresql://") {
            "PostgreSQL"
        } else if value.starts_with("mysql://") {
            "MySQL/MariaDB"
        } else if value.starts_with("sqlite:") {
            "SQLite"
        } else if value.starts_with("libsql://") {
            "Turso/libSQL"
        } else if value.is_empty() {
            "empty DATABASE_URL"
        } else {
            "custom URL"
        };
        return format!("configured: {provider}");
    }

    if contents.lines().any(|line| {
        line.trim()
            .strip_prefix("TURSO_DATABASE_URL=")
            .is_some_and(|value| !value.trim().is_empty())
    }) {
        return "configured: Turso/libSQL".to_string();
    }
    "not configured".to_string()
}

fn reduced_motion_requested() -> bool {
    reduced_motion_value(std::env::var("RULLST_REDUCED_MOTION").ok().as_deref())
}

fn reduced_motion_value(value: Option<&str>) -> bool {
    value.is_some_and(|value| matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
}

fn strip_ansi(value: &str) -> String {
    static ANSI_ESCAPE: std::sync::OnceLock<Result<regex::Regex, regex::Error>> =
        std::sync::OnceLock::new();
    match ANSI_ESCAPE.get_or_init(|| regex::Regex::new(r"\x1B\[[0-9;]*[a-zA-Z]")) {
        Ok(regex) => regex.replace_all(value, "").into_owned(),
        Err(_) => value.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ansi_sequences_are_removed_from_dashboard_logs() {
        assert_eq!(strip_ansi("\u{1b}[31mfailed\u{1b}[0m"), "failed");
    }

    #[test]
    fn database_profile_reports_configuration_without_exposing_credentials() {
        let postgres =
            database_profile_from_env("DATABASE_URL=postgres://admin:secret@127.0.0.1/app\n");
        assert_eq!(postgres, "configured: PostgreSQL");
        assert!(!postgres.contains("admin"));
        assert!(!postgres.contains("secret"));

        assert_eq!(
            database_profile_from_env("TURSO_DATABASE_URL=libsql://example.turso.io\n"),
            "configured: Turso/libSQL"
        );
        assert_eq!(
            database_profile_from_env("APP_ENV=development\n"),
            "not configured"
        );
    }

    #[test]
    fn control_c_requests_a_clean_dashboard_exit() {
        let (log_tx, _log_rx) = tokio::sync::mpsc::channel(1);
        let mut app = App::new(3_000, true, "not configured".to_string(), true, true);
        let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        let (commands, _rx) = tokio::sync::mpsc::channel(1);
        assert!(handle_key(key, &mut app, &log_tx, &commands));
    }
}
