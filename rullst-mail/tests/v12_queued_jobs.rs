//! Mail jobs that a 12.x producer queued are delivered by the driver that the
//! upgraded worker configures, and a driver that v13 removed fails the job
//! visibly instead of dropping its mail.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use async_trait::async_trait;
use rullst_core::queue::{Queue, QueueDriver, QueueError, QueuedJob, Worker};
use rullst_mail::{OfflineMailMock, register_mail_handler};
use serde_json::{Value, json};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Serves pre-seeded jobs and records how the worker settled each one.
#[derive(Clone, Default)]
struct SeededQueue {
    pending: Arc<Mutex<VecDeque<QueuedJob>>>,
    completed: Arc<Mutex<Vec<String>>>,
    failed: Arc<Mutex<Vec<(String, String)>>>,
}

impl SeededQueue {
    fn seed(&self, id: &str, payload: Value) {
        self.pending.lock().unwrap().push_back(QueuedJob {
            id: id.to_string(),
            name: "rullst_mail_send".to_string(),
            payload,
            attempts: 0,
        });
    }

    fn settled(&self) -> usize {
        self.completed.lock().unwrap().len() + self.failed.lock().unwrap().len()
    }
}

#[async_trait]
impl QueueDriver for SeededQueue {
    async fn push(&self, _id: &str, _name: &str, _payload: &str) -> Result<(), QueueError> {
        Err(QueueError::Unsupported("push".to_string()))
    }

    async fn pop(&self) -> Result<Option<QueuedJob>, QueueError> {
        Ok(self.pending.lock().unwrap().pop_front())
    }

    async fn mark_complete(&self, job_id: &str) -> Result<(), QueueError> {
        self.completed.lock().unwrap().push(job_id.to_string());
        Ok(())
    }

    async fn mark_failed(&self, job_id: &str, error: &str) -> Result<(), QueueError> {
        self.failed
            .lock()
            .unwrap()
            .push((job_id.to_string(), error.to_string()));
        Ok(())
    }

    async fn pending_count(&self) -> Result<u64, QueueError> {
        Ok(self.pending.lock().unwrap().len() as u64)
    }
}

/// A message exactly as a 12.1 or 12.2 producer serialized it: attachment
/// bytes as a JSON array.
fn v12_message(subject: &str, from: Option<&str>) -> Value {
    json!({
        "to": "member@example.com",
        "subject": subject,
        "body_html": null,
        "body_text": "queued before the upgrade",
        "from": from,
        "unsubscribe_url": null,
        "unsubscribe_email": null,
        "send_at": null,
        "attachments": [{
            "filename": "receipt.txt",
            "content": [114, 101, 99, 101, 105, 112, 116],
            "mime_type": "text/plain",
            "cid": null
        }]
    })
}

/// Runs a worker until it has settled `expected` jobs and returns the
/// processing error it reported, if any.
async fn drain(queue: &SeededQueue, expected: usize) -> Result<(), QueueError> {
    let mut worker = Worker::new(&Queue::custom(Box::new(queue.clone()))).poll_interval(10);
    register_mail_handler(&mut worker);
    let handle = worker.run().expect("worker starts");
    tokio::time::timeout(Duration::from_secs(20), async {
        while queue.settled() < expected {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("the worker settles every job");
    handle.shutdown().await
}

fn delivered(subject: &str) -> Vec<String> {
    OfflineMailMock::deliveries()
        .unwrap()
        .into_iter()
        .filter(|delivery| delivery.message.subject == subject)
        .map(|delivery| delivery.provider)
        .collect()
}

/// One test owns the process environment of this binary.
#[tokio::test]
async fn v12_jobs_use_the_configured_driver_and_removed_drivers_fail_visibly() {
    // SAFETY: this binary has a single test, so no other thread reads the
    // environment while it changes.
    unsafe {
        std::env::set_var("MAIL_DRIVER", "resend");
        std::env::set_var("RESEND_API_KEY", "mock_resend");
        std::env::set_var("MAIL_FROM", "Acme <billing@acme.example>");
    }
    let queue = SeededQueue::default();
    queue.seed(
        "v12-tenant",
        json!({
            "schema_version": 1,
            "tenant_id": "tenant_acme",
            "message": v12_message("v12 tenant envelope", Some("billing@acme.example")),
        }),
    );
    queue.seed(
        "v12-unscoped",
        json!({
            "schema_version": 1,
            "tenant_id": null,
            "message": v12_message("v12 unscoped envelope", None),
        }),
    );
    queue.seed("v12-legacy", v12_message("v12 legacy message", None));
    drain(&queue, 3).await.expect("12.x jobs are delivered");

    assert!(queue.failed.lock().unwrap().is_empty());
    assert_eq!(queue.completed.lock().unwrap().len(), 3);
    for subject in [
        "v12 tenant envelope",
        "v12 unscoped envelope",
        "v12 legacy message",
    ] {
        assert_eq!(delivered(subject), ["resend"], "{subject}");
    }

    // A worker still configured for a removed provider fails the job with
    // the reason and reports it, so the job stays in the queue's failed jobs
    // for a retry once the driver is changed; nothing is delivered elsewhere.
    unsafe { std::env::set_var("MAIL_DRIVER", "sendgrid") };
    let queue = SeededQueue::default();
    queue.seed(
        "v12-sendgrid",
        json!({
            "schema_version": 1,
            "tenant_id": null,
            "message": v12_message("v12 job for a removed driver", None),
        }),
    );
    let reported = drain(&queue, 1).await.expect_err("the failure is reported");
    assert!(
        reported
            .to_string()
            .contains("`sendgrid` (SendGrid) was removed"),
        "{reported}"
    );

    assert!(queue.completed.lock().unwrap().is_empty());
    let failed = queue.failed.lock().unwrap();
    assert_eq!(failed.len(), 1);
    assert_eq!(failed[0].0, "v12-sendgrid");
    assert!(
        failed[0].1.contains("`sendgrid` (SendGrid) was removed")
            && failed[0].1.contains("Mail providers removed"),
        "{}",
        failed[0].1
    );
    assert!(delivered("v12 job for a removed driver").is_empty());
}
