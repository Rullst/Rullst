// Supervised, lease-backed activation of scheduled Academy course versions.

pub fn get_files() -> Vec<(&'static str, String)> {
    vec![(
        "src/services/publication_scheduler_service.rs",
        PUBLICATION_SCHEDULER_SERVICE.to_string(),
    )]
}

const PUBLICATION_SCHEDULER_SERVICE: &str = r##"use crate::services::publication_service::{
    PublicationError, review_version_at,
};
use crate::services::scheduler_lease_service::{
    SchedulerLeaseError, acquire_at, release, renew_at,
};
use crate::services::school_service;
use rullst_security::{RbacGuard, UserContext};

pub const PUBLICATION_LEASE_KEY: &str = "academy:course-publication";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublicationSchedulerOutcome {
    Standby,
    Completed { activated: usize },
}

#[derive(Debug)]
pub enum PublicationSchedulerError {
    InvalidPolicy,
    Forbidden,
    Clock,
    NoRuntime,
    LeaseLost,
    Lease(SchedulerLeaseError),
    Publication(PublicationError),
    Database(rullst_orm::Error),
    Task(String),
}

impl std::fmt::Display for PublicationSchedulerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPolicy => formatter.write_str("publication scheduler policy is invalid"),
            Self::Forbidden => formatter.write_str("publication scheduler access denied"),
            Self::Clock => formatter.write_str("system clock is before the Unix epoch"),
            Self::NoRuntime => formatter.write_str("publication scheduler requires a Tokio runtime"),
            Self::LeaseLost => formatter.write_str("publication scheduler lost its exact lease"),
            Self::Lease(error) => write!(formatter, "publication scheduler lease error: {error}"),
            Self::Publication(error) => write!(formatter, "publication scheduler activation error: {error}"),
            Self::Database(error) => write!(formatter, "publication scheduler database error: {error}"),
            Self::Task(error) => write!(formatter, "publication scheduler task failed: {error}"),
        }
    }
}

impl std::error::Error for PublicationSchedulerError {}

impl From<SchedulerLeaseError> for PublicationSchedulerError {
    fn from(error: SchedulerLeaseError) -> Self { Self::Lease(error) }
}

impl From<PublicationError> for PublicationSchedulerError {
    fn from(error: PublicationError) -> Self { Self::Publication(error) }
}

impl From<rullst_orm::Error> for PublicationSchedulerError {
    fn from(error: rullst_orm::Error) -> Self { Self::Database(error) }
}

#[derive(Debug, Clone)]
pub struct PublicationSchedulerConfig {
    pub holder_id: String,
    pub lease_token_prefix: String,
    pub lease_seconds: i64,
    pub poll_interval_millis: u64,
    pub batch_limit: i64,
}

impl PublicationSchedulerConfig {
    fn validate(&self) -> Result<(), PublicationSchedulerError> {
        if !valid_key(&self.holder_id, 64)
            || !valid_key(&self.lease_token_prefix, 64)
            || !(1..=3_600).contains(&self.lease_seconds)
            || !(1..=60_000).contains(&self.poll_interval_millis)
            || !(1..=100).contains(&self.batch_limit)
        {
            return Err(PublicationSchedulerError::InvalidPolicy);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PublicationSchedulerMetrics {
    pub cycles: u64,
    pub leadership_acquired: u64,
    pub standby_cycles: u64,
    pub empty_cycles: u64,
    pub activated: u64,
}

pub struct PublicationSchedulerHandle {
    shutdown: tokio::sync::watch::Sender<bool>,
    task: Option<tokio::task::JoinHandle<
        Result<PublicationSchedulerMetrics, PublicationSchedulerError>,
    >>,
}

impl PublicationSchedulerHandle {
    pub async fn shutdown(
        mut self,
    ) -> Result<PublicationSchedulerMetrics, PublicationSchedulerError> {
        let _ = self.shutdown.send(true);
        let Some(task) = self.task.take() else {
            return Err(PublicationSchedulerError::Task(
                "scheduler task missing".to_string(),
            ));
        };
        task.await
            .map_err(|error| PublicationSchedulerError::Task(error.to_string()))?
    }
}

impl Drop for PublicationSchedulerHandle {
    fn drop(&mut self) {
        let _ = self.shutdown.send(true);
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

pub fn start(
    context: UserContext,
    config: PublicationSchedulerConfig,
) -> Result<PublicationSchedulerHandle, PublicationSchedulerError> {
    config.validate()?;
    authorize_scheduler(&context)?;
    let runtime =
        tokio::runtime::Handle::try_current().map_err(|_| PublicationSchedulerError::NoRuntime)?;
    let (shutdown, shutdown_receiver) = tokio::sync::watch::channel(false);
    let task = runtime.spawn(run_loop(context, config, shutdown_receiver));
    Ok(PublicationSchedulerHandle {
        shutdown,
        task: Some(task),
    })
}

async fn run_loop(
    context: UserContext,
    config: PublicationSchedulerConfig,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) -> Result<PublicationSchedulerMetrics, PublicationSchedulerError> {
    let mut metrics = PublicationSchedulerMetrics::default();
    while !*shutdown.borrow() {
        metrics.cycles = metrics
            .cycles
            .checked_add(1)
            .ok_or(PublicationSchedulerError::InvalidPolicy)?;
        match run_cycle(&context, &config).await? {
            PublicationSchedulerOutcome::Standby => {
                metrics.standby_cycles = metrics.standby_cycles.saturating_add(1);
            }
            PublicationSchedulerOutcome::Completed { activated } => {
                metrics.leadership_acquired = metrics.leadership_acquired.saturating_add(1);
                metrics.activated = metrics
                    .activated
                    .checked_add(u64::try_from(activated).map_err(|_| {
                        PublicationSchedulerError::InvalidPolicy
                    })?)
                    .ok_or(PublicationSchedulerError::InvalidPolicy)?;
                if activated == 0 {
                    metrics.empty_cycles = metrics.empty_cycles.saturating_add(1);
                }
            }
        }
        tokio::select! {
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() {
                    break;
                }
            }
            _ = tokio::time::sleep(
                std::time::Duration::from_millis(config.poll_interval_millis)
            ) => {}
        }
    }
    Ok(metrics)
}

pub async fn run_cycle(
    context: &UserContext,
    config: &PublicationSchedulerConfig,
) -> Result<PublicationSchedulerOutcome, PublicationSchedulerError> {
    let lease_token = format!(
        "{}:{}",
        config.lease_token_prefix,
        rullst::security::generate_csrf_token()
    );
    run_cycle_internal(context, config, &lease_token, unix_now()?, true).await
}

pub async fn run_cycle_at(
    context: &UserContext,
    config: &PublicationSchedulerConfig,
    lease_token: &str,
    now_epoch_seconds: i64,
) -> Result<PublicationSchedulerOutcome, PublicationSchedulerError> {
    run_cycle_internal(
        context,
        config,
        lease_token,
        now_epoch_seconds,
        false,
    )
    .await
}

async fn run_cycle_internal(
    context: &UserContext,
    config: &PublicationSchedulerConfig,
    lease_token: &str,
    now_epoch_seconds: i64,
    live_clock: bool,
) -> Result<PublicationSchedulerOutcome, PublicationSchedulerError> {
    config.validate()?;
    authorize_scheduler(context)?;
    if !valid_key(lease_token, 128) || now_epoch_seconds <= 0 {
        return Err(PublicationSchedulerError::InvalidPolicy);
    }
    let school_id = school_service::context_school_id(context).await
        .map_err(|error| match error {
            school_service::SchoolError::Database(error) => PublicationSchedulerError::Database(error),
            _ => PublicationSchedulerError::Forbidden,
        })?;
    if !acquire_at(
        PUBLICATION_LEASE_KEY,
        &config.holder_id,
        lease_token,
        now_epoch_seconds,
        config.lease_seconds,
    )
    .await?
    {
        return Ok(PublicationSchedulerOutcome::Standby);
    }

    let cycle_result = activate_due_versions(
        context,
        config,
        lease_token,
        now_epoch_seconds,
        live_clock,
        school_id,
    )
    .await;
    let release_result = release(PUBLICATION_LEASE_KEY, &config.holder_id, lease_token).await;
    match (cycle_result, release_result) {
        (Err(error), _) => Err(error),
        (Ok(_), Ok(false)) => Err(PublicationSchedulerError::LeaseLost),
        (Ok(activated), Ok(true)) => Ok(PublicationSchedulerOutcome::Completed { activated }),
        (Ok(_), Err(error)) => Err(error.into()),
    }
}

async fn activate_due_versions(
    context: &UserContext,
    config: &PublicationSchedulerConfig,
    lease_token: &str,
    observed_at_epoch: i64,
    live_clock: bool,
    school_id: i32,
) -> Result<usize, PublicationSchedulerError> {
    let driver = rullst::db::Orm::driver()?;
    let due_sql = match driver {
        "postgres" => "SELECT cv.id FROM course_versions cv INNER JOIN course_school_scopes css ON css.course_id = cv.course_id WHERE css.school_id = $1 AND cv.status = $2 AND cv.scheduled_at_epoch > 0 AND cv.scheduled_at_epoch <= $3 ORDER BY cv.scheduled_at_epoch, cv.id LIMIT $4",
        _ => "SELECT cv.id FROM course_versions cv INNER JOIN course_school_scopes css ON css.course_id = cv.course_id WHERE css.school_id = ? AND cv.status = ? AND cv.scheduled_at_epoch > 0 AND cv.scheduled_at_epoch <= ? ORDER BY cv.scheduled_at_epoch, cv.id LIMIT ?",
    };
    let due_ids = rullst::db::sqlx::query_scalar::<_, i32>(due_sql)
        .bind(school_id)
        .bind("scheduled")
        .bind(observed_at_epoch)
        .bind(config.batch_limit)
        .fetch_all(rullst::db::Orm::pool()?)
        .await
        .map_err(|error| PublicationSchedulerError::Database(error.into()))?;
    let mut activated = 0_usize;
    for version_id in due_ids {
        let activation_epoch = if live_clock { unix_now()? } else { observed_at_epoch };
        if !renew_at(
            PUBLICATION_LEASE_KEY,
            &config.holder_id,
            lease_token,
            activation_epoch,
            config.lease_seconds,
        )
        .await?
        {
            return Err(PublicationSchedulerError::LeaseLost);
        }
        let receipt = review_version_at(context, version_id, 0, activation_epoch).await?;
        if receipt.applied && receipt.status == "published" {
            activated = activated
                .checked_add(1)
                .ok_or(PublicationSchedulerError::InvalidPolicy)?;
        }
    }
    Ok(activated)
}

fn authorize_scheduler(context: &UserContext) -> Result<(), PublicationSchedulerError> {
    RbacGuard::authorize(context, "admin").map_err(|_| PublicationSchedulerError::Forbidden)?;
    if !context.user_id.parse::<i32>().is_ok_and(|user_id| user_id > 0) {
        return Err(PublicationSchedulerError::Forbidden);
    }
    Ok(())
}

fn unix_now() -> Result<i64, PublicationSchedulerError> {
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| PublicationSchedulerError::Clock)?;
    i64::try_from(elapsed.as_secs()).map_err(|_| PublicationSchedulerError::Clock)
}

fn valid_key(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.')
        })
}
"##;

#[cfg(test)]
mod tests {
    use super::PUBLICATION_SCHEDULER_SERVICE;

    #[test]
    fn scheduler_is_supervised_bounded_and_exactly_leased() {
        assert!(PUBLICATION_SCHEDULER_SERVICE.contains("Handle::try_current"));
        assert!(PUBLICATION_SCHEDULER_SERVICE.contains("shutdown.changed()"));
        assert!(PUBLICATION_SCHEDULER_SERVICE.contains("task.abort()"));
        assert!(PUBLICATION_SCHEDULER_SERVICE.contains("renew_at("));
        assert!(PUBLICATION_SCHEDULER_SERVICE.contains("PublicationSchedulerOutcome::Standby"));
        assert!(
            PUBLICATION_SCHEDULER_SERVICE.contains("ORDER BY cv.scheduled_at_epoch, cv.id LIMIT")
        );
        assert!(PUBLICATION_SCHEDULER_SERVICE.contains("css.school_id"));
        assert!(!PUBLICATION_SCHEDULER_SERVICE.contains("format!(\"SELECT"));
    }
}
