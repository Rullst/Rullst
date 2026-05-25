use axum::{
    extract::ws::{Message as AxumMessage, WebSocket as AxumWebSocket},
};

#[derive(Debug)]
pub enum WsError {
    SendError(String),
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
    pub fn new(inner: AxumWebSocket) -> Self {
        WebSocket { inner }
    }

    /// Send a text message to the WebSocket client
    pub async fn send_text(&mut self, text: impl Into<String>) -> Result<(), WsError> {
        self.inner
            .send(AxumMessage::Text(text.into()))
            .await
            .map_err(|e| WsError::SendError(e.to_string()))
    }

    /// Send an HTML fragment to the client (perfect for out-of-band HTMX swapping)
    pub async fn send_html(&mut self, html_content: String) -> Result<(), WsError> {
        self.send_text(html_content).await
    }

    /// Receive the next text/binary frame from the client
    pub async fn recv(&mut self) -> Option<Result<String, WsError>> {
        match self.inner.recv().await {
            Some(Ok(msg)) => match msg {
                AxumMessage::Text(t) => Some(Ok(t)),
                AxumMessage::Binary(b) => Some(Ok(String::from_utf8_lossy(&b).to_string())),
                AxumMessage::Close(_) => None,
                _ => Some(Err(WsError::RecvError(
                    "Unsupported message frame type".to_string(),
                ))),
            },
            Some(Err(e)) => Some(Err(WsError::RecvError(e.to_string()))),
            None => None,
        }
    }
}

#[cfg(test)]
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
