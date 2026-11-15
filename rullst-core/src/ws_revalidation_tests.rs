//! A revalidated WebSocket closes when its check fails, while it waits for
//! the client and before it sends; checks run once per interval.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use axum::extract::ws::WebSocketUpgrade;
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

const DEADLINE: Duration = Duration::from_secs(5);

#[derive(Clone, Default)]
struct Flag {
    revoked: Arc<AtomicBool>,
    checks: Arc<AtomicUsize>,
}

impl Flag {
    fn check(&self) -> impl Revalidate {
        let flag = self.clone();
        crate::realtime::revalidate_with(move || {
            let flag = flag.clone();
            async move {
                flag.checks.fetch_add(1, Ordering::SeqCst);
                !flag.revoked.load(Ordering::SeqCst)
            }
        })
    }
}

/// Serves `handler` for each upgraded connection and returns its URL.
async fn serve<F, Fut>(handler: F) -> String
where
    F: Fn(AxumWebSocket) -> Fut + Clone + Send + Sync + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    let app = axum::Router::new().route(
        "/ws",
        axum::routing::get(move |upgrade: WebSocketUpgrade| {
            let handler = handler.clone();
            async move { upgrade.on_upgrade(handler) }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await });
    format!("ws://{address}/ws")
}

async fn connect(
    url: &str,
) -> tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>> {
    tokio::time::timeout(DEADLINE, tokio_tungstenite::connect_async(url))
        .await
        .expect("handshake within the deadline")
        .expect("handshake")
        .0
}

#[tokio::test]
async fn a_waiting_connection_closes_when_its_check_fails() {
    let flag = Flag::default();
    let (done, mut finished) = mpsc::unbounded_channel();
    let server_flag = flag.clone();
    let url = serve(move |socket| {
        let (flag, done) = (server_flag.clone(), done.clone());
        async move {
            let mut ws =
                WebSocket::new(socket).with_revalidation(flag.check(), Duration::from_secs(1));
            while let Some(Ok(text)) = ws.recv().await {
                ws.send_text(text).await.unwrap();
            }
            done.send(()).unwrap();
        }
    })
    .await;
    let mut client = connect(&url).await;
    client.send(Message::Text("hello".into())).await.unwrap();
    let echoed = tokio::time::timeout(DEADLINE, client.next()).await.unwrap();
    assert!(matches!(&echoed, Some(Ok(Message::Text(text))) if text.as_str() == "hello"));
    flag.revoked.store(true, Ordering::SeqCst);
    // The idle connection is checked at its interval and closed.
    let closed = tokio::time::timeout(DEADLINE, client.next()).await.unwrap();
    assert!(matches!(closed, Some(Ok(Message::Close(_)))), "{closed:?}");
    tokio::time::timeout(DEADLINE, finished.recv())
        .await
        .unwrap();
}

#[tokio::test]
async fn a_sending_connection_checks_once_per_interval_and_then_closes() {
    let flag = Flag::default();
    let (result, mut results) = mpsc::unbounded_channel();
    let server_flag = flag.clone();
    let url = serve(move |socket| {
        let (flag, result) = (server_flag.clone(), result.clone());
        async move {
            let mut ws =
                WebSocket::new(socket).with_revalidation(flag.check(), Duration::from_secs(1));
            loop {
                tokio::time::sleep(Duration::from_millis(200)).await;
                if let Err(error) = ws.send_text("tick").await {
                    result.send(error.to_string()).unwrap();
                    break;
                }
            }
        }
    })
    .await;
    let mut client = connect(&url).await;
    // About 2.5 seconds of ticks while the check passes.
    let mut ticks = 0;
    while ticks < 12 {
        let message = tokio::time::timeout(DEADLINE, client.next()).await.unwrap();
        assert!(matches!(&message, Some(Ok(Message::Text(text))) if text.as_str() == "tick"));
        ticks += 1;
    }
    // Twelve sends, but a check only when an interval elapsed.
    let made = flag.checks.load(Ordering::SeqCst);
    assert!((1..=3).contains(&made), "{made} checks");
    flag.revoked.store(true, Ordering::SeqCst);
    // Within one interval and a few ticks, never later.
    let closed = tokio::time::timeout(DEADLINE, async {
        loop {
            match client.next().await {
                Some(Ok(Message::Text(_))) => continue,
                other => break other,
            }
        }
    })
    .await
    .expect("the connection closes within the deadline");
    assert!(matches!(closed, Some(Ok(Message::Close(_)))), "{closed:?}");
    let error = tokio::time::timeout(DEADLINE, results.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        error,
        "WebSocket send error: the connection is no longer authorized"
    );
}

#[tokio::test]
async fn a_connection_without_revalidation_never_checks() {
    let url = serve(|socket| async move {
        let mut ws = WebSocket::new(socket);
        while let Some(Ok(text)) = ws.recv().await {
            ws.send_html(text).await.unwrap();
        }
    })
    .await;
    let mut client = connect(&url).await;
    for round in ["one", "two"] {
        client.send(Message::Text(round.into())).await.unwrap();
        let echoed = tokio::time::timeout(DEADLINE, client.next()).await.unwrap();
        assert!(matches!(&echoed, Some(Ok(Message::Text(text))) if text.as_str() == round));
    }
}
