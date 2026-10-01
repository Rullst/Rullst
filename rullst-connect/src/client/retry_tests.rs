//! The optional retry policy must never replay a single-use token request
//! after a failure the provider may already have processed.

use super::{HttpClient, HttpRequest, ReqwestClient};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

fn post_form(url: String) -> HttpRequest {
    HttpRequest {
        method: "POST".into(),
        url,
        headers: reqwest::header::HeaderMap::new(),
        form: Some("grant_type=refresh_token&refresh_token=single-use".to_string()),
        json: None,
        basic_auth: None,
        bearer_auth: None,
    }
}

struct Counting {
    calls: Arc<AtomicUsize>,
    statuses: Vec<u16>,
}

impl wiremock::Respond for Counting {
    fn respond(&self, _request: &wiremock::Request) -> wiremock::ResponseTemplate {
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        let status = self
            .statuses
            .get(call)
            .or(self.statuses.last())
            .copied()
            .unwrap_or(200);
        wiremock::ResponseTemplate::new(status).set_body_json(serde_json::json!({"ok": true}))
    }
}

async fn counting_server(statuses: Vec<u16>) -> (wiremock::MockServer, Arc<AtomicUsize>) {
    let server = wiremock::MockServer::start().await;
    let calls = Arc::new(AtomicUsize::new(0));
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path("/token"))
        .respond_with(Counting {
            calls: calls.clone(),
            statuses,
        })
        .mount(&server)
        .await;
    (server, calls)
}

#[tokio::test]
async fn token_posts_are_not_replayed_after_server_errors() {
    let (server, calls) = counting_server(vec![500]).await;
    let client = ReqwestClient::new_with_retry(3);

    let response = client
        .execute(post_form(format!("{}/token", server.uri())))
        .await
        .unwrap();

    assert_eq!(response.status, 500);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn token_posts_are_retried_after_an_explicit_rate_limit() {
    let (server, calls) = counting_server(vec![429, 200]).await;
    let client = ReqwestClient::new_with_retry(1);

    let response = client
        .execute(post_form(format!("{}/token", server.uri())))
        .await
        .unwrap();

    assert_eq!(response.status, 200);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn token_posts_are_not_replayed_after_a_dropped_connection() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let accepted = Arc::new(AtomicUsize::new(0));
    let counter = accepted.clone();
    let server = tokio::spawn(async move {
        loop {
            let Ok((socket, _)) = listener.accept().await else {
                return;
            };
            counter.fetch_add(1, Ordering::SeqCst);
            // Read the request, as a provider that processed it would, then
            // drop the connection before any response bytes are written.
            let mut buffer = [0_u8; 4096];
            if socket.readable().await.is_ok() {
                let _ = socket.try_read(&mut buffer);
            }
            drop(socket);
        }
    });

    let client = ReqwestClient::new_with_retry(3);
    let result = client
        .execute(post_form(format!("http://{address}/token")))
        .await;

    assert!(result.is_err());
    assert_eq!(accepted.load(Ordering::SeqCst), 1);
    server.abort();
}
