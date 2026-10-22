//! The background poller: reads watched projects at a bounded interval,
//! keeps their history and evaluates their alert rules.

use super::alerts::{RuleKind, evaluate};
use super::history::{History, Sample};
use super::monitoring::unix_now;
use super::router::RemoteState;
use super::webhook::WebhookPayload;
use std::sync::Arc;
use std::time::Duration;

/// Shortest polling interval.
pub const MIN_POLL_INTERVAL: Duration = Duration::from_secs(15);
/// Longest polling interval.
pub const MAX_POLL_INTERVAL: Duration = Duration::from_secs(3_600);
/// Polling interval unless another is chosen.
pub const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(60);

/// `interval` within [`MIN_POLL_INTERVAL`] and [`MAX_POLL_INTERVAL`].
pub fn clamp_interval(interval: Duration) -> Duration {
    interval.clamp(MIN_POLL_INTERVAL, MAX_POLL_INTERVAL)
}

/// Reads every watched project once at time `now`, records the samples,
/// fires and delivers alerts. Returns how many projects were read.
pub(crate) async fn poll_round(state: &RemoteState, history: &History, now: u64) -> usize {
    let Ok(projects) = state.store.load() else {
        return 0;
    };
    let mut reads = tokio::task::JoinSet::new();
    for project in projects.into_iter().filter(|project| project.watch) {
        let client = state.client.clone();
        reads.spawn(async move {
            let outcome = client.fetch(&project).await;
            (project, outcome)
        });
    }
    let mut polled = 0;
    while let Some(read) = reads.join_next().await {
        let Ok((project, outcome)) = read else {
            continue;
        };
        polled += 1;
        state.monitoring.observe(&project.name, &outcome);
        let sample = Sample::from_outcome(now, &outcome);
        let previous = history.previous(&project.name, now).await.ok().flatten();
        if history.record(&project.name, &sample).await.is_err() {
            continue;
        }
        let last_fired = history.last_fired(&project.name).await.unwrap_or_default();
        let firings = evaluate(
            &project.name,
            &project.rules,
            &sample,
            previous.as_ref(),
            now,
            |rule: RuleKind| last_fired.get(rule.as_str()).copied(),
        );
        for firing in firings {
            let pending = if project.webhook.is_some() {
                "sending"
            } else if project.webhook_configured {
                "not delivered: the stored webhook cannot be decrypted"
            } else {
                "no webhook configured"
            };
            let Ok(id) = history
                .record_alert(
                    &project.name,
                    now,
                    firing.rule.as_str(),
                    &firing.message,
                    pending,
                )
                .await
            else {
                continue;
            };
            if let (Some(webhook), Some(client)) = (&project.webhook, &state.webhooks) {
                let payload = WebhookPayload {
                    source: "rullst-studio",
                    project: &project.name,
                    rule: firing.rule.as_str(),
                    message: &firing.message,
                    at: now,
                };
                let delivery = client.deliver(webhook.expose(), &payload).await;
                let _ = history.set_delivery(id, &delivery).await;
            }
        }
    }
    state
        .monitoring
        .set_open_alerts(history.unacknowledged().await.unwrap_or(0));
    polled
}

/// Polls forever at `interval` (clamped).
pub(crate) async fn run(state: Arc<RemoteState>, history: Arc<History>, interval: Duration) {
    let interval = clamp_interval(interval);
    loop {
        tokio::time::sleep(interval).await;
        poll_round(&state, &history, unix_now()).await;
    }
}
