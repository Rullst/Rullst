// src/worker.rs — Background mail worker handler registration.

use crate::{Mail, Message};
use rullst_core::queue::Worker;
use serde_json::Value;

/// Registers the background mail worker.
/// When the system polls a "rullst_mail_send" job, it will parse the JSON payload
/// into a `Message` and dispatch it synchronously via `Mail::send_now`.
pub fn register_mail_handler(worker: &mut Worker) {
    worker.register("rullst_mail_send", |payload: Value| async move {
        let msg: Message = serde_json::from_value(payload)?;
        Mail::send_now(msg).await.map_err(|e| e.into())
    });
}
