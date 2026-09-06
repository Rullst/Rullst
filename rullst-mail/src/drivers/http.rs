//! Bounded REST transport shared by the official HTTP mail adapters.
use super::MailError;
use std::{sync::OnceLock, time::Duration};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

fn build_client(timeout: Duration) -> Result<reqwest::Client, MailError> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .connect_timeout(Duration::from_secs(5))
        .timeout(timeout)
        .build()
        .map_err(|_| MailError::ConfigError("cannot initialize provider HTTP client".into()))
}

pub(super) fn client() -> Result<&'static reqwest::Client, MailError> {
    static CLIENT: OnceLock<Result<reqwest::Client, MailError>> = OnceLock::new();
    CLIENT
        .get_or_init(|| build_client(REQUEST_TIMEOUT))
        .as_ref()
        .map_err(Clone::clone)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    #[tokio::test]
    async fn request_deadline_covers_headers_and_provider_error_body() {
        for send_headers in [false, true] {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let url = format!("http://{}", listener.local_addr().unwrap());
            let server = tokio::spawn(async move {
                let (mut socket, _) =
                    tokio::time::timeout(Duration::from_secs(2), listener.accept())
                        .await
                        .unwrap()
                        .unwrap();
                let mut request = [0; 1024];
                tokio::time::timeout(Duration::from_secs(2), socket.read(&mut request))
                    .await
                    .unwrap()
                    .unwrap();
                if send_headers {
                    socket
                        .write_all(b"HTTP/1.1 500 Server Error\r\nContent-Length: 4\r\n\r\n")
                        .await
                        .unwrap();
                }
                std::future::pending::<()>().await;
            });
            let outcome = tokio::time::timeout(Duration::from_secs(2), async {
                let response = build_client(Duration::from_millis(150))
                    .unwrap()
                    .get(url)
                    .send()
                    .await;
                if send_headers {
                    assert!(matches!(
                        crate::error::provider_http_error("fixture", response.unwrap()).await,
                        MailError::TransportError { .. }
                    ));
                } else {
                    assert!(response.unwrap_err().is_timeout());
                }
            })
            .await;
            server.abort();
            let _ = server.await;
            outcome.unwrap();
        }
    }
}
