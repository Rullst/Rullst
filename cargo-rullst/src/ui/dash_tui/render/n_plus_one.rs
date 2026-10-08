//! Possible N+1 query warning panel and the inspector's N+1 status line.
//!
//! Only findings the application reported are drawn: the same ORM operation
//! fingerprint repeated within one request at least the reported threshold.

use super::super::metrics::Metrics;
use super::super::telemetry::{DatabaseReport, RepeatedReport};
use super::Palette;
use super::metrics::fit;
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

pub(in super::super) const N_PLUS_ONE_DOCS_URL: &str =
    "https://rullst.github.io/Rullst/book/cli_reference.html#n1-query-warning";
/// Rows of the warning panel: two findings, the hint and the docs link.
pub(super) const HEIGHT: u16 = 6;
const FINDINGS_SHOWN: usize = 2;

/// The warning panel, when the application reported at least one finding.
pub(super) fn panel(metrics: &Metrics, palette: Palette, width: u16) -> Option<Paragraph<'static>> {
    let Some(RepeatedReport::Observed { threshold, .. }) = &metrics.repeated else {
        return None;
    };
    let room = usize::from(width.saturating_sub(2));
    let mut lines = metrics
        .repeated_queries()
        .take(FINDINGS_SHOWN)
        .map(|finding| {
            let count = format!("  ×{} in one request", finding.occurrences);
            let route = format!(" ▲ {} {}  ", finding.method, finding.route);
            let used = route.chars().count() + count.chars().count();
            Line::from(vec![
                Span::styled(
                    route,
                    Style::default()
                        .fg(palette.yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    fit(&finding.fingerprint, room.saturating_sub(used)),
                    Style::default().fg(palette.cyan),
                ),
                Span::styled(count, Style::default().fg(palette.yellow)),
            ])
        })
        .collect::<Vec<_>>();
    if lines.is_empty() {
        return None;
    }
    lines.push(Line::from(vec![
        Span::styled(" Hint: ", Style::default().fg(palette.cyan)),
        Span::styled(
            fit(
                "load the related rows eagerly or batch the lookups into one query.",
                room.saturating_sub(7),
            ),
            Style::default().fg(palette.text),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(" Docs: ", Style::default().fg(palette.cyan)),
        Span::styled(
            N_PLUS_ONE_DOCS_URL,
            Style::default()
                .fg(palette.green)
                .add_modifier(Modifier::UNDERLINED),
        ),
    ]));
    let title =
        format!(" POSSIBLE N+1 QUERIES · ≥{threshold} identical ORM operations in one request ");
    Some(Paragraph::new(lines).block(neon_block_owned(title, palette)))
}

fn neon_block_owned(title: String, palette: Palette) -> Block<'static> {
    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(palette.yellow))
}

/// The inspector's N+1 detection state.
pub(super) fn status(metrics: &Metrics) -> String {
    match (&metrics.repeated, &metrics.database) {
        (None, _) => "waiting for telemetry".to_string(),
        (
            Some(RepeatedReport::Observed {
                total: 0,
                threshold,
                ..
            }),
            _,
        ) => {
            format!("none yet (≥{threshold} per request)")
        }
        (Some(RepeatedReport::Observed { total, .. }), _) => {
            format!("{total} possible N+1 since start")
        }
        (Some(RepeatedReport::NotReported), Some(DatabaseReport::Observed { .. })) => {
            "N+1 detection needs request-correlated ORM telemetry (Rullst Core 13)".to_string()
        }
        (Some(RepeatedReport::NotReported), _) => {
            "N+1 detection needs ORM query telemetry (see Database)".to_string()
        }
    }
}
