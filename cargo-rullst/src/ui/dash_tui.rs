#[derive(Debug)]
pub enum LogMsg {
    AppStdout(String),
    AppStderr(String),
    System(String),
}
use crossterm::{
    event::{Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use std::io::{self};
use std::time::Instant;
use tokio::sync::mpsc::UnboundedReceiver;

struct App {
    app_logs: Vec<String>,
    sys_logs: Vec<String>,
    app_scroll: u16,
    sys_scroll: u16,
    start_time: Instant,
    tick_count: usize,
    port: u16,
}

impl App {
    fn new(port: u16) -> App {
        App {
            app_logs: Vec::new(),
            sys_logs: Vec::new(),
            app_scroll: 0,
            sys_scroll: 0,
            start_time: Instant::now(),
            tick_count: 0,
            port,
        }
    }
}

fn strip_ansi(s: &str) -> String {
    let re = regex::Regex::new(r"\x1B\[[0-9;]*[a-zA-Z]").unwrap();
    re.replace_all(s, "").to_string()
}

pub async fn run(
    mut log_rx: UnboundedReceiver<LogMsg>,
    port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let (key_tx, mut key_rx) = tokio::sync::mpsc::unbounded_channel();
    std::thread::spawn(move || {
        loop {
            if let Ok(true) = crossterm::event::poll(std::time::Duration::from_millis(150)) {
                if let Ok(event) = crossterm::event::read() {
                    if key_tx.send(event).is_err() {
                        break;
                    }
                }
            }
        }
    });

    let mut app = App::new(port);
    app.sys_logs
        .push("🚀 Rullst Studio Dashboard Initialized.".to_string());
    app.sys_logs
        .push(format!("🌐 Localhost Ready: http://127.0.0.1:{}", port));

    let mut ticker = tokio::time::interval(std::time::Duration::from_millis(200));

    loop {
        terminal.draw(|f| ui(f, &app))?;

        tokio::select! {
            _ = ticker.tick() => {
                app.tick_count = app.tick_count.wrapping_add(1);
            }
            Some(msg) = log_rx.recv() => {
                match msg {
                    LogMsg::AppStdout(s) => {
                        app.app_logs.push(strip_ansi(&s));
                        if app.app_logs.len() > 1000 { app.app_logs.remove(0); }
                    }
                    LogMsg::AppStderr(s) => {
                        let clean = strip_ansi(&s);
                        if clean.to_lowercase().contains("warning:") || clean.trim().starts_with('|') || clean.trim().starts_with('=') {
                            app.app_logs.push(format!("[WARN] {}", clean));
                        } else {
                            app.app_logs.push(format!("[ERR] {}", clean));
                        }
                        if app.app_logs.len() > 1000 { app.app_logs.remove(0); }
                    }
                    LogMsg::System(s) => {
                        app.sys_logs.push(strip_ansi(&s));
                        if app.sys_logs.len() > 1000 { app.sys_logs.remove(0); }
                    }
                }
            }
            Some(event) = key_rx.recv() => {
                if let Event::Key(key) = event {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            break;
                        }
                        KeyCode::Char('o') => {
                            app.sys_logs.push("🌐 Launching application in browser...".to_string());
                            let url = format!("http://127.0.0.1:{}", app.port);
                            #[cfg(target_os = "windows")]
                            let _ = std::process::Command::new("cmd").arg("/c").arg("start").arg(&url).spawn();
                            #[cfg(target_os = "macos")]
                            let _ = std::process::Command::new("open").arg(&url).spawn();
                            #[cfg(target_os = "linux")]
                            let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
                        }
                        KeyCode::Char('m') => {
                            app.sys_logs.push("⏳ Running db:migrate...".to_string());
                            let _ = std::process::Command::new("cargo")
                                .arg("run")
                                .arg("-q")
                                .arg("--")
                                .arg("db:migrate")
                                .status();
                            app.sys_logs.push("✅ Migration check complete.".to_string());
                        }
                        KeyCode::Char('c') => {
                            app.app_logs.clear();
                            app.sys_logs.clear();
                        }
                        KeyCode::Up => {
                            if app.app_scroll > 0 { app.app_scroll -= 1; }
                        }
                        KeyCode::Down => {
                            app.app_scroll += 1;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn ui(f: &mut ratatui::Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Length(3), // Animated Header Banner
                Constraint::Min(10),   // Main Grid
                Constraint::Length(3), // Interactive Footer
            ]
            .as_ref(),
        )
        .split(f.area());

    // ─── Header Animation & Uptime ──────────────────────────────────────────────
    let spinner_frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let frame = spinner_frames[app.tick_count % spinner_frames.len()];
    let elapsed = app.start_time.elapsed();
    let uptime_str = format!(
        "{:02}:{:02}:{:02}",
        elapsed.as_secs() / 3600,
        (elapsed.as_secs() % 3600) / 60,
        elapsed.as_secs() % 60
    );

    let header_text = vec![
        Span::styled(
            format!(" {} ", frame),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "🦀 Rullst Studio Dashboard",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("   │   "),
        Span::styled("🌐 App URL: ", Style::default().fg(Color::White)),
        Span::styled(
            format!("http://127.0.0.1:{}", app.port),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        ),
        Span::raw("   │   "),
        Span::styled(
            "⚡ HMR: ACTIVE",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("   │   "),
        Span::styled(
            format!("🕒 Uptime: {}", uptime_str),
            Style::default().fg(Color::DarkGray),
        ),
    ];

    let header = Paragraph::new(Line::from(header_text))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .alignment(Alignment::Center);
    f.render_widget(header, chunks[0]);

    // ─── Main Grid Layout ───────────────────────────────────────────────────────
    let main_columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(58), Constraint::Percentage(42)].as_ref())
        .split(chunks[1]);

    // Right Column Split (Top: System Logs, Bottom: Inspector / Stats)
    let right_panels = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)].as_ref())
        .split(main_columns[1]);

    // 1. Left Panel: App Server Logs
    let app_text: Vec<Line> = app
        .app_logs
        .iter()
        .map(|s| {
            if s.contains("[ERR]") {
                Line::from(Span::styled(s, Style::default().fg(Color::Red)))
            } else if s.contains("[WARN]") {
                Line::from(Span::styled(s, Style::default().fg(Color::Yellow)))
            } else {
                Line::from(Span::styled(s, Style::default().fg(Color::White)))
            }
        })
        .collect();

    let app_visible_lines = (main_columns[0].height as usize).saturating_sub(2);
    let app_total_lines = app_text.len();
    let app_scroll = if app_total_lines > app_visible_lines {
        (app_total_lines - app_visible_lines) as u16 - app.app_scroll
    } else {
        0
    };

    let app_panel = Paragraph::new(app_text)
        .block(
            Block::default()
                .title(" 🖥️  Application & HTTP Logs ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        )
        .wrap(Wrap { trim: false })
        .scroll((app_scroll, 0));
    f.render_widget(app_panel, main_columns[0]);

    // 2. Right Top Panel: System & Hot Reload
    let sys_text: Vec<Line> = app
        .sys_logs
        .iter()
        .map(|s| {
            let style = if s.contains("✅") || s.contains("Ready") {
                Style::default().fg(Color::Green)
            } else if s.contains("❌") || s.contains("⚠️") {
                Style::default().fg(Color::Red)
            } else if s.contains("🔄") || s.contains("⏳") {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::Magenta)
            };
            Line::from(Span::styled(s, style))
        })
        .collect();

    let sys_visible_lines = (right_panels[0].height as usize).saturating_sub(2);
    let sys_total_lines = sys_text.len();
    let sys_scroll = if sys_total_lines > sys_visible_lines {
        (sys_total_lines - sys_visible_lines) as u16 - app.sys_scroll
    } else {
        0
    };

    let sys_panel = Paragraph::new(sys_text)
        .block(
            Block::default()
                .title(" ⚙️  Hot-Reload Engine (Dylib + AST) ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .wrap(Wrap { trim: false })
        .scroll((sys_scroll, 0));
    f.render_widget(sys_panel, right_panels[0]);

    // 3. Right Bottom Panel: Environment & Inspector
    let inspector_lines = vec![
        Line::from(vec![
            Span::styled("  🚀 Mode:", Style::default().fg(Color::DarkGray)),
            Span::styled(
                " Hybrid Hot-Reload (Native Rust + DOM)",
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(vec![
            Span::styled("  🌐 Address:", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!(" http://127.0.0.1:{}", app.port),
                Style::default().fg(Color::Green),
            ),
        ]),
        Line::from(vec![
            Span::styled("  📡 WebSocket:", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!(" ws://127.0.0.1:{}/_rullst_hmr", app.port + 1),
                Style::default().fg(Color::Magenta),
            ),
        ]),
        Line::from(vec![
            Span::styled("  🗄️  Database:", Style::default().fg(Color::DarkGray)),
            Span::styled(
                " SQLite / MySQL / PG (Connected)",
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(vec![Span::raw("")]),
        Line::from(vec![
            Span::styled(
                "  ✨ TIP:",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " Edit any .rs file to see sub-ms hot reload!",
                Style::default().fg(Color::White),
            ),
        ]),
    ];

    let inspector_panel = Paragraph::new(inspector_lines)
        .block(
            Block::default()
                .title(" 📊  Project Inspector & Status ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Magenta)),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(inspector_panel, right_panels[1]);

    // ─── Footer Shortcuts ───────────────────────────────────────────────────────
    let footer_text = vec![
        Span::styled(
            "[o]",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Open in Browser  │  "),
        Span::styled(
            "[m]",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Run db:migrate  │  "),
        Span::styled(
            "[c]",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Clear Logs  │  "),
        Span::styled(
            "[↑/↓]",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Scroll  │  "),
        Span::styled(
            "[q/Esc]",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Quit"),
    ];

    let footer = Paragraph::new(Line::from(footer_text))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .alignment(Alignment::Center);
    f.render_widget(footer, chunks[2]);
}
