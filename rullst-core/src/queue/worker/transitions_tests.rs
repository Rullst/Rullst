#![allow(clippy::unwrap_used)]

use super::*;
use crate::error_buffer::error_buffer;
use async_trait::async_trait;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

#[derive(Default)]
struct Driver {
    requeues: AtomicU64,
    reject_requeue: AtomicBool,
}

#[async_trait]
impl QueueDriver for Driver {
    async fn push(&self, _id: &str, _name: &str, _payload: &str) -> Result<(), QueueError> {
        Ok(())
    }

    async fn pop(&self) -> Result<Option<QueuedJob>, QueueError> {
        Ok(None)
    }

    async fn mark_complete(&self, _job_id: &str) -> Result<(), QueueError> {
        Ok(())
    }

    async fn mark_failed(&self, _job_id: &str, _error: &str) -> Result<(), QueueError> {
        Ok(())
    }

    async fn requeue(&self, _job_id: &str, _reason: &str) -> Result<(), QueueError> {
        self.requeues.fetch_add(1, Ordering::SeqCst);
        if self.reject_requeue.load(Ordering::SeqCst) {
            Err(QueueError::Driver("requeue failed".to_string()))
        } else {
            Ok(())
        }
    }

    async fn pending_count(&self) -> Result<u64, QueueError> {
        Ok(0)
    }
}

fn claimed() -> Result<Option<QueuedJob>, QueueError> {
    Ok(Some(QueuedJob {
        id: "job-1".to_string(),
        name: "work".to_string(),
        payload: serde_json::json!({}),
        attempts: 2,
    }))
}

#[tokio::test]
async fn a_claim_released_after_shutdown_is_counted_as_retried() {
    let driver = Driver::default();
    let activity = QueueActivity::default();
    let (errors, mut buffer) = error_buffer(4, "test");

    release_claim_after_shutdown(claimed(), &driver, &errors, &activity).await;
    assert_eq!(driver.requeues.load(Ordering::SeqCst), 1);
    assert_eq!(activity.snapshot().retried, 1);
    assert!(buffer.try_next().is_none());

    release_claim_after_shutdown(Ok(None), &driver, &errors, &activity).await;
    assert_eq!(driver.requeues.load(Ordering::SeqCst), 1);
    assert_eq!(activity.snapshot().retried, 1);
}

#[tokio::test]
async fn a_rejected_release_is_reported_and_not_counted() {
    let driver = Driver::default();
    driver.reject_requeue.store(true, Ordering::SeqCst);
    let activity = Arc::new(QueueActivity::default());
    let (errors, mut buffer) = error_buffer(4, "test");

    release_claim_after_shutdown(claimed(), &driver, &errors, &activity).await;
    assert_eq!(activity.snapshot().retried, 0);
    assert!(matches!(
        buffer.try_next(),
        Some(QueueError::StateTransition {
            operation: "requeue_claim_after_shutdown",
            ..
        })
    ));
}
