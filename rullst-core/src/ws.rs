use axum::extract::ws::{Message as AxumMessage, WebSocket as AxumWebSocket};

/// Error type for Rullst WebSocket operations.
#[derive(Debug)]
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

/// A high-level, elegant wrapper for Axum WebSockets, built for premium DX and HTMX
pub struct WebSocket {
    inner: AxumWebSocket,
}

impl WebSocket {
    /// Wraps a raw Axum WebSocket connection in the high-level `WebSocket` interface.
    pub fn new(inner: AxumWebSocket) -> Self {
        WebSocket { inner }
    }

    /// Send a text message to the WebSocket client
    #[cfg_attr(mutants, mutants::skip)]
    pub async fn send_text(&mut self, text: impl Into<String>) -> Result<(), WsError> {
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
    /// frame or the end of the stream returns `None`.
    #[cfg_attr(mutants, mutants::skip)]
    pub async fn recv(&mut self) -> Option<Result<String, WsError>> {
        loop {
            return match self.inner.recv().await {
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
