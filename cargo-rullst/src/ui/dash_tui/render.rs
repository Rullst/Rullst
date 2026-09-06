use super::state::{App, FocusPane, LogLevel, ServerStatus, scroll_position};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
};

#[derive(Clone, Copy)]
struct Palette {
    cyan: Color,
    blue: Color,
    magenta: Color,
    orange: Color,
    green: Color,
    yellow: Color,
    red: Color,
    muted: Color,
    text: Color,
}

impl Palette {
    const fn new(enabled: bool) -> Self {
        if enabled {
            Self {
                cyan: Color::Rgb(0, 245, 255),
                blue: Color::Rgb(80, 130, 255),
                magenta: Color::Rgb(255, 45, 205),
                orange: Color::Rgb(255, 145, 35),
                green: Color::Rgb(45, 255, 155),
                yellow: Color::Rgb(255, 235, 80),
                red: Color::Rgb(255, 70, 95),
                muted: Color::Rgb(115, 130, 155),
                text: Color::Rgb(235, 242, 255),
            }
        } else {
            Self {
                cyan: Color::Reset,
                blue: Color::Reset,
                magenta: Color::Reset,
                orange: Color::Reset,
                green: Color::Reset,
                yellow: Color::Reset,
                red: Color::Reset,
                muted: Color::Reset,
                text: Color::Reset,
            }
        }
    }
}

pub(super) fn ui(frame: &mut ratatui::Frame, app: &App) {
    let palette = Palette::new(app.colors_enabled);
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(12),
            Constraint::Length(if app.search_editing || app.action_notice.is_some() {
                5
            } else {
                3
            }),
        ])
        .split(frame.area());

    render_header(frame, areas[0], app, palette);
    render_workspace(frame, areas[1], app, palette);
    render_footer(frame, areas[2], app, palette);
}

fn render_header(frame: &mut ratatui::Frame, area: Rect, app: &App, palette: Palette) {
    let frames = ["◆", "◇", "◈", "◇"];
    let pulse = if app.animations_enabled {
        frames[(app.tick_count / 2) % frames.len()]
    } else {
        "◆"
    };
    let elapsed = app.start_time.elapsed().as_secs();
    let uptime = format!(
        "{:02}:{:02}:{:02}",
        elapsed / 3_600,
        (elapsed % 3_600) / 60,
        elapsed % 60
    );
    let status_color = match app.server_status {
        ServerStatus::Starting => palette.yellow,
        ServerStatus::Ready => palette.green,
        ServerStatus::Exited { success: true, .. } => palette.muted,
        ServerStatus::Exited { success: false, .. } => palette.red,
    };
    let title = Line::from(vec![
        Span::styled(
            format!(" {pulse} "),
            Style::default()
                .fg(palette.magenta)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "RULLST",
            Style::default()
                .fg(palette.cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" // ", Style::default().fg(palette.muted)),
        Span::styled(
            "DEV CONTROL",
            Style::default()
                .fg(palette.orange)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("   APP ", Style::default().fg(palette.muted)),
        Span::styled(
            app.server_status.label(),
            Style::default()
                .fg(status_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            if app.hmr_enabled {
                "   HMR ACTIVE"
            } else {
                "   HMR DISABLED"
            },
            Style::default().fg(if app.hmr_enabled {
                palette.magenta
            } else {
                palette.muted
            }),
        ),
    ]);
    let details = Line::from(vec![
        Span::styled(
            format!(" http://127.0.0.1:{} ", app.port),
            Style::default()
                .fg(palette.green)
                .add_modifier(Modifier::UNDERLINED),
        ),
        Span::styled("│", Style::default().fg(palette.muted)),
        Span::styled(
            if app.hmr_enabled {
                " same-origin /_rullst_hmr "
            } else {
                " generate with --hot-reload "
            },
            Style::default().fg(palette.magenta),
        ),
        Span::styled("│", Style::default().fg(palette.muted)),
        Span::styled(
            format!(" uptime {uptime} "),
            Style::default().fg(palette.muted),
        ),
    ]);
    let header = Paragraph::new(vec![title, details])
        .alignment(Alignment::Center)
        .block(neon_block("", palette.cyan, true));
    frame.render_widget(header, area);
}

fn render_workspace(frame: &mut ratatui::Frame, area: Rect, app: &App, palette: Palette) {
    if area.width >= 105 {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(61), Constraint::Percentage(39)])
            .split(area);
        render_app_logs(frame, columns[0], app, palette);
        let right = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(57), Constraint::Percentage(43)])
            .split(columns[1]);
        render_system_logs(frame, right[0], app, palette);
        render_inspector(frame, right[1], app, palette);
    } else {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(area);
        render_app_logs(frame, rows[0], app, palette);
        render_system_logs(frame, rows[1], app, palette);
    }
}

fn render_app_logs(frame: &mut ratatui::Frame, area: Rect, app: &App, palette: Palette) {
    let logs = app.app_logs();
    let lines = logs
        .iter()
        .map(|entry| {
            let (prefix, color) = match entry.level {
                LogLevel::Info => ("  ", palette.text),
                LogLevel::Warning => ("▲ ", palette.yellow),
                LogLevel::Error => ("✕ ", palette.red),
            };
            Line::from(vec![
                Span::styled(prefix, Style::default().fg(color)),
                Span::styled(&entry.text, Style::default().fg(color)),
            ])
        })
        .collect::<Vec<_>>();
    let visible = area.height.saturating_sub(2) as usize;
    let scroll = scroll_position(lines.len(), visible, app.app_scroll_from_bottom);
    let focused = app.focus == FocusPane::Application;
    let title = format!(
        " APPLICATION LOGS  •  {}  •  {} shown ",
        app.filter.label(),
        lines.len()
    );
    let panel = Paragraph::new(lines)
        .block(neon_block(&title, palette.blue, focused))
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));
    frame.render_widget(panel, area);
}

fn render_system_logs(frame: &mut ratatui::Frame, area: Rect, app: &App, palette: Palette) {
    let logs = app.system_logs();
    let lines = logs
        .iter()
        .map(|entry| {
            let color = if entry.contains("failed") || entry.contains("Failed") {
                palette.red
            } else if entry.contains("complete") || entry.contains("ready") {
                palette.green
            } else if entry.contains("Running")
                || entry.contains("Checking")
                || entry.contains("API docs unavailable")
            {
                palette.yellow
            } else {
                palette.magenta
            };
            Line::from(Span::styled(*entry, Style::default().fg(color)))
        })
        .collect::<Vec<_>>();
    let visible = area.height.saturating_sub(2) as usize;
    let scroll = scroll_position(lines.len(), visible, app.system_scroll_from_bottom);
    let focused = app.focus == FocusPane::System;
    let panel = Paragraph::new(lines)
        .block(neon_block(" SYSTEM & TASKS ", palette.magenta, focused))
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));
    frame.render_widget(panel, area);
}

fn render_inspector(frame: &mut ratatui::Frame, area: Rect, app: &App, palette: Palette) {
    let migration = if app.migration_running {
        ("running", palette.yellow)
    } else {
        ("idle", palette.muted)
    };
    let studio = if app.studio_probe_running {
        ("checking", palette.yellow)
    } else {
        ("on demand", palette.muted)
    };
    let lines = vec![
        status_line("App process", app.server_status.label(), palette),
        status_line(
            "Hot reload",
            if app.hmr_enabled {
                "same-origin /_rullst_hmr"
            } else {
                "disabled"
            },
            palette,
        ),
        status_line("Database", app.database_profile.as_str(), palette),
        Line::from(vec![
            Span::styled("  Migration     ", Style::default().fg(palette.muted)),
            Span::styled(migration.0, Style::default().fg(migration.1)),
        ]),
        Line::from(vec![
            Span::styled("  Studio        ", Style::default().fg(palette.muted)),
            Span::styled(studio.0, Style::default().fg(studio.1)),
        ]),
        status_line("Log filter", app.filter.label(), palette),
        status_line(
            "Search",
            if app.search_query.is_empty() {
                "inactive"
            } else {
                &app.search_query
            },
            palette,
        ),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .block(neon_block(
                " VERIFIED PROJECT STATE ",
                palette.orange,
                false,
            ))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn status_line(label: &str, value: impl Into<String>, palette: Palette) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {label:<14}"), Style::default().fg(palette.muted)),
        Span::styled(value.into(), Style::default().fg(palette.text)),
    ])
}

fn render_footer(frame: &mut ratatui::Frame, area: Rect, app: &App, palette: Palette) {
    let shortcuts = Line::from(vec![
        key("o", palette.green),
        Span::raw(" app  "),
        key("s", palette.magenta),
        Span::raw(" studio  "),
        key("d", palette.blue),
        Span::raw(" api docs  "),
        key("m", palette.yellow),
        Span::raw(" migrate  "),
        key("/", palette.cyan),
        Span::raw(" search  "),
        key("f", palette.orange),
        Span::raw(" filter  "),
        key("tab", palette.magenta),
        Span::raw(" focus  "),
        key("q", palette.red),
        Span::raw(" quit"),
    ]);
    let mut lines = vec![shortcuts];
    if app.search_editing {
        lines.push(Line::from(vec![
            Span::styled(" search › ", Style::default().fg(palette.cyan)),
            Span::styled(
                if app.search_query.is_empty() {
                    "type to filter both log panes"
                } else {
                    &app.search_query
                },
                Style::default()
                    .fg(palette.text)
                    .add_modifier(Modifier::UNDERLINED),
            ),
            Span::styled("   Enter/Esc close", Style::default().fg(palette.muted)),
        ]));
    } else if let Some(notice) = &app.action_notice {
        lines.push(Line::from(vec![
            Span::styled(" notice › ", Style::default().fg(palette.yellow)),
            Span::styled(notice, Style::default().fg(palette.text)),
        ]));
    }
    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .block(neon_block("", palette.muted, false)),
        area,
    );
}

fn key(label: &str, color: Color) -> Span<'_> {
    Span::styled(
        format!("[{label}]"),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    )
}

fn neon_block<'a>(title: &'a str, color: Color, focused: bool) -> Block<'a> {
    let mut style = Style::default().fg(color);
    if focused {
        style = style.add_modifier(Modifier::BOLD);
    }
    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(style)
}
