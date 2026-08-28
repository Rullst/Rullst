pub(super) const PRIVACY_REQUEST_EXECUTOR_SERVICE: &str = r##"use crate::services::privacy_request_worker_service::{
    PrivacyRequestClaim, PrivacyWorkerError, claim_next_at, complete_at, fail_at,
};
use rullst::db::async_trait;
use rullst_security::UserContext;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivacyAdapterFailure {
    pub code: String,
}

impl PrivacyAdapterFailure {
    pub fn new(code: impl Into<String>) -> Self {
        Self { code: code.into() }
    }
}

/// Product-owned fulfillment boundary.
///
/// Implementations perform the real export/deletion policy and return a
/// lowercase SHA-256 digest of their canonical, durable result evidence.
#[async_trait]
pub trait PrivacyFulfillmentAdapter: Send + Sync + 'static {
    async fn fulfill(
        &self,
        claim: &PrivacyRequestClaim,
    ) -> Result<String, PrivacyAdapterFailure>;
}

#[derive(Debug, Clone)]
enum MockOutcome {
    Complete(String),
    Fail(String),
}

/// Explicit protocol-only mock. It never exports, deletes or anonymizes data.
#[derive(Debug, Clone)]
pub struct DeterministicPrivacyMockAdapter {
    outcome: MockOutcome,
}

impl DeterministicPrivacyMockAdapter {
    pub fn complete(result_digest: impl Into<String>) -> Self {
        Self {
            outcome: MockOutcome::Complete(result_digest.into()),
        }
    }

    pub fn fail(error_code: impl Into<String>) -> Self {
        Self {
            outcome: MockOutcome::Fail(error_code.into()),
        }
    }
}

#[async_trait]
impl PrivacyFulfillmentAdapter for DeterministicPrivacyMockAdapter {
    async fn fulfill(
        &self,
        _claim: &PrivacyRequestClaim,
    ) -> Result<String, PrivacyAdapterFailure> {
        match &self.outcome {
            MockOutcome::Complete(digest) => Ok(digest.clone()),
            MockOutcome::Fail(code) => Err(PrivacyAdapterFailure::new(code.clone())),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PrivacyExecutorConfig {
    pub claim_key_prefix: String,
    pub lease_seconds: i64,
    pub adapter_timeout_seconds: u64,
    pub retry_delay_seconds: i64,
    pub max_attempts: i32,
    pub idle_delay_millis: u64,
}

impl PrivacyExecutorConfig {
    fn validate(&self) -> Result<(), PrivacyExecutorError> {
        let timeout = i64::try_from(self.adapter_timeout_seconds)
            .map_err(|_| PrivacyExecutorError::InvalidPolicy)?;
        if !valid_key(&self.claim_key_prefix, 64)
            || !(2..=3_600).contains(&self.lease_seconds)
            || timeout <= 0
            || timeout >= self.lease_seconds
            || !(0..=86_400).contains(&self.retry_delay_seconds)
            || !(1..=10).contains(&self.max_attempts)
            || !(1..=60_000).contains(&self.idle_delay_millis)
        {
            return Err(PrivacyExecutorError::InvalidPolicy);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrivacyExecutorOutcome {
    Idle,
    Completed { request_key: String },
    Failed {
        request_key: String,
        error_code: String,
        dead_lettered: bool,
    },
}

#[derive(Debug)]
pub enum PrivacyExecutorError {
    InvalidPolicy,
    Clock,
    NoRuntime,
    ClaimLost,
    Worker(PrivacyWorkerError),
    Task(String),
}

impl std::fmt::Display for PrivacyExecutorError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPolicy => formatter.write_str("privacy executor policy is invalid"),
            Self::Clock => formatter.write_str("privacy executor clock is before the Unix epoch"),
            Self::NoRuntime => formatter.write_str("privacy executor requires a Tokio runtime"),
            Self::ClaimLost => formatter.write_str("privacy executor lost its exact claim"),
            Self::Worker(error) => write!(formatter, "privacy executor worker error: {error}"),
            Self::Task(error) => write!(formatter, "privacy executor task failed: {error}"),
        }
    }
}

impl std::error::Error for PrivacyExecutorError {}

impl From<PrivacyWorkerError> for PrivacyExecutorError {
    fn from(error: PrivacyWorkerError) -> Self {
        Self::Worker(error)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PrivacyExecutorMetrics {
    pub iterations: u64,
    pub completed: u64,
    pub failed: u64,
    pub dead_lettered: u64,
    pub idle: u64,
    pub last_request_key: Option<String>,
}

pub struct PrivacyExecutorHandle {
    shutdown: tokio::sync::watch::Sender<bool>,
    task:
        Option<tokio::task::JoinHandle<Result<PrivacyExecutorMetrics, PrivacyExecutorError>>>,
}

impl PrivacyExecutorHandle {
    pub async fn shutdown(mut self) -> Result<PrivacyExecutorMetrics, PrivacyExecutorError> {
        let _ = self.shutdown.send(true);
        let Some(task) = self.task.take() else {
            return Err(PrivacyExecutorError::Task(
                "privacy executor task missing".to_string(),
            ));
        };
        task.await
            .map_err(|error| PrivacyExecutorError::Task(error.to_string()))?
    }
}

impl Drop for PrivacyExecutorHandle {
    fn drop(&mut self) {
        let _ = self.shutdown.send(true);
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

pub fn start<A: PrivacyFulfillmentAdapter>(
    context: UserContext,
    config: PrivacyExecutorConfig,
    adapter: A,
) -> Result<PrivacyExecutorHandle, PrivacyExecutorError> {
    config.validate()?;
    let runtime =
        tokio::runtime::Handle::try_current().map_err(|_| PrivacyExecutorError::NoRuntime)?;
    let (shutdown, shutdown_receiver) = tokio::sync::watch::channel(false);
    let task = runtime.spawn(run_loop(context, config, adapter, shutdown_receiver));
    Ok(PrivacyExecutorHandle {
        shutdown,
        task: Some(task),
    })
}

async fn run_loop<A: PrivacyFulfillmentAdapter>(
    context: UserContext,
    config: PrivacyExecutorConfig,
    adapter: A,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) -> Result<PrivacyExecutorMetrics, PrivacyExecutorError> {
    let mut metrics = PrivacyExecutorMetrics::default();
    while !*shutdown.borrow() {
        metrics.iterations = metrics
            .iterations
            .checked_add(1)
            .ok_or(PrivacyExecutorError::InvalidPolicy)?;
        let outcome = run_once(&context, &config, &adapter).await?;
        let should_pause = match outcome {
            PrivacyExecutorOutcome::Idle => {
                metrics.idle = metrics.idle.saturating_add(1);
                true
            }
            PrivacyExecutorOutcome::Completed { request_key } => {
                metrics.completed = metrics.completed.saturating_add(1);
                metrics.last_request_key = Some(request_key);
                false
            }
            PrivacyExecutorOutcome::Failed {
                request_key,
                dead_lettered,
                ..
            } => {
                metrics.failed = metrics.failed.saturating_add(1);
                metrics.dead_lettered = metrics
                    .dead_lettered
                    .saturating_add(u64::from(dead_lettered));
                metrics.last_request_key = Some(request_key);
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
                _ = tokio::time::sleep(
                    std::time::Duration::from_millis(config.idle_delay_millis)
                ) => {}
            }
        } else {
            tokio::task::yield_now().await;
        }
    }
    Ok(metrics)
}

pub async fn run_once<A: PrivacyFulfillmentAdapter>(
    context: &UserContext,
    config: &PrivacyExecutorConfig,
    adapter: &A,
) -> Result<PrivacyExecutorOutcome, PrivacyExecutorError> {
    let now = unix_now()?;
    let claim_key = format!(
        "{}:{}",
        config.claim_key_prefix,
        rullst::security::generate_csrf_token()
    );
    run_once_internal(context, config, adapter, &claim_key, now, true).await
}

pub async fn run_once_at<A: PrivacyFulfillmentAdapter>(
    context: &UserContext,
    config: &PrivacyExecutorConfig,
    adapter: &A,
    claim_key: &str,
    observed_at_epoch: i64,
) -> Result<PrivacyExecutorOutcome, PrivacyExecutorError> {
    run_once_internal(
        context,
        config,
        adapter,
        claim_key,
        observed_at_epoch,
        false,
    )
    .await
}

async fn run_once_internal<A: PrivacyFulfillmentAdapter>(
    context: &UserContext,
    config: &PrivacyExecutorConfig,
    adapter: &A,
    claim_key: &str,
    observed_at_epoch: i64,
    live_clock: bool,
) -> Result<PrivacyExecutorOutcome, PrivacyExecutorError> {
    config.validate()?;
    if !valid_key(claim_key, 128) || observed_at_epoch <= 0 {
        return Err(PrivacyExecutorError::InvalidPolicy);
    }
    let Some(claim) = claim_next_at(
        context,
        claim_key,
        observed_at_epoch,
        config.lease_seconds,
    )
    .await?
    else {
        return Ok(PrivacyExecutorOutcome::Idle);
    };
    let adapter_result = tokio::time::timeout(
        std::time::Duration::from_secs(config.adapter_timeout_seconds),
        adapter.fulfill(&claim),
    )
    .await;
    let transition_epoch = if live_clock {
        unix_now()?
    } else {
        observed_at_epoch
    };
    match adapter_result {
        Ok(Ok(digest)) if valid_digest(&digest) => {
            if !complete_at(
                context,
                claim.id,
                &claim.claim_key,
                transition_epoch,
                &digest,
            )
            .await?
            {
                return Err(PrivacyExecutorError::ClaimLost);
            }
            Ok(PrivacyExecutorOutcome::Completed {
                request_key: claim.request_key,
            })
        }
        result => {
            let error_code = match result {
                Err(_) => "adapter-timeout".to_string(),
                Ok(Ok(_)) => "adapter-invalid-digest".to_string(),
                Ok(Err(error)) if valid_key(&error.code, 64) => error.code,
                Ok(Err(_)) => "adapter-invalid-error-code".to_string(),
            };
            if !fail_at(
                context,
                claim.id,
                &claim.claim_key,
                &error_code,
                transition_epoch,
                config.retry_delay_seconds,
                config.max_attempts,
            )
            .await?
            {
                return Err(PrivacyExecutorError::ClaimLost);
            }
            Ok(PrivacyExecutorOutcome::Failed {
                request_key: claim.request_key,
                error_code,
                dead_lettered: claim.attempts >= config.max_attempts,
            })
        }
    }
}

fn valid_key(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':')
        })
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn unix_now() -> Result<i64, PrivacyExecutorError> {
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| PrivacyExecutorError::Clock)?;
    i64::try_from(elapsed.as_secs()).map_err(|_| PrivacyExecutorError::Clock)
}
"##;

#[cfg(test)]
mod tests {
    use super::PRIVACY_REQUEST_EXECUTOR_SERVICE;

    #[test]
    fn executor_is_static_dispatch_supervised_and_mock_is_explicit() {
        for required in [
            "pub trait PrivacyFulfillmentAdapter",
            "pub fn start<A: PrivacyFulfillmentAdapter>",
            "pub async fn run_once_at<A: PrivacyFulfillmentAdapter>",
            "adapter_timeout_seconds",
            "PrivacyExecutorHandle",
            "DeterministicPrivacyMockAdapter",
            "It never exports, deletes or anonymizes data",
        ] {
            assert!(
                PRIVACY_REQUEST_EXECUTOR_SERVICE.contains(required),
                "missing executor marker: {required}"
            );
        }
        assert!(!PRIVACY_REQUEST_EXECUTOR_SERVICE.contains("dyn PrivacyFulfillmentAdapter"));
    }
}
