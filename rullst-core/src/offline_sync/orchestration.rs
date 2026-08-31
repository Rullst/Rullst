use super::policy::MAX_OFFLINE_PUSH_BATCH;
use super::{
    OfflineAccountId, OfflineSyncError, OfflineSyncPolicy, OfflineSyncState, SyncCursor,
    SyncPullPage, SyncPushBatch, SyncPushResult,
};
use std::future::Future;
use std::time::Duration;
use thiserror::Error;

const MAX_PUSH_BATCHES_PER_RUN: usize = 20;
const MAX_PULL_PAGES_PER_RUN: usize = 100;
const MAX_REQUEST_TIMEOUT_MILLIS: u64 = 120_000;

/// Bounded orchestration policy for one foreground synchronization attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct OfflineSyncRunPolicy {
    push_batch_size: usize,
    max_push_batches: usize,
    max_pull_pages: usize,
    request_timeout_millis: u64,
}

impl OfflineSyncRunPolicy {
    /// Creates a bounded run policy.
    pub const fn new(
        push_batch_size: usize,
        max_push_batches: usize,
        max_pull_pages: usize,
        request_timeout_millis: u64,
    ) -> Result<Self, OfflineSyncError> {
        if push_batch_size == 0
            || push_batch_size > MAX_OFFLINE_PUSH_BATCH
            || max_push_batches == 0
            || max_push_batches > MAX_PUSH_BATCHES_PER_RUN
            || max_pull_pages == 0
            || max_pull_pages > MAX_PULL_PAGES_PER_RUN
            || request_timeout_millis == 0
            || request_timeout_millis > MAX_REQUEST_TIMEOUT_MILLIS
        {
            return Err(OfflineSyncError::InvalidRunPolicy);
        }
        Ok(Self {
            push_batch_size,
            max_push_batches,
            max_pull_pages,
            request_timeout_millis,
        })
    }

    /// Returns the maximum mutations submitted in one request.
    pub const fn push_batch_size(self) -> usize {
        self.push_batch_size
    }

    /// Returns the maximum push requests issued in one run.
    pub const fn max_push_batches(self) -> usize {
        self.max_push_batches
    }

    /// Returns the maximum incremental pull requests issued in one run.
    pub const fn max_pull_pages(self) -> usize {
        self.max_pull_pages
    }

    /// Returns the hard timeout around each transport request.
    pub const fn request_timeout_millis(self) -> u64 {
        self.request_timeout_millis
    }
}

impl Default for OfflineSyncRunPolicy {
    fn default() -> Self {
        Self {
            push_batch_size: 50,
            max_push_batches: 4,
            max_pull_pages: 20,
            request_timeout_millis: 15_000,
        }
    }
}

/// Authenticated server response to one push request.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct AuthoritativePush {
    result: SyncPushResult,
    server_epoch_ms: u64,
}

impl AuthoritativePush {
    /// Wraps a typed push result with server-authored time.
    pub const fn new(result: SyncPushResult, server_epoch_ms: u64) -> Self {
        Self {
            result,
            server_epoch_ms,
        }
    }

    /// Returns the typed push result.
    pub const fn result(&self) -> &SyncPushResult {
        &self.result
    }

    /// Returns untrusted-for-authorization server time used for ordering.
    pub const fn server_epoch_ms(&self) -> u64 {
        self.server_epoch_ms
    }
}

/// Authenticated server response to one incremental pull request.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct AuthoritativePull {
    page: SyncPullPage,
    server_epoch_ms: u64,
}

impl AuthoritativePull {
    /// Wraps a typed pull page with server-authored time.
    pub const fn new(page: SyncPullPage, server_epoch_ms: u64) -> Self {
        Self {
            page,
            server_epoch_ms,
        }
    }

    /// Returns the typed pull page.
    pub const fn page(&self) -> &SyncPullPage {
        &self.page
    }

    /// Returns untrusted-for-authorization server time used for ordering.
    pub const fn server_epoch_ms(&self) -> u64 {
        self.server_epoch_ms
    }
}

/// Application-owned authenticated transport used by the static coordinator.
///
/// `account_id` is a routing and local-erasure binding, never proof of identity.
/// Implementations must authenticate independently and the server must derive
/// account, tenant, ownership and authorization from that authenticated context.
pub trait OfflineSyncTransport {
    /// Transport-specific error without erased dynamic dispatch.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Sends one replay-safe FIFO mutation batch.
    fn push<'a>(
        &'a self,
        account_id: &'a OfflineAccountId,
        batch: SyncPushBatch,
    ) -> impl Future<Output = Result<AuthoritativePush, Self::Error>> + Send + 'a;

    /// Fetches one authorized incremental page after the supplied cursor.
    fn pull<'a>(
        &'a self,
        account_id: &'a OfflineAccountId,
        cursor: Option<SyncCursor>,
    ) -> impl Future<Output = Result<AuthoritativePull, Self::Error>> + Send + 'a;
}

/// Observable work completed by one bounded synchronization attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct OfflineSyncReport {
    /// Push requests completed and accepted locally.
    pub push_batches: usize,
    /// Mutations submitted, including retryable server rejections.
    pub submitted_mutations: usize,
    /// Pending mutations removed by authoritative decisions.
    pub decided_mutations: usize,
    /// Pull pages completed and accepted locally.
    pub pull_pages: usize,
    /// Server-authored records observed across accepted pages.
    pub pulled_records: usize,
    /// Mutations still waiting for a later push.
    pub pending_mutations: usize,
    /// Conflicts waiting for an explicit application decision.
    pub conflicts: usize,
    /// More pending mutations remain after the push request budget.
    pub push_limit_reached: bool,
    /// The final page said more data remains after the pull request budget.
    pub pull_limit_reached: bool,
}

/// Typed state, transport, timeout, or protocol failure from orchestration.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum OfflineSyncRunError<E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    /// Local state rejected an untrusted server transition.
    #[error("offline synchronization state transition failed")]
    State(#[source] OfflineSyncError),
    /// The application transport failed.
    #[error("offline synchronization transport failed")]
    Transport(#[source] E),
    /// One transport future exceeded the mandatory per-request timeout.
    #[error("offline synchronization request timed out")]
    RequestTimedOut,
    /// A page claimed continuation without advancing its opaque cursor.
    #[error("offline synchronization cursor did not advance")]
    CursorDidNotAdvance,
}

/// Executes bounded push-then-pull synchronization over an application adapter.
#[non_exhaustive]
pub struct OfflineSyncCoordinator;

impl OfflineSyncCoordinator {
    /// Runs one bounded synchronization attempt.
    ///
    /// Successfully applied batches/pages remain committed if a later network
    /// request fails. Durable replay keys let the server safely return the same
    /// decision after a process crash; callers should persist the encrypted
    /// state after each successful run and after any returned error.
    pub async fn synchronize<T>(
        transport: &T,
        state: &mut OfflineSyncState,
        state_policy: OfflineSyncPolicy,
        run_policy: OfflineSyncRunPolicy,
    ) -> Result<OfflineSyncReport, OfflineSyncRunError<T::Error>>
    where
        T: OfflineSyncTransport,
    {
        if run_policy.push_batch_size() > state_policy.max_push_batch() {
            return Err(OfflineSyncRunError::State(
                OfflineSyncError::InvalidBatchLimit,
            ));
        }
        let timeout = Duration::from_millis(run_policy.request_timeout_millis());
        let mut report = OfflineSyncReport {
            push_batches: 0,
            submitted_mutations: 0,
            decided_mutations: 0,
            pull_pages: 0,
            pulled_records: 0,
            pending_mutations: state.pending().len(),
            conflicts: state.conflicts().len(),
            push_limit_reached: false,
            pull_limit_reached: false,
        };

        for _ in 0..run_policy.max_push_batches() {
            if state.pending().is_empty() {
                break;
            }
            let before = state.pending().len();
            let batch = state
                .push_batch(state_policy, run_policy.push_batch_size())
                .map_err(OfflineSyncRunError::State)?;
            report.submitted_mutations = report
                .submitted_mutations
                .saturating_add(batch.mutations().len());
            let response = tokio::time::timeout(timeout, transport.push(state.account_id(), batch))
                .await
                .map_err(|_| OfflineSyncRunError::RequestTimedOut)?
                .map_err(OfflineSyncRunError::Transport)?;
            state
                .apply_push(state_policy, response.result, response.server_epoch_ms)
                .map_err(OfflineSyncRunError::State)?;
            report.push_batches = report.push_batches.saturating_add(1);
            report.decided_mutations = report
                .decided_mutations
                .saturating_add(before.saturating_sub(state.pending().len()));
            if state.pending().len() == before {
                break;
            }
        }
        report.push_limit_reached =
            !state.pending().is_empty() && report.push_batches == run_policy.max_push_batches();

        for page_index in 0..run_policy.max_pull_pages() {
            let prior_cursor = state.cursor().cloned();
            let response = tokio::time::timeout(
                timeout,
                transport.pull(state.account_id(), prior_cursor.clone()),
            )
            .await
            .map_err(|_| OfflineSyncRunError::RequestTimedOut)?
            .map_err(OfflineSyncRunError::Transport)?;
            let has_more = response.page.has_more();
            if has_more && prior_cursor.as_ref() == Some(response.page.cursor()) {
                return Err(OfflineSyncRunError::CursorDidNotAdvance);
            }
            report.pulled_records = report
                .pulled_records
                .saturating_add(response.page.changes().len());
            state
                .apply_pull(state_policy, response.page, response.server_epoch_ms)
                .map_err(OfflineSyncRunError::State)?;
            report.pull_pages = report.pull_pages.saturating_add(1);
            if !has_more {
                break;
            }
            if page_index + 1 == run_policy.max_pull_pages() {
                report.pull_limit_reached = true;
            }
        }

        report.pending_mutations = state.pending().len();
        report.conflicts = state.conflicts().len();
        Ok(report)
    }
}
