use crate::realtime::revalidation::{self, NoRevalidation, Revalidate, Revalidator};
use axum::extract::ws::{Message as AxumMessage, WebSocket as AxumWebSocket};
use std::time::Duration;

/// Error type for Rullst WebSocket operations.
#[derive(Debug)]
#[non_exhaustive]
pub enum WsError {
    /// Returned when sending a message to the client fails (e.g. the connection was dropped).
    SendError(String),
    /// Returned when receiving a message from the client fails (e.g. the connection was reset).
    RecvError(String),
}

impl std::fmt::Display for WsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WsError::SendError(err) => write!(f, "WebSocket send error: {}", err),
            WsError::RecvError(err) => write!(f, "WebSocket receive error: {}", err),
        }
    }
}

impl std::error::Error for WsError {}

/// A high-level, elegant wrapper for Axum WebSockets, built for premium DX and HTMX.
///
/// With [`Self::with_revalidation`] the connection also checks, while it
/// stays open, that its request may still use it (for example that its
/// session was not signed out), and closes when the check fails.
pub struct WebSocket<R = NoRevalidation> {
    inner: AxumWebSocket,
    revalidator: Option<Revalidator<R>>,
}

impl WebSocket {
    /// Wraps a raw Axum WebSocket connection in the high-level `WebSocket` interface.
    pub fn new(inner: AxumWebSocket) -> Self {
        WebSocket {
            inner,
            revalidator: None,
        }
    }
}

impl<R: Revalidate> WebSocket<R> {
    /// Revalidates the connection with `check` every `interval` (clamped to
    /// 1 second–5 minutes;
    /// [`DEFAULT_REVALIDATION_INTERVAL`](crate::realtime::DEFAULT_REVALIDATION_INTERVAL)
    /// is 15 seconds)
    /// and whenever its [`Revalidate::changed`] signal fires: while
    /// [`Self::recv`] waits, and before [`Self::send_text`] when a check is
    /// due. When the check answers `false` or takes longer than
    /// [`crate::realtime::REVALIDATION_DEADLINE`], the connection sends a
    /// Close frame, `recv` returns `None` and `send_text` fails.
    ///
    /// Added in v13.0.0-alpha.6.
    pub fn with_revalidation<S: Revalidate>(self, check: S, interval: Duration) -> WebSocket<S> {
        WebSocket {
            inner: self.inner,
            revalidator: Some(Revalidator::new(check, interval)),
        }
    }

    /// Runs the due check, if one is due now, and closes the connection when
    /// it fails. Returns whether the connection may continue.
    async fn revalidate_if_due(&mut self) -> bool {
        let Some(revalidator) = self.revalidator.as_mut() else {
            return true;
        };
        if !revalidator.due_now() || revalidator.check().await {
            return true;
        }
        self.close().await;
        false
    }

    /// Sends a Close frame; the peer may already be gone.
    async fn close(&mut self) {
        let _ = self.inner.send(AxumMessage::Close(None)).await;
    }

    /// Send a text message to the WebSocket client
    #[cfg_attr(mutants, mutants::skip)]
    pub async fn send_text(&mut self, text: impl Into<String>) -> Result<(), WsError> {
        if !self.revalidate_if_due().await {
            return Err(WsError::SendError(
                "the connection is no longer authorized".to_owned(),
            ));
        }
        self.inner
            .send(AxumMessage::Text(text.into().into()))
            .await
            .map_err(|e| WsError::SendError(e.to_string()))
    }

    /// Send an HTML fragment to the client (perfect for out-of-band HTMX swapping)
    #[cfg_attr(mutants, mutants::skip)]
    pub async fn send_html(&mut self, html_content: String) -> Result<(), WsError> {
        self.send_text(html_content).await
    }

    /// Receive the next text/binary frame from the client.
    ///
    /// Ping and Pong control frames are skipped (the underlying socket already
    /// answers pings), so client keepalives never surface as errors. A Close
    /// frame or the end of the stream returns `None`, as does a failed
    /// revalidation (see [`Self::with_revalidation`]).
    #[cfg_attr(mutants, mutants::skip)]
    pub async fn recv(&mut self) -> Option<Result<String, WsError>> {
        loop {
            // Yields when the task's budget runs out, so this loop never
            // holds the runtime even if both branches were ready at once.
            tokio::task::consume_budget().await;
            let frame = tokio::select! {
                biased;
                () = revalidation::due(&mut self.revalidator) => None,
                frame = self.inner.recv() => Some(frame),
            };
            let Some(frame) = frame else {
                // A due check runs outside the `select!`, so that frames
                // arriving meanwhile cannot cancel it.
                if let Some(revalidator) = self.revalidator.as_mut()
                    && !revalidator.check().await
                {
                    self.close().await;
                    return None;
                }
                continue;
            };
            return match frame {
                Some(Ok(AxumMessage::Text(t))) => Some(Ok(t.to_string())),
                Some(Ok(AxumMessage::Binary(b))) => {
                    Some(Ok(String::from_utf8_lossy(&b).to_string()))
                }
                Some(Ok(AxumMessage::Ping(_) | AxumMessage::Pong(_))) => continue,
                Some(Ok(AxumMessage::Close(_))) | None => None,
                Some(Err(e)) => Some(Err(WsError::RecvError(e.to_string()))),
            };
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_ws_error_display() {
        let err1 = WsError::SendError("timeout".to_string());
        let err2 = WsError::RecvError("closed".to_string());

        assert_eq!(format!("{}", err1), "WebSocket send error: timeout");
        assert_eq!(format!("{}", err2), "WebSocket receive error: closed");
    }
}

#[cfg(test)]
#[path = "ws_revalidation_tests.rs"]
mod revalidation_tests;
