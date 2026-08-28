// Operable database-backed worker iteration for Academy automation events.

#[path = "automation_worker_events.rs"]
mod automation_worker_events;

pub fn get_files() -> Vec<(&'static str, String)> {
    let mut files = vec![(
        "src/services/automation_worker_service.rs",
        AUTOMATION_WORKER_SERVICE.to_string(),
    )];
    files.extend(automation_worker_events::get_files());
    files
}

const AUTOMATION_WORKER_SERVICE: &str = r##"use crate::services::automation_execution_service::apply_claimed_plan;
use crate::services::automation_service::{AutomationRuleInput, plan_score_automations};
use crate::services::automation_worker_event_service::validate_passive_event;
use crate::services::notification_service::deliver_claimed_achievement;
use crate::services::outbox_service::{
    ClaimedOutboxEvent, OutboxError, acknowledge, claim_next_at, fail_at,
};
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AutomationWorkerOutcome {
    Idle,
    Delivered { event_key: String, planned_actions: usize },
    Failed { event_key: String, dead_lettered: bool, reason: &'static str },
}

#[derive(Debug)]
pub enum AutomationWorkerError {
    Outbox(OutboxError),
    ClaimLost,
    InvalidPolicy,
    Clock,
    NoRuntime,
    Task(String),
}

impl std::fmt::Display for AutomationWorkerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Outbox(error) => write!(formatter, "automation worker outbox error: {error}"),
            Self::ClaimLost => formatter.write_str("automation worker no longer holds the exact claim"),
            Self::InvalidPolicy => formatter.write_str("automation worker retry policy is invalid"),
            Self::Clock => formatter.write_str("system clock is before the Unix epoch"),
            Self::NoRuntime => formatter.write_str("automation worker requires a Tokio runtime"),
            Self::Task(error) => write!(formatter, "automation worker task failed: {error}"),
        }
    }
}

impl std::error::Error for AutomationWorkerError {}

impl From<OutboxError> for AutomationWorkerError {
    fn from(error: OutboxError) -> Self { Self::Outbox(error) }
}

fn unix_now() -> Result<i64, AutomationWorkerError> {
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| AutomationWorkerError::Clock)?;
    i64::try_from(elapsed.as_secs()).map_err(|_| AutomationWorkerError::Clock)
}

#[derive(Debug, Clone)]
pub struct AutomationWorkerConfig {
    pub worker_id: String,
    pub claim_key_prefix: String,
    pub lease_seconds: i64,
    pub max_attempts: i32,
    pub retry_delay_seconds: i64,
    pub idle_delay_millis: u64,
}

impl AutomationWorkerConfig {
    fn validate(&self) -> Result<(), AutomationWorkerError> {
        if !valid_key(&self.worker_id, 64)
            || !valid_key(&self.claim_key_prefix, 96)
            || !(1..=3_600).contains(&self.lease_seconds)
            || !(1..=100).contains(&self.max_attempts)
            || !(0..=86_400).contains(&self.retry_delay_seconds)
            || !(1..=60_000).contains(&self.idle_delay_millis)
        {
            return Err(AutomationWorkerError::InvalidPolicy);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AutomationWorkerMetrics {
    pub iterations: u64,
    pub delivered: u64,
    pub failed: u64,
    pub dead_lettered: u64,
    pub idle: u64,
    pub last_event_key: Option<String>,
}

pub struct AutomationWorkerHandle {
    shutdown: tokio::sync::watch::Sender<bool>,
    task: Option<tokio::task::JoinHandle<Result<AutomationWorkerMetrics, AutomationWorkerError>>>,
}

impl AutomationWorkerHandle {
    pub async fn shutdown(mut self) -> Result<AutomationWorkerMetrics, AutomationWorkerError> {
        let _ = self.shutdown.send(true);
        let Some(task) = self.task.take() else {
            return Err(AutomationWorkerError::Task("worker task missing".to_string()));
        };
        task.await
            .map_err(|error| AutomationWorkerError::Task(error.to_string()))?
    }
}

impl Drop for AutomationWorkerHandle {
    fn drop(&mut self) {
        let _ = self.shutdown.send(true);
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

pub fn start(
    config: AutomationWorkerConfig,
) -> Result<AutomationWorkerHandle, AutomationWorkerError> {
    config.validate()?;
    let runtime =
        tokio::runtime::Handle::try_current().map_err(|_| AutomationWorkerError::NoRuntime)?;
    let (shutdown, shutdown_receiver) = tokio::sync::watch::channel(false);
    let task = runtime.spawn(run_loop(config, shutdown_receiver));
    Ok(AutomationWorkerHandle {
        shutdown,
        task: Some(task),
    })
}

async fn run_loop(
    config: AutomationWorkerConfig,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) -> Result<AutomationWorkerMetrics, AutomationWorkerError> {
    let mut metrics = AutomationWorkerMetrics::default();
    while !*shutdown.borrow() {
        metrics.iterations = metrics
            .iterations
            .checked_add(1)
            .ok_or(AutomationWorkerError::InvalidPolicy)?;
        let claim_key = format!("{}:{}", config.claim_key_prefix, metrics.iterations);
        let outcome = run_once(
            &config.worker_id,
            &claim_key,
            config.lease_seconds,
            config.max_attempts,
            config.retry_delay_seconds,
        )
        .await?;
        let should_pause = match outcome {
            AutomationWorkerOutcome::Idle => {
                metrics.idle = metrics.idle.saturating_add(1);
                true
            }
            AutomationWorkerOutcome::Delivered { event_key, .. } => {
                metrics.delivered = metrics.delivered.saturating_add(1);
                metrics.last_event_key = Some(event_key);
                false
            }
            AutomationWorkerOutcome::Failed {
                event_key,
                dead_lettered,
                ..
            } => {
                metrics.failed = metrics.failed.saturating_add(1);
                metrics.dead_lettered = metrics
                    .dead_lettered
                    .saturating_add(u64::from(dead_lettered));
                metrics.last_event_key = Some(event_key);
                true
            }
        };
        if should_pause {
            tokio::select! {
                changed = shutdown.changed() => {
                    if changed.is_err() || *shutdown.borrow() {
                        break;
                    }
                }
                _ = tokio::time::sleep(std::time::Duration::from_millis(config.idle_delay_millis)) => {}
            }
        } else {
            tokio::task::yield_now().await;
        }
    }
    Ok(metrics)
}

fn valid_key(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.')
        })
}

pub async fn run_once(
    worker_id: &str,
    claim_key: &str,
    lease_seconds: i64,
    max_attempts: i32,
    retry_delay_seconds: i64,
) -> Result<AutomationWorkerOutcome, AutomationWorkerError> {
    run_once_at(
        worker_id,
        claim_key,
        unix_now()?,
        lease_seconds,
        max_attempts,
        retry_delay_seconds,
    )
    .await
}

pub async fn run_once_at(
    worker_id: &str,
    claim_key: &str,
    now_epoch_seconds: i64,
    lease_seconds: i64,
    max_attempts: i32,
    retry_delay_seconds: i64,
) -> Result<AutomationWorkerOutcome, AutomationWorkerError> {
    if !(1..=100).contains(&max_attempts) || !(0..=86_400).contains(&retry_delay_seconds) {
        return Err(AutomationWorkerError::InvalidPolicy);
    }
    let Some(event) = claim_next_at(worker_id, claim_key, now_epoch_seconds, lease_seconds).await?
    else {
        return Ok(AutomationWorkerOutcome::Idle);
    };
    match process_claimed(&event).await {
        Ok(planned_actions) => {
            if !acknowledge(event.id, &event.claim_key).await? {
                return Err(AutomationWorkerError::ClaimLost);
            }
            Ok(AutomationWorkerOutcome::Delivered {
                event_key: event.event_key,
                planned_actions,
            })
        }
        Err(reason) => {
            if !fail_at(
                event.id,
                &event.claim_key,
                reason,
                max_attempts,
                now_epoch_seconds,
                retry_delay_seconds,
            )
            .await?
            {
                return Err(AutomationWorkerError::ClaimLost);
            }
            Ok(AutomationWorkerOutcome::Failed {
                event_key: event.event_key,
                dead_lettered: event.attempts >= max_attempts,
                reason,
            })
        }
    }
}

async fn process_claimed(event: &ClaimedOutboxEvent) -> Result<usize, &'static str> {
    if event.school_id <= 0 {
        return Err("invalid event school scope");
    }
    if let Some(planned_actions) = validate_passive_event(event)? {
        return Ok(planned_actions);
    }
    if event.event_kind == "achievement_awarded" {
        let receipt = deliver_claimed_achievement(&event.event_key, &event.claim_key)
            .await
            .map_err(|_| "notification delivery failed")?;
        return Ok(usize::from(receipt.applied));
    }
    if event.event_kind != "score_recorded" {
        return Err("unsupported event kind");
    }
    let driver = rullst::db::Orm::driver().map_err(|_| "database driver unavailable")?;
    let rules_sql = match driver {
        "postgres" => "SELECT id, enabled, trigger_kind, action_kind, config_json FROM automation_rules WHERE school_id = $1 AND enabled = $2 AND trigger_kind = $3 ORDER BY id ASC",
        _ => "SELECT id, enabled, trigger_kind, action_kind, config_json FROM automation_rules WHERE school_id = ? AND enabled = ? AND trigger_kind = ? ORDER BY id ASC",
    };
    let rows = rullst::db::sqlx::query_as::<_, (i32, i32, String, String, String)>(rules_sql)
        .bind(event.school_id)
        .bind(1_i32)
        .bind(&event.event_kind)
        .fetch_all(rullst::db::Orm::pool().map_err(|_| "database pool unavailable")?)
        .await
        .map_err(|_| "automation rule query failed")?;
    let rules = rows
        .into_iter()
        .map(|row| AutomationRuleInput {
            id: row.0,
            enabled: row.1 == 1,
            trigger_kind: row.2,
            action_kind: row.3,
            config_json: row.4,
        })
        .collect::<Vec<_>>();
    let plans = plan_score_automations(
        &event.event_key,
        &event.event_kind,
        &event.payload_json,
        &rules,
    )
    .map_err(|_| "automation planning failed")?;
    for plan in &plans {
        apply_claimed_plan(plan, &event.claim_key)
            .await
            .map_err(|_| "automation execution failed")?;
    }
    Ok(plans.len())
}
"##;

#[cfg(test)]
mod tests {
    use super::AUTOMATION_WORKER_SERVICE;

    #[test]
    fn worker_binds_processing_and_transition_to_exact_claim() {
        assert!(AUTOMATION_WORKER_SERVICE.contains("claim_next_at"));
        assert!(AUTOMATION_WORKER_SERVICE.contains("apply_claimed_plan"));
        assert!(AUTOMATION_WORKER_SERVICE.contains("acknowledge(event.id, &event.claim_key)"));
        assert!(AUTOMATION_WORKER_SERVICE.contains("dead_lettered"));
        assert!(AUTOMATION_WORKER_SERVICE.contains("validate_passive_event"));
        assert!(AUTOMATION_WORKER_SERVICE.contains("Handle::try_current"));
        assert!(AUTOMATION_WORKER_SERVICE.contains("shutdown.changed()"));
        assert!(AUTOMATION_WORKER_SERVICE.contains("task.abort()"));
        assert!(!AUTOMATION_WORKER_SERVICE.contains("format!(\"SELECT"));
    }
}
