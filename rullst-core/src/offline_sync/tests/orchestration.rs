use super::*;
use std::collections::VecDeque;
use std::fmt;
use std::future::Future;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug)]
struct MockTransportError;

impl fmt::Display for MockTransportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("mock transport failure")
    }
}

impl std::error::Error for MockTransportError {}

struct ApplyingTransport {
    pulls: Mutex<VecDeque<AuthoritativePull>>,
}

impl OfflineSyncTransport for ApplyingTransport {
    type Error = MockTransportError;

    async fn push(
        &self,
        _account_id: &OfflineAccountId,
        batch: SyncPushBatch,
    ) -> Result<AuthoritativePush, Self::Error> {
        let outcomes = batch
            .mutations()
            .iter()
            .map(|mutation| {
                let revision = mutation.base_revision().unwrap_or(0).saturating_add(1);
                let value = match mutation.operation() {
                    OfflineMutationKind::Upsert(value) => OfflineRecordValue::Upsert(value.clone()),
                    OfflineMutationKind::Delete => OfflineRecordValue::Deleted,
                };
                let record = ServerRecord::new(mutation.entity().clone(), revision, value)
                    .expect("server record");
                MutationOutcome::Applied {
                    idempotency_key: mutation.idempotency_key().clone(),
                    record,
                }
            })
            .collect();
        Ok(AuthoritativePush::new(
            SyncPushResult::new(batch.cursor().cloned(), outcomes),
            100,
        ))
    }

    async fn pull(
        &self,
        _account_id: &OfflineAccountId,
        _cursor: Option<SyncCursor>,
    ) -> Result<AuthoritativePull, Self::Error> {
        Ok(self
            .pulls
            .lock()
            .expect("pull fixture lock")
            .pop_front()
            .expect("pull fixture"))
    }
}

#[tokio::test]
async fn coordinator_applies_bounded_pushes_then_incremental_pull() {
    let mut state = OfflineSyncState::new(account("user-1"));
    state
        .queue(policy(), upsert("a", "mutation-a", None))
        .expect("first mutation");
    state
        .queue(policy(), upsert("b", "mutation-b", None))
        .expect("second mutation");
    let transport = ApplyingTransport {
        pulls: Mutex::new(VecDeque::from([AuthoritativePull::new(
            SyncPullPage::new(
                SyncCursor::new("cursor-final").expect("cursor"),
                vec![record("server-only", 1, 90)],
                false,
                false,
            ),
            101,
        )])),
    };
    let run_policy = OfflineSyncRunPolicy::new(1, 3, 2, 1_000).expect("run policy");

    let report = OfflineSyncCoordinator::synchronize(&transport, &mut state, policy(), run_policy)
        .await
        .expect("bounded synchronization");

    assert_eq!(report.push_batches, 2);
    assert_eq!(report.submitted_mutations, 2);
    assert_eq!(report.decided_mutations, 2);
    assert_eq!(report.pull_pages, 1);
    assert_eq!(report.pulled_records, 1);
    assert_eq!(report.pending_mutations, 0);
    assert_eq!(state.records().len(), 3);
    assert_eq!(state.cursor().map(SyncCursor::as_str), Some("cursor-final"));
}

#[test]
fn run_policy_and_authoritative_response_accessors_are_explicit() {
    assert!(matches!(
        OfflineSyncRunPolicy::new(0, 1, 1, 1),
        Err(OfflineSyncError::InvalidRunPolicy)
    ));
    assert!(OfflineSyncRunPolicy::new(101, 1, 1, 1).is_err());
    assert!(OfflineSyncRunPolicy::new(1, 0, 1, 1).is_err());
    assert!(OfflineSyncRunPolicy::new(1, 21, 1, 1).is_err());
    assert!(OfflineSyncRunPolicy::new(1, 1, 0, 1).is_err());
    assert!(OfflineSyncRunPolicy::new(1, 1, 101, 1).is_err());
    assert!(OfflineSyncRunPolicy::new(1, 1, 1, 0).is_err());
    assert!(OfflineSyncRunPolicy::new(1, 1, 1, 120_001).is_err());

    let configured = OfflineSyncRunPolicy::new(7, 8, 9, 10).expect("run policy");
    assert_eq!(configured.push_batch_size(), 7);
    assert_eq!(configured.max_push_batches(), 8);
    assert_eq!(configured.max_pull_pages(), 9);
    assert_eq!(configured.request_timeout_millis(), 10);
    let defaults = OfflineSyncRunPolicy::default();
    assert_eq!(defaults.push_batch_size(), 50);
    assert_eq!(defaults.max_push_batches(), 4);
    assert_eq!(defaults.max_pull_pages(), 20);
    assert_eq!(defaults.request_timeout_millis(), 15_000);

    let push = AuthoritativePush::new(SyncPushResult::new(None, vec![]), 44);
    assert!(push.result().outcomes().is_empty());
    assert_eq!(push.server_epoch_ms(), 44);
    let pull = AuthoritativePull::new(
        SyncPullPage::new(
            SyncCursor::new("cursor-accessor").expect("cursor"),
            vec![],
            false,
            false,
        ),
        45,
    );
    assert_eq!(pull.page().cursor().as_str(), "cursor-accessor");
    assert_eq!(pull.server_epoch_ms(), 45);
}

#[tokio::test]
async fn coordinator_reports_both_request_budgets_without_dropping_work() {
    let mut state = OfflineSyncState::new(account("user-1"));
    state
        .queue(policy(), upsert("a", "mutation-a", None))
        .expect("first mutation");
    state
        .queue(policy(), upsert("b", "mutation-b", None))
        .expect("second mutation");
    let transport = ApplyingTransport {
        pulls: Mutex::new(VecDeque::from([AuthoritativePull::new(
            SyncPullPage::new(
                SyncCursor::new("cursor-more").expect("cursor"),
                vec![],
                true,
                false,
            ),
            101,
        )])),
    };
    let report = OfflineSyncCoordinator::synchronize(
        &transport,
        &mut state,
        policy(),
        OfflineSyncRunPolicy::new(1, 1, 1, 1_000).expect("run policy"),
    )
    .await
    .expect("bounded run");
    assert!(report.push_limit_reached);
    assert!(report.pull_limit_reached);
    assert_eq!(report.pending_mutations, 1);
    assert_eq!(report.pull_pages, 1);
}

struct RetryableTransport {
    push_calls: AtomicUsize,
}

impl OfflineSyncTransport for RetryableTransport {
    type Error = MockTransportError;

    async fn push(
        &self,
        _account_id: &OfflineAccountId,
        batch: SyncPushBatch,
    ) -> Result<AuthoritativePush, Self::Error> {
        self.push_calls.fetch_add(1, Ordering::Relaxed);
        let outcomes = batch
            .mutations()
            .iter()
            .map(|mutation| MutationOutcome::Rejected {
                idempotency_key: mutation.idempotency_key().clone(),
                code: FailureCode::new("network.retry").expect("failure code"),
                retryable: true,
            })
            .collect();
        Ok(AuthoritativePush::new(
            SyncPushResult::new(batch.cursor().cloned(), outcomes),
            100,
        ))
    }

    fn pull<'a>(
        &'a self,
        _account_id: &'a OfflineAccountId,
        _cursor: Option<SyncCursor>,
    ) -> impl Future<Output = Result<AuthoritativePull, Self::Error>> + Send + 'a {
        std::future::ready(Ok(AuthoritativePull::new(
            SyncPullPage::new(
                SyncCursor::new("cursor-empty").expect("cursor"),
                vec![],
                false,
                false,
            ),
            101,
        )))
    }
}

#[tokio::test]
async fn retryable_rejection_does_not_spin_until_the_push_budget() {
    let transport = RetryableTransport {
        push_calls: AtomicUsize::new(0),
    };
    let mut state = OfflineSyncState::new(account("user-1"));
    state
        .queue(policy(), upsert("a", "mutation-a", None))
        .expect("mutation");
    let report = OfflineSyncCoordinator::synchronize(
        &transport,
        &mut state,
        policy(),
        OfflineSyncRunPolicy::new(1, 3, 1, 1_000).expect("run policy"),
    )
    .await
    .expect("retryable synchronization");
    assert_eq!(transport.push_calls.load(Ordering::Relaxed), 1);
    assert_eq!(report.push_batches, 1);
    assert_eq!(report.decided_mutations, 0);
    assert_eq!(report.pending_mutations, 1);
    assert!(!report.push_limit_reached);
}

struct StalledCursorTransport;

impl OfflineSyncTransport for StalledCursorTransport {
    type Error = MockTransportError;

    fn push<'a>(
        &'a self,
        _account_id: &'a OfflineAccountId,
        _batch: SyncPushBatch,
    ) -> impl Future<Output = Result<AuthoritativePush, Self::Error>> + Send + 'a {
        std::future::ready(Ok(AuthoritativePush::new(
            SyncPushResult::new(None, vec![]),
            2,
        )))
    }

    fn pull<'a>(
        &'a self,
        _account_id: &'a OfflineAccountId,
        cursor: Option<SyncCursor>,
    ) -> impl Future<Output = Result<AuthoritativePull, Self::Error>> + Send + 'a {
        let cursor = cursor.expect("existing cursor");
        std::future::ready(Ok(AuthoritativePull::new(
            SyncPullPage::new(cursor, vec![], true, false),
            2,
        )))
    }
}

#[tokio::test]
async fn coordinator_rejects_continuation_without_cursor_progress() {
    let mut state = OfflineSyncState::new(account("user-1"));
    state
        .replace_server_snapshot(
            policy(),
            vec![],
            SyncCursor::new("cursor-1").expect("cursor"),
            1,
        )
        .expect("initial cursor");
    let before = state.clone();
    let result = OfflineSyncCoordinator::synchronize(
        &StalledCursorTransport,
        &mut state,
        policy(),
        OfflineSyncRunPolicy::new(1, 1, 2, 1_000).expect("run policy"),
    )
    .await;
    assert!(matches!(
        result,
        Err(OfflineSyncRunError::CursorDidNotAdvance)
    ));
    assert_eq!(state, before);
}

struct HangingTransport;

impl OfflineSyncTransport for HangingTransport {
    type Error = MockTransportError;

    fn push<'a>(
        &'a self,
        _account_id: &'a OfflineAccountId,
        _batch: SyncPushBatch,
    ) -> impl Future<Output = Result<AuthoritativePush, Self::Error>> + Send + 'a {
        std::future::pending()
    }

    fn pull<'a>(
        &'a self,
        _account_id: &'a OfflineAccountId,
        _cursor: Option<SyncCursor>,
    ) -> impl Future<Output = Result<AuthoritativePull, Self::Error>> + Send + 'a {
        std::future::pending()
    }
}

#[tokio::test]
async fn coordinator_times_out_without_mutating_pending_work() {
    let mut state = OfflineSyncState::new(account("user-1"));
    state
        .queue(policy(), upsert("a", "mutation-a", None))
        .expect("mutation");
    let before = state.clone();
    let result = OfflineSyncCoordinator::synchronize(
        &HangingTransport,
        &mut state,
        policy(),
        OfflineSyncRunPolicy::new(1, 1, 1, 1).expect("run policy"),
    )
    .await;
    assert!(matches!(result, Err(OfflineSyncRunError::RequestTimedOut)));
    assert_eq!(state, before);

    let mut pull_state = OfflineSyncState::new(account("user-2"));
    let pull_before = pull_state.clone();
    let pull_result = OfflineSyncCoordinator::synchronize(
        &HangingTransport,
        &mut pull_state,
        policy(),
        OfflineSyncRunPolicy::new(1, 1, 1, 1).expect("run policy"),
    )
    .await;
    assert!(matches!(
        pull_result,
        Err(OfflineSyncRunError::RequestTimedOut)
    ));
    assert_eq!(pull_state, pull_before);
}

struct FailingTransport;

impl OfflineSyncTransport for FailingTransport {
    type Error = MockTransportError;

    fn push<'a>(
        &'a self,
        _account_id: &'a OfflineAccountId,
        _batch: SyncPushBatch,
    ) -> impl Future<Output = Result<AuthoritativePush, Self::Error>> + Send + 'a {
        std::future::ready(Err(MockTransportError))
    }

    fn pull<'a>(
        &'a self,
        _account_id: &'a OfflineAccountId,
        _cursor: Option<SyncCursor>,
    ) -> impl Future<Output = Result<AuthoritativePull, Self::Error>> + Send + 'a {
        std::future::ready(Err(MockTransportError))
    }
}

#[tokio::test]
async fn coordinator_preserves_typed_transport_and_state_policy_failures() {
    let run_policy = OfflineSyncRunPolicy::new(1, 1, 1, 1_000).expect("run policy");
    let mut push_state = OfflineSyncState::new(account("user-1"));
    push_state
        .queue(policy(), upsert("a", "mutation-a", None))
        .expect("pending mutation");
    assert!(matches!(
        OfflineSyncCoordinator::synchronize(
            &FailingTransport,
            &mut push_state,
            policy(),
            run_policy,
        )
        .await,
        Err(OfflineSyncRunError::Transport(MockTransportError))
    ));
    assert_eq!(push_state.pending().len(), 1);

    let mut pull_state = OfflineSyncState::new(account("user-2"));
    assert!(matches!(
        OfflineSyncCoordinator::synchronize(
            &FailingTransport,
            &mut pull_state,
            policy(),
            run_policy,
        )
        .await,
        Err(OfflineSyncRunError::Transport(MockTransportError))
    ));

    let mut invalid_batch_state = OfflineSyncState::new(account("user-3"));
    let oversized_run = OfflineSyncRunPolicy::new(4, 1, 1, 1_000).expect("run policy");
    assert!(matches!(
        OfflineSyncCoordinator::synchronize(
            &FailingTransport,
            &mut invalid_batch_state,
            policy(),
            oversized_run,
        )
        .await,
        Err(OfflineSyncRunError::State(
            OfflineSyncError::InvalidBatchLimit
        ))
    ));
}
