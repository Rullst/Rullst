#![allow(clippy::expect_used)]

use async_trait::async_trait;
use rullst_core::queue::{Queue, QueueDriver, QueueError, QueuedJob};
use rullst_mail::{Mail, Message};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Default)]
struct RecordedDispatch {
    immediate_count: usize,
    scheduled: Option<(String, String, SystemTime)>,
}

struct RecordingDriver {
    state: Arc<Mutex<RecordedDispatch>>,
}

#[async_trait]
impl QueueDriver for RecordingDriver {
    async fn push(&self, _id: &str, _job_name: &str, _payload: &str) -> Result<(), QueueError> {
        self.state.lock().expect("recording state").immediate_count += 1;
        Ok(())
    }

    async fn push_at(
        &self,
        _id: &str,
        job_name: &str,
        payload: &str,
        available_at: SystemTime,
    ) -> Result<(), QueueError> {
        self.state.lock().expect("recording state").scheduled =
            Some((job_name.to_string(), payload.to_string(), available_at));
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
        Ok(0)
    }
}

#[tokio::test]
async fn mail_facade_preserves_schedule_and_tenant_in_durable_envelope() {
    let state = Arc::new(Mutex::new(RecordedDispatch::default()));
    let queue = Queue::custom(Box::new(RecordingDriver {
        state: Arc::clone(&state),
    }));
    let target = chrono::Utc::now() + chrono::Duration::minutes(5);
    let message = Message::new()
        .to("user@example.com")
        .subject("Scheduled")
        .text("safe")
        .send_at(target);

    Mail::enqueue_for_tenant(&queue, "tenant:acme", message)
        .await
        .expect("scheduled tenant mail");

    let state = state.lock().expect("recorded scheduled dispatch");
    assert_eq!(state.immediate_count, 0);
    let (job_name, payload, available_at) = state
        .scheduled
        .as_ref()
        .expect("push_at must receive scheduled mail");
    assert_eq!(job_name, "rullst_mail_send");
    let envelope: serde_json::Value = serde_json::from_str(payload).expect("mail envelope JSON");
    assert_eq!(envelope["schema_version"], 1);
    assert_eq!(envelope["tenant_id"], "tenant:acme");
    assert_eq!(envelope["message"]["to"], "user@example.com");

    let target_system_time = UNIX_EPOCH
        + Duration::new(
            u64::try_from(target.timestamp()).expect("post-epoch test timestamp"),
            target.timestamp_subsec_nanos(),
        );
    assert_eq!(*available_at, target_system_time);
}
