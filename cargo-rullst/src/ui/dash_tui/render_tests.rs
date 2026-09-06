use super::{
    LogMsg, database_profile_from_env, exit_status, handle_log_message, probe_port, render,
    state::{App, FocusPane, LogFilter, LogLevel, ServerStatus},
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Terminal, backend::TestBackend};

fn handle_key(key: KeyEvent, app: &mut App, logs: &tokio::sync::mpsc::Sender<LogMsg>) -> bool {
    let (commands, _rx) = tokio::sync::mpsc::channel(1);
    super::handle_key(key, app, logs, &commands)
}

#[test]
fn migration_is_a_single_supervisor_command_and_unverified_status_is_not_ready() {
    use crate::generators::dev::{DevCommand, DevStatus};
    let (logs, _log_rx) = tokio::sync::mpsc::channel(8);
    let (commands, mut rx) = tokio::sync::mpsc::channel(1);
    let mut app = App::new(3000, true, "SQLite".into(), false, false);
    for _ in 0..2 {
        assert!(!super::handle_key(
            KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE),
            &mut app,
            &logs,
            &commands
        ));
    }
    assert!(matches!(rx.try_recv(), Ok(DevCommand::Migrate)));
    assert!(rx.try_recv().is_err());
    assert!(app.migration_running);
    app.server_status = super::supervisor_status(DevStatus::Unverified);
    assert_eq!(app.server_status, ServerStatus::Unverified);
    assert!(rendered(&app, 104, 30).contains("UNVERIFIED"));
    assert_eq!(
        super::supervisor_status(DevStatus::Ready),
        ServerStatus::Ready
    );
    assert_eq!(
        super::supervisor_status(DevStatus::Starting),
        ServerStatus::Starting
    );
}

#[tokio::test]
async fn non_interactive_dashboard_fails_with_actionable_guidance() {
    let (_log_tx, log_rx) = tokio::sync::mpsc::channel(1);
    let (log_tx, _logs) = tokio::sync::mpsc::channel(1);
    let (_status_tx, status_rx) =
        tokio::sync::watch::channel(crate::generators::dev::DevStatus::Starting);
    let (commands, _command_rx) = tokio::sync::mpsc::channel(1);
    let error = super::run(log_rx, log_tx, 3_000, true, status_rx, commands)
        .await
        .expect_err("the test process has no interactive stdout");
    assert!(
        error
            .to_string()
            .contains("requires an interactive terminal")
    );
}

fn rendered(app: &App, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    terminal
        .draw(|frame| render::ui(frame, app))
        .expect("dashboard draw");
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>()
}

#[test]
fn wide_dashboard_renders_live_state_logs_search_and_inspector() {
    let mut app = App::new(3_000, true, "configured: SQLite".to_string(), true, true);
    app.server_status = ServerStatus::Ready;
    app.migration_running = true;
    app.studio_probe_running = true;
    app.search_editing = true;
    app.search_query = "request".to_string();
    app.focus = FocusPane::System;
    app.filter = LogFilter::WarningsAndErrors;
    app.tick_count = 4;
    app.push_app(LogLevel::Info, "request complete".to_string());
    app.push_app(LogLevel::Warning, "request warning".to_string());
    app.push_app(LogLevel::Error, "request failed".to_string());
    app.push_system("worker failed".to_string());
    app.push_system("migration complete".to_string());
    app.push_system("Running verification".to_string());
    app.push_system("neutral request".to_string());

    let output = rendered(&app, 140, 40);
    for expected in [
        "RULLST",
        "DEV CONTROL",
        "READY",
        "AUTO-RELOAD",
        "request warning",
        "request failed",
        "VERIFIED PROJECT STATE",
        "configured: SQLite",
        "search",
    ] {
        assert!(
            output.contains(expected),
            "missing rendered text: {expected}"
        );
    }
}

#[test]
fn compact_dashboard_renders_no_color_static_and_exit_states() {
    let mut app = App::new(4_000, false, "not configured".to_string(), false, false);
    app.server_status = ServerStatus::Exited {
        success: true,
        code: Some(0),
    };
    app.push_app(LogLevel::Info, "server stopped".to_string());
    app.push_system("waiting".to_string());

    let stopped = rendered(&app, 80, 26);
    assert!(stopped.contains("EXITED (0)"));
    assert!(stopped.contains("RELOAD OFF"));
    assert!(stopped.contains("server stopped"));

    app.server_status = ServerStatus::Exited {
        success: false,
        code: None,
    };
    app.search_editing = true;
    let failed = rendered(&app, 104, 30);
    assert!(failed.contains("FAILED (signal)"));
    assert!(failed.contains("type to filter both log panes"));
}

#[test]
fn docs_shortcut_feedback_stays_visible_when_system_logs_are_full() {
    let (log_tx, _log_rx) = tokio::sync::mpsc::channel(8);
    let mut app = App::new(3_000, true, "configured: SQLite".to_string(), true, true);
    for index in 0..5 {
        app.push_system(format!(
            "Earlier system event {index} contains enough detail to wrap across the compact dashboard panel."
        ));
    }

    assert!(!handle_key(
        KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE),
        &mut app,
        &log_tx
    ));

    let output = rendered(&app, 104, 22);
    assert!(output.contains("API docs unavailable"));
    assert!(output.contains("make:scalar"));
    assert!(app.action_notice.is_some());
}

#[test]
fn log_messages_and_keyboard_navigation_update_only_bounded_state() {
    let (log_tx, _log_rx) = tokio::sync::mpsc::channel(8);
    let mut app = App::new(3_000, true, "configured: SQLite".to_string(), true, true);

    for message in [
        LogMsg::AppStdout("\u{1b}[32mready\u{1b}[0m".to_string()),
        LogMsg::AppStderr("warning: slow".to_string()),
        LogMsg::AppStderr("fatal".to_string()),
        LogMsg::System("\u{1b}[36mchecking\u{1b}[0m".to_string()),
        LogMsg::MigrationFinished {
            success: false,
            summary: "migration failed".to_string(),
        },
        LogMsg::StudioProbe { available: true },
        LogMsg::StudioProbe { available: false },
    ] {
        handle_log_message(&mut app, message);
    }
    assert_eq!(app.app_logs().len(), 4);
    assert!(
        app.system_logs()
            .iter()
            .any(|line| line.contains("migration failed"))
    );
    assert!(!app.migration_running);
    assert!(!app.studio_probe_running);

    for key in [
        KeyCode::Char('f'),
        KeyCode::Tab,
        KeyCode::Up,
        KeyCode::PageUp,
        KeyCode::Down,
        KeyCode::PageDown,
        KeyCode::End,
        KeyCode::Char('/'),
    ] {
        assert!(!handle_key(
            KeyEvent::new(key, KeyModifiers::NONE),
            &mut app,
            &log_tx
        ));
    }
    assert!(app.search_editing);
    for key in [KeyCode::Char('x'), KeyCode::Backspace, KeyCode::Enter] {
        assert!(!handle_key(
            KeyEvent::new(key, KeyModifiers::NONE),
            &mut app,
            &log_tx
        ));
    }
    assert!(!app.search_editing);
    assert!(!handle_key(
        KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE),
        &mut app,
        &log_tx
    ));
    assert!(
        app.system_logs()
            .iter()
            .any(|line| line.contains("API docs unavailable") && line.contains("make:scalar"))
    );
    assert!(handle_key(
        KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
        &mut app,
        &log_tx
    ));
    app.clear_logs();
    assert!(app.app_logs().is_empty());
    assert!(app.system_logs().is_empty());
}

#[test]
fn busy_migration_search_and_clear_shortcuts_remain_bounded() {
    let (log_tx, _log_rx) = tokio::sync::mpsc::channel(8);
    let (commands, command_rx) = tokio::sync::mpsc::channel(1);
    drop(command_rx);
    let mut app = App::new(3_000, true, "configured: SQLite".to_string(), true, true);
    app.push_app(LogLevel::Info, "application log".to_string());

    assert!(!super::handle_key(
        KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE),
        &mut app,
        &log_tx,
        &commands,
    ));
    assert!(!app.migration_running);
    assert!(
        app.system_logs()
            .iter()
            .any(|line| line.contains("could not be queued"))
    );

    app.search_editing = true;
    assert!(!super::handle_key(
        KeyEvent::new(KeyCode::Left, KeyModifiers::NONE),
        &mut app,
        &log_tx,
        &commands,
    ));
    assert!(app.search_editing);
    assert!(!super::handle_key(
        KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
        &mut app,
        &log_tx,
        &commands,
    ));
    assert!(!app.search_editing);

    assert!(!super::handle_key(
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE),
        &mut app,
        &log_tx,
        &commands,
    ));
    assert!(app.app_logs().is_empty());
    assert!(app.system_logs().is_empty());
    assert!(!super::handle_key(
        KeyEvent::new(KeyCode::F(12), KeyModifiers::NONE),
        &mut app,
        &log_tx,
        &commands,
    ));
}

#[test]
fn platform_browser_command_and_motion_policy_are_explicit() {
    let command = super::browser_command("https://example.invalid/docs")
        .expect("supported desktop browser command");
    #[cfg(all(unix, not(target_os = "macos")))]
    assert_eq!(command.get_program(), "xdg-open");
    #[cfg(target_os = "macos")]
    assert_eq!(command.get_program(), "open");
    #[cfg(target_os = "windows")]
    assert_eq!(command.get_program(), "cmd");

    assert!(!super::reduced_motion_value(None));
    assert!(!super::reduced_motion_value(Some("0")));
    for value in ["1", "TRUE", "Yes"] {
        assert!(super::reduced_motion_value(Some(value)));
    }
}

#[tokio::test]
async fn probes_and_supervised_process_exit_paths_have_real_effects() {
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .await
        .expect("test listener");
    let port = listener.local_addr().expect("listener address").port();
    assert!(probe_port(port).await);
    drop(listener);
    // Port zero is reserved for requesting an ephemeral bind and cannot be a
    // reachable TCP service. Reusing the released ephemeral port is racy with
    // unrelated concurrent tests and local processes.
    assert!(!probe_port(0).await);

    #[cfg(unix)]
    {
        let mut child = std::process::Command::new("/bin/sh")
            .args(["-c", "exit 7"])
            .spawn()
            .expect("short-lived child");
        let status = child.wait().expect("child exit");
        assert!(matches!(
            super::supervisor_status(crate::generators::dev::DevStatus::Exited(status)),
            ServerStatus::Exited {
                success: false,
                code: Some(7)
            }
        ));
    }

    #[cfg(unix)]
    let successful = std::process::Command::new("true")
        .status()
        .expect("successful status");
    #[cfg(windows)]
    let successful = std::process::Command::new("cmd")
        .args(["/c", "exit", "0"])
        .status()
        .expect("successful status");
    assert!(matches!(
        exit_status(successful),
        ServerStatus::Exited {
            success: true,
            code: Some(0)
        }
    ));
}

#[test]
fn database_profile_classifies_every_supported_url_without_credentials() {
    for (environment, expected) in [
        (
            "DATABASE_URL=postgresql://user:secret@host/db",
            "configured: PostgreSQL",
        ),
        (
            "DATABASE_URL=mysql://user:secret@host/db",
            "configured: MySQL/MariaDB",
        ),
        ("DATABASE_URL=sqlite://app.db", "configured: SQLite"),
        (
            "DATABASE_URL=libsql://tenant.turso.io",
            "configured: Turso/libSQL",
        ),
        ("DATABASE_URL=", "configured: empty DATABASE_URL"),
        ("DATABASE_URL=custom://opaque", "configured: custom URL"),
    ] {
        let profile = database_profile_from_env(environment);
        assert_eq!(profile, expected);
        assert!(!profile.contains("secret"));
    }
}

#[test]
fn non_interactive_branding_uses_the_accessible_static_fallback() {
    assert!(super::super::dashboard_brand::print_neon_logo().is_ok());
    assert!(super::super::dashboard_brand::play_launch_pulse().is_ok());
    assert_eq!(
        super::super::dashboard_brand::menu_icon("◆", (65, 255, 170)),
        "◆"
    );
}
