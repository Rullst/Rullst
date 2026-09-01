#![allow(clippy::expect_used, clippy::unwrap_used)]

use async_trait::async_trait;
use rullst_core::queue::{Queue, QueueDriver, QueueError, QueuedJob};
use rullst_mail::{Mail, Message};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

type Captured = (String, Value, Option<SystemTime>);

#[derive(Clone)]
struct CapturingQueue {
    jobs: Arc<Mutex<Vec<Captured>>>,
}

#[async_trait]
impl QueueDriver for CapturingQueue {
    async fn push(&self, _id: &str, name: &str, payload: &str) -> Result<(), QueueError> {
        let payload = serde_json::from_str(payload)
            .map_err(|error| QueueError::Serialization(error.to_string()))?;
        self.jobs
            .lock()
            .expect("capture lock")
            .push((name.to_string(), payload, None));
        Ok(())
    }

    async fn push_at(
        &self,
        _id: &str,
        name: &str,
        payload: &str,
        available_at: SystemTime,
    ) -> Result<(), QueueError> {
        let payload = serde_json::from_str(payload)
            .map_err(|error| QueueError::Serialization(error.to_string()))?;
        self.jobs.lock().expect("capture lock").push((
            name.to_string(),
            payload,
            Some(available_at),
        ));
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

    async fn pending_count(&self) -> Result<u64, QueueError> {
        Ok(self.jobs.lock().expect("capture lock").len() as u64)
    }
}

fn message(subject: &str) -> Message {
    Message::new()
        .to("learner@example.com")
        .from("academy@example.com")
        .subject(subject)
        .text("durable delivery")
}

#[tokio::test]
async fn global_queue_dispatches_scoped_and_unscoped_mail_jobs() {
    let jobs = Arc::new(Mutex::new(Vec::new()));
    Mail::init_queue(Queue::custom(Box::new(CapturingQueue {
        jobs: Arc::clone(&jobs),
    })));

    Mail::send(message("unscoped"))
        .await
        .expect("queued unscoped mail");
    Mail::send_for_tenant("tenant_academy", message("scoped"))
        .await
        .expect("queued tenant mail");

    let captured = jobs.lock().expect("capture lock");
    assert_eq!(captured.len(), 2);
    assert!(captured.iter().all(|job| job.0 == "rullst_mail_send"));
    assert!(captured.iter().all(|job| job.2.is_none()));
    assert!(captured[0].1["tenant_id"].is_null());
    assert_eq!(captured[0].1["message"]["subject"], "unscoped");
    assert_eq!(captured[1].1["tenant_id"], "tenant_academy");
    assert_eq!(captured[1].1["message"]["subject"], "scoped");
}
