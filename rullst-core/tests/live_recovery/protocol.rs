use super::app::{self, Server};
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use std::time::Duration;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async,
    tungstenite::{
        Message,
        client::IntoClientRequest,
        http::{Request, header},
    },
};

type Socket = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

fn request(server: &Server, tenant: &str, token: &str) -> Request<()> {
    let mut request = format!("ws://{}/live/{tenant}", server.address)
        .into_client_request()
        .unwrap();
    request.headers_mut().insert(
        header::ORIGIN,
        format!("http://{}", server.address).parse().unwrap(),
    );
    request.headers_mut().insert(
        header::SEC_WEBSOCKET_PROTOCOL,
        "rullst.live.v1".parse().unwrap(),
    );
    request.headers_mut().insert(
        header::COOKIE,
        format!("live_fixture={token}; rullst_csrf={}", app::CSRF)
            .parse()
            .unwrap(),
    );
    request
        .headers_mut()
        .insert(header::USER_AGENT, "Mozilla/5.0".parse().unwrap());
    request
}

async fn connect(server: &Server, token: &str) -> Socket {
    let (socket, response) = connect_async(request(server, "school-a", token))
        .await
        .unwrap();
    assert_eq!(
        response
            .headers()
            .get(header::SEC_WEBSOCKET_PROTOCOL)
            .unwrap(),
        "rullst.live.v1"
    );
    socket
}

async fn frame(socket: &mut Socket) -> Message {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            match socket.next().await.unwrap().unwrap() {
                Message::Ping(bytes) => socket.send(Message::Pong(bytes)).await.unwrap(),
                Message::Pong(_) => (),
                message => return message,
            }
        }
    })
    .await
    .unwrap()
}

async fn snapshot(socket: &mut Socket, revision: &str, outcome: &str) -> Value {
    let Message::Text(text) = frame(socket).await else {
        panic!("expected snapshot");
    };
    let value: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(value["version"], 1);
    assert_eq!(value["revision"], revision);
    assert_eq!(value["outcome"], outcome);
    value
}

async fn action(socket: &mut Socket, revision: &str, action: &str) {
    socket.send(Message::Text(json!({"version":1,"kind":"action","id":"0123456789abcdef0123456789abcdef","revision":revision,"action":action,"fields":{}}).to_string().into())).await.unwrap();
}

#[tokio::test]
async fn independent_sockets_conflict_and_recover_an_ambiguous_committed_action() {
    let server = Server::start("sqlite::memory:", 0).await;
    let mut first = connect(&server, app::TEACHER).await;
    let mut second = connect(&server, app::FRESH_TEACHER).await;
    snapshot(&mut first, "0", "recovered").await;
    snapshot(&mut second, "0", "recovered").await;
    action(&mut first, "0", "increment").await;
    snapshot(&mut first, "1", "applied").await;
    action(&mut second, "0", "increment").await;
    snapshot(&mut second, "1", "conflict").await;
    action(&mut first, "1", "increment-drop").await;
    let Message::Close(Some(close)) = frame(&mut first).await else {
        panic!("expected unavailable close");
    };
    assert_eq!(u16::from(close.code), 1013);
    let mut recovered = connect(&server, app::TEACHER).await;
    snapshot(&mut recovered, "2", "recovered").await;
    action(&mut recovered, "1", "increment-drop").await;
    snapshot(&mut recovered, "2", "conflict").await;
    let count: i64 = sqlx::query_scalar("SELECT value FROM counters WHERE tenant=?")
        .bind("school-a")
        .fetch_one(&server.pool)
        .await
        .unwrap();
    assert_eq!(count, 2);
    first.close(None).await.ok();
    second.close(None).await.ok();
    recovered.close(None).await.ok();
    server.stop().await;
}

#[tokio::test]
async fn handshake_checks_origin_protocol_session_and_tenant() {
    let server = Server::start("sqlite::memory:", 0).await;
    let mut cases = vec![
        (request(&server, "school-a", "unknown"), 401),
        (request(&server, "school-b", app::TEACHER), 403),
    ];
    for origin in [Some("https://evil.example"), Some("null"), None] {
        let mut candidate = request(&server, "school-a", app::TEACHER);
        candidate.headers_mut().remove(header::ORIGIN);
        if let Some(origin) = origin {
            candidate
                .headers_mut()
                .insert(header::ORIGIN, origin.parse().unwrap());
        }
        cases.push((candidate, 403));
    }
    let mut no_protocol = request(&server, "school-a", app::TEACHER);
    no_protocol
        .headers_mut()
        .remove(header::SEC_WEBSOCKET_PROTOCOL);
    cases.push((no_protocol, 403));
    for (request, expected) in cases {
        let Err(tokio_tungstenite::tungstenite::Error::Http(response)) =
            connect_async(request).await
        else {
            panic!("handshake must deny");
        };
        assert_eq!(response.status().as_u16(), expected);
    }
    server.stop().await;
}

#[tokio::test]
async fn revocation_and_domain_permissions_close_without_disclosing_new_state() {
    let server = Server::start("sqlite::memory:", 0).await;
    let mut socket = connect(&server, app::TEACHER).await;
    snapshot(&mut socket, "0", "recovered").await;
    sqlx::query("UPDATE sessions SET active=0 WHERE token=?")
        .bind(app::TEACHER)
        .execute(&server.pool)
        .await
        .unwrap();
    let Message::Close(Some(close)) = frame(&mut socket).await else {
        panic!("revocation must close");
    };
    assert_eq!(u16::from(close.code), 4401);
    let mut learner = connect(&server, app::LEARNER).await;
    snapshot(&mut learner, "0", "recovered").await;
    action(&mut learner, "0", "increment").await;
    let Message::Close(Some(close)) = frame(&mut learner).await else {
        panic!("write permission must close");
    };
    assert_eq!(u16::from(close.code), 4401);
    let count: i64 = sqlx::query_scalar("SELECT value FROM counters WHERE tenant=?")
        .bind("school-a")
        .fetch_one(&server.pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    server.stop().await;
}

#[tokio::test]
async fn malformed_commands_and_slow_callbacks_never_execute_unbounded_work() {
    let server = Server::start("sqlite::memory:", 0).await;
    let mut malformed = connect(&server, app::TEACHER).await;
    snapshot(&mut malformed, "0", "recovered").await;
    malformed
        .send(Message::Text("{\"version\":99}".into()))
        .await
        .unwrap();
    let Message::Close(Some(close)) = frame(&mut malformed).await else {
        panic!("invalid command must close");
    };
    assert_eq!(u16::from(close.code), 4400);
    let mut slow = connect(&server, app::TEACHER).await;
    snapshot(&mut slow, "0", "recovered").await;
    let began = std::time::Instant::now();
    action(&mut slow, "0", "slow").await;
    let Message::Close(Some(close)) = frame(&mut slow).await else {
        panic!("slow callback must close");
    };
    assert_eq!(u16::from(close.code), 1013);
    assert!(began.elapsed() < Duration::from_millis(900));
    let count: i64 = sqlx::query_scalar("SELECT value FROM counters WHERE tenant=?")
        .bind("school-a")
        .fetch_one(&server.pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    server.stop().await;
}

#[tokio::test]
async fn shared_admission_and_idle_peer_deadline_release_capacity() {
    let server = Server::with_limits("sqlite::memory:", 0, 1, 32, Duration::from_secs(10)).await;
    let mut silent = connect(&server, app::TEACHER).await;
    snapshot(&mut silent, "0", "recovered").await;
    let Err(tokio_tungstenite::tungstenite::Error::Http(response)) =
        connect_async(request(&server, "school-a", app::TEACHER)).await
    else {
        panic!("shared capacity must reject a second connection");
    };
    assert_eq!(response.status().as_u16(), 429);
    // Do not poll the silent socket: tungstenite must not automatically answer
    // pings on its behalf. The bounded server heartbeat releases admission.
    tokio::time::sleep(Duration::from_millis(1200)).await;
    let mut recovered = connect(&server, app::TEACHER).await;
    snapshot(&mut recovered, "0", "recovered").await;
    recovered.close(None).await.ok();
    drop(silent);
    server.stop().await;
}

#[tokio::test]
async fn action_budget_and_lifetime_are_enforced_even_for_responsive_peers() {
    let server = Server::with_limits("sqlite::memory:", 0, 2, 1, Duration::from_secs(1)).await;
    let mut socket = connect(&server, app::TEACHER).await;
    snapshot(&mut socket, "0", "recovered").await;
    action(&mut socket, "0", "increment").await;
    snapshot(&mut socket, "1", "applied").await;
    action(&mut socket, "1", "increment").await;
    let Message::Close(Some(close)) = frame(&mut socket).await else {
        panic!("action budget must close");
    };
    assert_eq!(u16::from(close.code), 1013);
    let mut responsive = connect(&server, app::TEACHER).await;
    snapshot(&mut responsive, "1", "recovered").await;
    let began = std::time::Instant::now();
    let Message::Close(Some(close)) = frame(&mut responsive).await else {
        panic!("lifetime must close");
    };
    assert_eq!(u16::from(close.code), 1013);
    assert!(began.elapsed() >= Duration::from_millis(800));
    assert!(began.elapsed() < Duration::from_secs(3));
    server.stop().await;
}

#[tokio::test]
async fn binary_and_oversized_frames_never_reach_domain_mutations() {
    let server = Server::start("sqlite::memory:", 0).await;
    for rejected in [
        Message::Binary(vec![0; 16].into()),
        Message::Text("x".repeat(16385).into()),
    ] {
        let mut socket = connect(&server, app::TEACHER).await;
        snapshot(&mut socket, "0", "recovered").await;
        socket.send(rejected).await.unwrap();
        let Message::Close(Some(close)) = frame(&mut socket).await else {
            panic!("invalid frame must close");
        };
        assert_eq!(u16::from(close.code), 4400);
    }
    let count: i64 = sqlx::query_scalar("SELECT value FROM counters WHERE tenant=?")
        .bind("school-a")
        .fetch_one(&server.pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    server.stop().await;
}

#[tokio::test]
async fn revocation_during_a_committed_action_prevents_its_snapshot_disclosure() {
    let server = Server::start("sqlite::memory:", 0).await;
    let mut socket = connect(&server, app::TEACHER).await;
    snapshot(&mut socket, "0", "recovered").await;
    action(&mut socket, "0", "increment-revoke").await;
    let Message::Close(Some(close)) = frame(&mut socket).await else {
        panic!("new state must not be disclosed after revocation");
    };
    assert_eq!(u16::from(close.code), 4401);
    let count: i64 = sqlx::query_scalar("SELECT value FROM counters WHERE tenant=?")
        .bind("school-a")
        .fetch_one(&server.pool)
        .await
        .unwrap();
    assert_eq!(
        count, 1,
        "cancellation/disconnect does not prove domain rollback"
    );
    server.stop().await;
}
