//! One open Server-Sent Events stream: its state and the loop that turns
//! channel messages into events.

use super::super::revalidation::{self, Revalidate, Revalidator};
use super::super::{MAX_EVENT_BYTES, RealtimeError, RealtimeMessage, validate_name};
use axum::response::sse::Event;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::{OwnedSemaphorePermit, broadcast, watch};
use tokio::time::Instant;

/// The state of one open stream; dropping it releases the connection slot.
pub(super) struct Connection<R> {
    pub(super) receiver: broadcast::Receiver<RealtimeMessage>,
    pub(super) revalidator: Option<Revalidator<R>>,
    pub(super) shutdown: watch::Receiver<bool>,
    /// Keeps the signal alive after every [`RealtimeSse`](super::RealtimeSse)
    /// clone is dropped, so that only
    /// [`RealtimeSse::shutdown`](super::RealtimeSse::shutdown) ends the stream.
    pub(super) _shutdown_sender: Arc<watch::Sender<bool>>,
    pub(super) deadline: Instant,
    pub(super) max_payload_bytes: usize,
    pub(super) _permit: OwnedSemaphorePermit,
}

pub(super) async fn next_event<R: Revalidate>(
    mut connection: Connection<R>,
) -> Option<(Result<Event, Infallible>, Connection<R>)> {
    loop {
        // Yields when the task's budget runs out, so this loop never holds
        // the runtime even if every branch below were ready at once.
        tokio::task::consume_budget().await;
        let received = tokio::select! {
            biased;
            _ = connection.shutdown.wait_for(|stopped| *stopped) => return None,
            () = tokio::time::sleep_until(connection.deadline) => return None,
            () = revalidation::due(&mut connection.revalidator) => None,
            received = connection.receiver.recv() => Some(received),
        };
        // A due check runs here, outside the `select!`, so that messages
        // arriving meanwhile cannot cancel it.
        let Some(received) = received else {
            if let Some(revalidator) = connection.revalidator.as_mut()
                && !revalidator.check().await
            {
                tracing::debug!(
                    target: "rullst::realtime",
                    "ended a server-sent event stream whose request is no longer valid"
                );
                return None;
            }
            continue;
        };
        match received {
            Ok(message) => {
                if let Some(event) = event_for(&message, connection.max_payload_bytes) {
                    return Some((Ok(event), connection));
                }
                tracing::debug!(
                    target: "rullst::realtime",
                    payload_bytes = message.payload.len(),
                    "skipped a realtime message that cannot be sent as a server-sent event"
                );
            }
            Err(broadcast::error::RecvError::Lagged(missed)) => {
                tracing::debug!(
                    target: "rullst::realtime",
                    missed,
                    "ended a server-sent event stream that fell behind its channel"
                );
                return None;
            }
            Err(broadcast::error::RecvError::Closed) => return None,
        }
    }
}

/// The event for one message, or `None` when SSE cannot carry it within the
/// limits.
fn event_for(message: &RealtimeMessage, max_payload_bytes: usize) -> Option<Event> {
    if message.payload.len() > max_payload_bytes
        || validate_name(&message.event, MAX_EVENT_BYTES, RealtimeError::InvalidEvent).is_err()
    {
        return None;
    }
    // The validated name has no line break, which `Event::event` rejects;
    // `data` splits the payload's line breaks into `data:` lines itself.
    Some(
        Event::default()
            .event(&message.event)
            .data(&message.payload),
    )
}
