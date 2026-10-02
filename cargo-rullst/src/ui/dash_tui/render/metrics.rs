//! Live metrics row: traffic, recent requests, database and queue panels, or
//! an explicit "not available" panel. Nothing is drawn that the application
//! did not report.

use super::super::metrics::{Metrics, Source, Totals, format_duration};
use super::super::state::App;
use super::super::telemetry::{DatabaseReport, QueueReport, RequestSample};
use super::{Palette, neon_block};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
};
use std::time::Instant;

pub(in super::super) const DOCS_URL: &str =
    "https://rullst.github.io/Rullst/book/cli_reference.html#cargo-rullst-dash";

/// Rows of the metrics row for a terminal of `height` rows (0 hides it).
pub(super) fn height(height: u16) -> u16 {
    match height {
        34.. => 10,
        26..=33 => 8,
        _ => 0,
    }
}

pub(super) fn render(
    frame: &mut ratatui::Frame,
    area: Rect,
    app: &App,
    palette: Palette,
    now: Instant,
) {
    let metrics = &app.metrics;
    match metrics.source {
        Source::NotServed | Source::Rejected(_) => {
            return frame.render_widget(unavailable(metrics.source, palette), area);
        }
        Source::Waiting | Source::Unreachable if !metrics.has_data() => {
            return frame.render_widget(waiting(app.port, palette), area);
        }
        _ => {}
    }
    let wide = area.width >= 105;
    let constraints = if wide {
        vec![
            Constraint::Percentage(34),
            Constraint::Percentage(38),
            Constraint::Percentage(28),
        ]
    } else {
        vec![Constraint::Percentage(55), Constraint::Percentage(45)]
    };
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area);
    frame.render_widget(traffic(metrics, palette, now, columns[0]), columns[0]);
    if wide {
        frame.render_widget(recent(metrics, palette, columns[1]), columns[1]);
        frame.render_widget(database_and_queue(metrics, palette), columns[2]);
    } else {
        frame.render_widget(database_and_queue(metrics, palette), columns[1]);
    }
}

fn label(text: &str, palette: Palette) -> Span<'static> {
    Span::styled(format!(" {text:<9}"), Style::default().fg(palette.muted))
}

fn value(text: String, palette: Palette) -> Span<'static> {
    Span::styled(
        text,
        Style::default()
            .fg(palette.text)
            .add_modifier(Modifier::BOLD),
    )
}

fn note(text: impl Into<String>, palette: Palette) -> Span<'static> {
    Span::styled(text.into(), Style::default().fg(palette.muted))
}

fn traffic(metrics: &Metrics, palette: Palette, now: Instant, area: Rect) -> Paragraph<'static> {
    let rate = metrics
        .requests_per_second(now)
        .map_or_else(|| "—".to_string(), |rate| format!("{rate:.1}"));
    let totals = metrics.totals.unwrap_or(Totals {
        requests: 0,
        client_errors: 0,
        server_errors: 0,
    });
    let errors = metrics.error_rate(now).map_or_else(
        || {
            Line::from(vec![
                label("errors", palette),
                note("waiting for a second poll", palette),
            ])
        },
        |rate| {
            let color = if rate.server_errors > 0 {
                palette.red
            } else {
                palette.green
            };
            Line::from(vec![
                label("errors", palette),
                Span::styled(
                    format!("{} ({:.1}%)", rate.server_errors, rate.percent),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                note(format!("  5xx of {} in 60 s", rate.requests), palette),
            ])
        },
    );
    let latency = match metrics.latency(now) {
        Some(latency) => Line::from(vec![
            label("latency", palette),
            value(
                format!(
                    "p50 {} · p95 {}",
                    format_duration(latency.p50_us),
                    format_duration(latency.p95_us)
                ),
                palette,
            ),
            note(
                if metrics.sampled(now) {
                    "  sampled"
                } else {
                    ""
                },
                palette,
            ),
        ]),
        None => Line::from(vec![
            label("latency", palette),
            note("no requests in 60 s", palette),
        ]),
    };
    let title = if metrics.source == Source::Live {
        " TRAFFIC "
    } else {
        " TRAFFIC · paused "
    };
    let mut lines = vec![
        Line::from(vec![
            label("req/s", palette),
            value(rate, palette),
            note("  last 10 s", palette),
        ]),
        Line::from(vec![
            label("requests", palette),
            value(totals.requests.to_string(), palette),
            note(
                format!(
                    "  4xx {}  5xx {}",
                    totals.client_errors, totals.server_errors
                ),
                palette,
            ),
        ]),
        errors,
        latency,
    ];
    // The remaining rows hold the p95-per-poll history, newest on the right.
    let spark_rows = usize::from(area.height.saturating_sub(2)).saturating_sub(lines.len());
    let spark_width = usize::from(area.width.saturating_sub(13));
    for (index, row) in metrics
        .sparkline(spark_width, spark_rows.max(1))
        .into_iter()
        .enumerate()
    {
        let caption = if index + 1 == spark_rows.max(1) {
            "p95/poll"
        } else {
            ""
        };
        lines.push(Line::from(vec![
            label(caption, palette),
            Span::styled(row, Style::default().fg(palette.cyan)),
        ]));
    }
    Paragraph::new(lines).block(neon_block(title, palette.green, false))
}

fn recent(metrics: &Metrics, palette: Palette, area: Rect) -> Paragraph<'static> {
    let rows = usize::from(area.height.saturating_sub(2));
    let width = usize::from(area.width.saturating_sub(2));
    let mut lines = metrics
        .recent()
        .take(rows)
        .map(|sample| request_line(sample, palette, width))
        .collect::<Vec<_>>();
    if lines.is_empty() {
        lines.push(Line::from(note(
            " No requests yet (o opens the app).",
            palette,
        )));
    }
    Paragraph::new(lines).block(neon_block(" RECENT REQUESTS ", palette.blue, false))
}

fn request_line(sample: &RequestSample, palette: Palette, width: usize) -> Line<'static> {
    let color = match sample.status {
        500.. => palette.red,
        400..=499 => palette.yellow,
        300..=399 => palette.cyan,
        _ => palette.green,
    };
    let prefix = format!(
        " {} {:<7} {:>9}  ",
        sample.status,
        sample.method,
        format_duration(sample.duration_us)
    );
    let room = width.saturating_sub(prefix.chars().count());
    Line::from(vec![
        Span::styled(prefix, Style::default().fg(color)),
        Span::styled(fit(&sample.path, room), Style::default().fg(palette.text)),
    ])
}

/// Cuts `text` to `room` characters, ending a cut with `…`.
fn fit(text: &str, room: usize) -> String {
    if text.chars().count() <= room {
        return text.to_string();
    }
    let mut cut = text
        .chars()
        .take(room.saturating_sub(1))
        .collect::<String>();
    if room > 0 {
        cut.push('…');
    }
    cut
}

fn database_and_queue(metrics: &Metrics, palette: Palette) -> Paragraph<'static> {
    let mut lines = Vec::new();
    match &metrics.database {
        Some(DatabaseReport::Observed {
            queries_total,
            slow_total,
            slow_threshold_ms,
            ..
        }) => {
            lines.push(Line::from(vec![
                label("queries", palette),
                value(queries_total.to_string(), palette),
                note("  ORM, since start", palette),
            ]));
            lines.push(Line::from(vec![
                label("slow", palette),
                Span::styled(
                    slow_total.to_string(),
                    Style::default()
                        .fg(if *slow_total > 0 {
                            palette.yellow
                        } else {
                            palette.green
                        })
                        .add_modifier(Modifier::BOLD),
                ),
                note(format!("  ≥ {slow_threshold_ms} ms"), palette),
            ]));
            for slow in metrics.slow_queries().take(2) {
                let subject = match (&slow.model, &slow.table) {
                    (Some(model), _) => format!("{model}.{}", slow.operation),
                    (None, Some(table)) => format!("{table}.{}", slow.operation),
                    (None, None) => slow.operation.clone(),
                };
                lines.push(Line::from(note(
                    format!(
                        "   {} {}",
                        fit(&subject, 22),
                        format_duration(slow.duration_us)
                    ),
                    palette,
                )));
            }
        }
        report => {
            let hint = match report {
                Some(DatabaseReport::SubscriberNotInstalled) => "custom tracing subscriber",
                Some(DatabaseReport::SpansFiltered) => "RUST_LOG hides rullst_orm",
                _ => "not in this app version",
            };
            lines.push(Line::from(vec![
                label("queries", palette),
                note("not reported", palette),
            ]));
            lines.push(Line::from(note(format!("   {hint}"), palette)));
        }
    }
    let (queue, hint) = match metrics.queue {
        Some(QueueReport::Observed { pending }) => (
            vec![
                label("queue", palette),
                value(pending.to_string(), palette),
                note("  pending", palette),
            ],
            None,
        ),
        Some(QueueReport::NotConfigured) => (
            vec![label("queue", palette), note("not reported", palette)],
            Some("Server::with_dev_queue(q)"),
        ),
        Some(QueueReport::Timeout) => (
            vec![
                label("queue", palette),
                note("no answer in 250 ms", palette),
            ],
            None,
        ),
        Some(QueueReport::DriverError) => (
            vec![label("queue", palette), note("driver error", palette)],
            Some("see the application logs"),
        ),
        _ => (
            vec![label("queue", palette), note("not reported", palette)],
            None,
        ),
    };
    lines.push(Line::from(queue));
    if let Some(hint) = hint {
        lines.push(Line::from(note(format!("   {hint}"), palette)));
    }
    Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .block(neon_block(" DATABASE · QUEUE ", palette.orange, false))
}

fn unavailable(source: Source, palette: Palette) -> Paragraph<'static> {
    let what = match source {
        Source::Rejected(reason) => {
            format!("The application's telemetry response was rejected ({reason}).")
        }
        _ => "The application does not serve GET /_rullst/dev-telemetry.".to_string(),
    };
    Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                format!(" {what} "),
                Style::default()
                    .fg(palette.yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            note("No request, latency, error, database or queue numbers are shown.", palette),
        ]),
        Line::from(vec![
            Span::styled(" How to enable: ", Style::default().fg(palette.cyan)),
            Span::styled(
                "1) serve the app with rullst::Server from rullst 13 or newer (generated starters do); \
                 2) run a debug build in Development: unset RULLST_ENV/APP_ENV or set RULLST_ENV=development; \
                 3) press r to restart.",
                Style::default().fg(palette.text),
            ),
        ]),
        Line::from(vec![
            Span::styled(" Docs: ", Style::default().fg(palette.cyan)),
            Span::styled(
                DOCS_URL,
                Style::default()
                    .fg(palette.green)
                    .add_modifier(Modifier::UNDERLINED),
            ),
        ]),
    ])
    .wrap(Wrap { trim: true })
    .block(neon_block(" TELEMETRY NOT AVAILABLE ", palette.yellow, false))
}

fn waiting(port: u16, palette: Palette) -> Paragraph<'static> {
    Paragraph::new(vec![
        Line::from(Span::styled(
            format!(" Waiting for the application on 127.0.0.1:{port}..."),
            Style::default().fg(palette.yellow),
        )),
        Line::from(note(
            " Metrics appear once it answers GET /_rullst/dev-telemetry (debug builds in Development).",
            palette,
        )),
    ])
    .wrap(Wrap { trim: true })
    .block(neon_block(" LIVE METRICS ", palette.muted, false))
}

/// One-line summary for terminals too short for the metrics row.
pub(super) fn summary(metrics: &Metrics, now: Instant) -> Option<String> {
    match metrics.source {
        Source::NotServed | Source::Rejected(_) => Some(" metrics: not available (?) ".to_string()),
        Source::Live => {
            let rate = metrics
                .requests_per_second(now)
                .map_or_else(|| "—".to_string(), |rate| format!("{rate:.1}"));
            let p95 = metrics.latency(now).map_or_else(
                || "—".to_string(),
                |latency| format_duration(latency.p95_us),
            );
            let errors = metrics.totals.map_or(0, |totals| totals.server_errors);
            Some(format!(" {rate} req/s · p95 {p95} · 5xx {errors} "))
        }
        _ => None,
    }
}
