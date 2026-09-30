// src/worker.rs — Background mail worker handler registration.

use crate::facade::{MAIL_JOB_SCHEMA_VERSION, QueuedMail};
use crate::{Mail, MailError, Message};
use rullst_core::queue::Worker;
use serde_json::Value;

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum MailJobPayload {
    Current(QueuedMail),
    Legacy(Message),
}

/// Registers the background mail worker.
/// When the system polls a "rullst_mail_send" job, it will parse the JSON payload
/// into a versioned envelope and dispatch it synchronously through the same safe pipeline.
pub fn register_mail_handler(worker: &mut Worker) {
    worker.register("rullst_mail_send", |payload: Value| async move {
        let payload: MailJobPayload = serde_json::from_value(payload)?;
        match payload {
            MailJobPayload::Current(job) => {
                if job.schema_version != MAIL_JOB_SCHEMA_VERSION {
                    return Err(format!(
                        "unsupported rullst-mail job schema version: {}",
                        job.schema_version
                    )
                    .into());
                }
                let message = prepare_claimed_message(job.message)?;
                if let Some(tenant_id) = job.tenant_id {
                    Mail::send_now_for_tenant(tenant_id, message)
                        .await
                        .map_err(Into::into)
                } else {
                    Mail::send_now(message).await.map_err(Into::into)
                }
            }
            MailJobPayload::Legacy(message) => Mail::send_now(prepare_claimed_message(message)?)
                .await
                .map_err(Into::into),
        }
    });
}

/// How far a claimed job's `send_at` may lie ahead of this worker's clock.
///
/// Redis promotes scheduled jobs by the server's `TIME`, so a worker whose
/// clock lags that server sees a correctly due job as slightly early. The
/// check still rejects a queue that claims a schedule materially early.
const CLAIM_CLOCK_SKEW: chrono::TimeDelta = chrono::TimeDelta::seconds(300);

fn prepare_claimed_message(mut message: Message) -> Result<Message, MailError> {
    if message
        .send_at
        .as_ref()
        .is_some_and(|send_at| *send_at > chrono::Utc::now() + CLAIM_CLOCK_SKEW)
    {
        return Err(MailError::SendError(
            "queue claimed scheduled mail before its due timestamp".to_string(),
        ));
    }
    message.send_at = None;
    Ok(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_envelope_preserves_tenant_context() {
        let payload = serde_json::to_value(QueuedMail {
            schema_version: MAIL_JOB_SCHEMA_VERSION,
            tenant_id: Some("tenant_acme".to_string()),
            message: Message::new().to("user@example.com").text("safe"),
        })
        .expect("serialize queue envelope");
        let decoded: MailJobPayload =
            serde_json::from_value(payload).expect("deserialize queue envelope");
        let MailJobPayload::Current(decoded) = decoded else {
            panic!("versioned envelope must not decode as legacy");
        };
        assert_eq!(decoded.tenant_id.as_deref(), Some("tenant_acme"));
        assert_eq!(decoded.schema_version, MAIL_JOB_SCHEMA_VERSION);
    }

    #[test]
    fn queued_attachments_decode_from_base64_and_legacy_arrays() {
        let message =
            Message::new()
                .to("user@example.com")
                .text("safe")
                .attach(crate::Attachment::new(
                    "a.bin",
                    vec![0, 1, 255],
                    "application/octet-stream",
                ));
        let payload = serde_json::to_value(QueuedMail {
            schema_version: MAIL_JOB_SCHEMA_VERSION,
            tenant_id: None,
            message,
        })
        .expect("serialize queue envelope");
        assert_eq!(payload["message"]["attachments"][0]["content"], "AAH/");
        let mut legacy = payload.clone();
        legacy["message"]["attachments"][0]["content"] = serde_json::json!([0, 1, 255]);
        for payload in [payload, legacy] {
            let decoded: MailJobPayload =
                serde_json::from_value(payload).expect("deserialize queue envelope");
            let MailJobPayload::Current(decoded) = decoded else {
                panic!("versioned envelope must not decode as legacy");
            };
            assert_eq!(decoded.message.attachments[0].content, vec![0, 1, 255]);
        }
    }

    #[test]
    fn claimed_schedule_is_enforced_then_consumed_by_the_queue() {
        let future = Message::new()
            .to("future@example.com")
            .send_in(std::time::Duration::from_secs(3_600));
        assert!(matches!(
            prepare_claimed_message(future),
            Err(MailError::SendError(_))
        ));

        // A worker clock slightly behind the queue server's still sends.
        let lagging = Message::new()
            .to("lagging@example.com")
            .send_in(std::time::Duration::from_millis(50));
        let claimed = prepare_claimed_message(lagging).expect("due within skew");
        assert!(claimed.send_at.is_none());

        let due = Message::new()
            .to("due@example.com")
            .send_at(chrono::Utc::now() - chrono::Duration::seconds(1));
        let claimed = prepare_claimed_message(due).expect("due message");
        assert!(claimed.send_at.is_none());
    }
}
