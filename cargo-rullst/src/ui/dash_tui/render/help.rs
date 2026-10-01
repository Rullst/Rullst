//! The `?` keyboard and metrics reference, drawn over the dashboard.

use super::metrics::DOCS_URL;
use super::{Palette, neon_block};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, Paragraph, Wrap},
};

const KEYS: [(&str, &str); 12] = [
    ("r", "restart the application (same build)"),
    ("o", "open the application in a browser"),
    ("s", "open Studio when it is reachable"),
    ("d", "open the API docs (Scalar)"),
    ("m", "run db:migrate"),
    ("/", "search both log panes"),
    ("f", "cycle the log filter"),
    ("Tab", "switch the focused log pane"),
    ("↑ ↓ PgUp PgDn End", "scroll the focused pane"),
    ("c", "clear the dashboard logs"),
    ("?", "show or close this help"),
    ("q  Esc  Ctrl+C", "quit and stop the application"),
];

pub(super) fn render(frame: &mut ratatui::Frame, area: Rect, palette: Palette) {
    let width = area.width.saturating_sub(4).min(78);
    let height = area.height.saturating_sub(2).min(22);
    let popup = Rect {
        x: area.x + (area.width.saturating_sub(width)) / 2,
        y: area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    };
    let mut lines = KEYS
        .iter()
        .map(|(key, action)| {
            Line::from(vec![
                Span::styled(
                    format!(" {key:<18}"),
                    Style::default()
                        .fg(palette.cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(*action, Style::default().fg(palette.text)),
            ])
        })
        .collect::<Vec<_>>();
    lines.extend([
        Line::default(),
        Line::from(Span::styled(
            " Metrics come from GET /_rullst/dev-telemetry on 127.0.0.1, served only by debug \
             builds in Development started by dev/dash. Unreported values stay hidden.",
            Style::default().fg(palette.muted),
        )),
        Line::from(vec![
            Span::styled(" Docs: ", Style::default().fg(palette.muted)),
            Span::styled(DOCS_URL, Style::default().fg(palette.green)),
        ]),
        Line::from(Span::styled(
            " q quits; any other key closes this help.",
            Style::default().fg(palette.muted),
        )),
    ]);
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(neon_block(" HELP ", palette.magenta, true)),
        popup,
    );
}
