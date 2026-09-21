#![cfg(any(feature = "recovery-sqlite", feature = "recovery-postgres"))]
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "session_process/worker.rs"]
mod worker;

use rullst_auth::recovery::{SessionLabel, SqlRecoveryStore};
use std::{path::Path, process::Stdio, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::{Child, Command},
};

#[tokio::test]
#[ignore = "invoked only as an owned subprocess with configuration on stdin"]
async fn process_server_helper() {
    worker::serve().await;
}

struct Server {
    child: Child,
    address: String,
}

impl Server {
    async fn start(url: &str, subject: &str, directory: &Path) -> Self {
        let ready = directory.join(format!("ready-{}.txt", rand::random::<u64>()));
        let config = worker::Configuration {
            database: url.to_string(),
            subject: subject.to_string(),
            ready: ready.clone(),
        };
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--ignored",
                "--exact",
                "process_server_helper",
                "--nocapture",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(&serde_json::to_vec(&config).unwrap())
            .await
            .unwrap();
        let address = tokio::time::timeout(Duration::from_secs(15), async {
            loop {
                assert!(
                    child.try_wait().unwrap().is_none(),
                    "owned request verifier exited before readiness"
                );
                if let Ok(address) = tokio::fs::read_to_string(&ready).await
                    && address.parse::<std::net::SocketAddr>().is_ok()
                {
                    break address;
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        })
        .await
        .expect("owned HTTP verifier must become ready");
        Self { child, address }
    }

    async fn stop(mut self) {
        self.child.kill().await.unwrap();
        self.child.wait().await.unwrap();
    }

    async fn request(
        &self,
        method: &str,
        path: &str,
        token: &str,
        csrf: bool,
        body: &str,
    ) -> (u16, String) {
        tokio::time::timeout(Duration::from_secs(10), async {
            let mut socket = tokio::net::TcpStream::connect(&self.address).await.unwrap();
            let csrf_header = if csrf { "X-CSRF-Token: 12345678901234567890123456789012\r\n" } else { "" };
            let request = format!("{method} {path} HTTP/1.1\r\nHost: {}\r\nUser-Agent: Mozilla/5.0\r\nCookie: session={token}; rullst_csrf=12345678901234567890123456789012\r\n{csrf_header}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", self.address, body.len());
            socket.write_all(request.as_bytes()).await.unwrap();
            let mut response = Vec::new();
            socket.take(16384).read_to_end(&mut response).await.unwrap();
            let response = String::from_utf8(response).unwrap();
            let status = response.split_whitespace().nth(1).unwrap().parse().unwrap();
            (status, response)
        }).await.expect("owned verifier request deadline")
    }

    async fn status(&self, token: &str) -> u16 {
        self.request("GET", "/tenants/school-a/private", token, false, "")
            .await
            .0
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Receipt {
    subject: String,
    current: String,
    sibling: String,
    extra: String,
}

async fn process_contract(url: &str, directory: &Path) -> Receipt {
    let store = SqlRecoveryStore::connect(url, worker::keys())
        .await
        .unwrap();
    store.migrate().await.unwrap();
    let subject = format!("process-{}", rand::random::<u64>());
    let email = format!("{subject}@example.com");
    let password = format!("Fixture_{:032x}", rand::random::<u128>());
    store
        .register_account(&subject, &email, &password, worker::now())
        .await
        .unwrap();
    let account = store.authenticate(email, password).await.unwrap().unwrap();
    let current = store
        .create_session_with_label(
            &account,
            worker::now(),
            600,
            SessionLabel::new("Laptop").unwrap(),
        )
        .await
        .unwrap();
    let sibling = store
        .create_session_with_label(
            &account,
            worker::now(),
            600,
            SessionLabel::new("Phone").unwrap(),
        )
        .await
        .unwrap();
    let extra = store
        .create_session(&account, worker::now(), 600)
        .await
        .unwrap();
    let first = Server::start(url, &subject, directory).await;
    let second = Server::start(url, &subject, directory).await;
    for server in [&first, &second] {
        assert_eq!(server.status(current.expose()).await, 204);
        assert_eq!(server.status(sibling.expose()).await, 204);
        assert_eq!(
            server
                .request(
                    "GET",
                    "/tenants/school-b/private",
                    current.expose(),
                    false,
                    ""
                )
                .await
                .0,
            403
        );
    }
    let (status, response) = first
        .request(
            "GET",
            "/tenants/school-a/sessions",
            current.expose(),
            false,
            "",
        )
        .await;
    assert_eq!(status, 200);
    assert!(
        response
            .to_ascii_lowercase()
            .contains("cache-control: no-store")
    );
    assert!(
        response
            .to_ascii_lowercase()
            .contains("x-content-type-options: nosniff")
    );
    assert!(!response.contains(current.expose()));
    assert!(!response.contains(sibling.expose()));
    let target = store
        .active_sessions(current.expose(), worker::now())
        .await
        .unwrap()
        .into_iter()
        .find(|session| {
            session
                .label()
                .is_some_and(|label| label.as_str() == "Phone")
        })
        .unwrap();
    let body = serde_json::json!({"target":target.id().as_str()}).to_string();
    let route = "/tenants/school-a/sessions/revoke";
    assert_eq!(
        first
            .request("POST", route, current.expose(), false, &body)
            .await
            .0,
        403
    );
    assert_eq!(second.status(sibling.expose()).await, 204);
    assert_eq!(
        first
            .request(
                "POST",
                "/tenants/school-b/sessions/revoke",
                current.expose(),
                true,
                &body
            )
            .await
            .0,
        403
    );
    assert_eq!(second.status(sibling.expose()).await, 204);
    assert_eq!(
        first
            .request("POST", route, current.expose(), true, &body)
            .await
            .0,
        204
    );
    assert_eq!(second.status(sibling.expose()).await, 401);
    assert_eq!(second.status(current.expose()).await, 204);
    assert_eq!(
        first
            .request(
                "POST",
                "/tenants/school-a/sessions/revoke-others",
                current.expose(),
                true,
                ""
            )
            .await
            .0,
        204
    );
    assert_eq!(second.status(extra.expose()).await, 401);
    assert_eq!(second.status(current.expose()).await, 204);
    assert!(
        store
            .create_session(&account, worker::now(), 600)
            .await
            .is_err()
    );
    first.stop().await;
    second.stop().await;
    store.close().await;
    let restarted = Server::start(url, &subject, directory).await;
    assert_eq!(restarted.status(sibling.expose()).await, 401);
    assert_eq!(restarted.status(extra.expose()).await, 401);
    assert_eq!(restarted.status(current.expose()).await, 204);
    restarted.stop().await;
    Receipt {
        subject,
        current: current.expose().to_string(),
        sibling: sibling.expose().to_string(),
        extra: extra.expose().to_string(),
    }
}

#[cfg(feature = "recovery-sqlite")]
#[tokio::test]
async fn sqlite_requests_observe_revocation_across_processes_and_restart() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("session request 100%#.db");
    let encoded: String =
        url::form_urlencoded::byte_serialize(path.to_str().unwrap().as_bytes()).collect();
    let url = format!("sqlite:{}?mode=rwc", encoded.replace('+', "%20"));
    process_contract(&url, directory.path()).await;
}

#[cfg(feature = "recovery-postgres")]
#[tokio::test]
#[ignore = "requires the owned PostgreSQL fixture"]
async fn postgres_requests_observe_revocation_across_processes_and_restart() {
    let directory = tempfile::tempdir().unwrap();
    let url = std::env::var("RULLST_RECOVERY_TEST_POSTGRES_URL").unwrap();
    let checkpoint = std::env::var("RULLST_SESSION_RESTART_RECEIPT").unwrap();
    match std::env::var("RULLST_SESSION_RESTART_PHASE")
        .unwrap()
        .as_str()
    {
        "exercise" => {
            let receipt = process_contract(&url, directory.path()).await;
            // Only synthetic fixture tokens, in the script's owned private file.
            tokio::fs::write(checkpoint, serde_json::to_vec(&receipt).unwrap())
                .await
                .unwrap();
        }
        "restart" => {
            assert!(tokio::fs::metadata(&checkpoint).await.unwrap().len() <= 4096);
            let receipt: Receipt =
                serde_json::from_slice(&tokio::fs::read(checkpoint).await.unwrap()).unwrap();
            let server = Server::start(&url, &receipt.subject, directory.path()).await;
            assert_eq!(server.status(&receipt.current).await, 204);
            assert_eq!(server.status(&receipt.sibling).await, 401);
            assert_eq!(server.status(&receipt.extra).await, 401);
            let store = SqlRecoveryStore::connect(&url, worker::keys())
                .await
                .unwrap();
            assert_eq!(
                store
                    .active_sessions(&receipt.current, worker::now())
                    .await
                    .unwrap()
                    .len(),
                1
            );
            store.revoke_session(&receipt.current).await.unwrap();
            assert_eq!(server.status(&receipt.current).await, 401);
            store.close().await;
            server.stop().await;
        }
        _ => panic!("unknown owned-fixture phase"),
    }
}
