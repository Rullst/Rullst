//! Connection-reuse tests against a minimal in-process RESP server.

use super::*;
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;

/// Admitted fixed-window decision: the first request of a 60 s window.
const ADMITTED: &[u8] = b"*2\r\n:1\r\n:60000\r\n";

/// Scripted `EVAL` replies, shared by every connection, used before
/// [`ADMITTED`].
type Replies = Arc<Mutex<VecDeque<&'static [u8]>>>;

/// Answers every `EVAL` with the next scripted reply, then with an admitted
/// fixed-window decision, and every other command with `+OK`, counting the
/// TCP connections it accepts.
struct FakeRedis {
    address: SocketAddr,
    accepted: Arc<AtomicUsize>,
    disconnect: watch::Sender<u64>,
    server: tokio::task::JoinHandle<()>,
}

impl FakeRedis {
    async fn start() -> Self {
        Self::start_with(&[]).await
    }

    async fn start_with(replies: &[&'static [u8]]) -> Self {
        let replies: Replies = Arc::new(Mutex::new(replies.iter().copied().collect()));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let accepted = Arc::new(AtomicUsize::new(0));
        let (disconnect, epochs) = watch::channel(0_u64);
        let counter = Arc::clone(&accepted);
        let server = tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                counter.fetch_add(1, Ordering::SeqCst);
                tokio::spawn(serve(stream, epochs.clone(), Arc::clone(&replies)));
            }
        });
        Self {
            address,
            accepted,
            disconnect,
            server,
        }
    }

    fn limiter(&self) -> RedisRateLimiter {
        RedisRateLimiter::new(
            format!("redis://{}/", self.address),
            "rullst:test",
            1_000,
            Duration::from_secs(60),
        )
        .unwrap()
    }

    fn accepted(&self) -> usize {
        self.accepted.load(Ordering::SeqCst)
    }

    /// Closes every open connection from the server side.
    fn drop_connections(&self) {
        self.disconnect.send_modify(|epoch| *epoch += 1);
    }
}

impl Drop for FakeRedis {
    fn drop(&mut self) {
        self.server.abort();
    }
}

async fn serve(stream: TcpStream, mut epochs: watch::Receiver<u64>, replies: Replies) {
    epochs.borrow_and_update();
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    loop {
        let command = tokio::select! {
            command = read_command(&mut reader) => command,
            _ = epochs.changed() => return,
        };
        let Some(command) = command else {
            return;
        };
        let reply: &[u8] = if command.eq_ignore_ascii_case("EVAL") {
            let scripted = replies.lock().unwrap().pop_front();
            scripted.unwrap_or(ADMITTED)
        } else {
            b"+OK\r\n"
        };
        if writer.write_all(reply).await.is_err() {
            return;
        }
    }
}

/// Reads one RESP array command and returns its name.
async fn read_command(reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>) -> Option<String> {
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .await
        .ok()
        .filter(|read| *read > 0)?;
    let count: usize = line.trim_end().strip_prefix('*')?.parse().ok()?;
    let mut name = None;
    for _ in 0..count {
        line.clear();
        reader.read_line(&mut line).await.ok()?;
        let length: usize = line.trim_end().strip_prefix('$')?.parse().ok()?;
        let mut bytes = vec![0; length + 2];
        reader.read_exact(&mut bytes).await.ok()?;
        bytes.truncate(length);
        if name.is_none() {
            name = Some(String::from_utf8(bytes).ok()?);
        }
    }
    name
}

#[tokio::test]
async fn distributed_checks_and_clones_reuse_one_connection() {
    let redis = FakeRedis::start().await;
    let limiter = redis.limiter();
    let clone = limiter.clone();
    assert_eq!(redis.accepted(), 0, "construction must stay lazy");

    for index in 0..10 {
        let key = format!("client-{index}");
        assert!(limiter.check(&key).await.unwrap().allowed);
        assert!(clone.check(&key).await.unwrap().allowed);
    }

    assert_eq!(redis.accepted(), 1);
}

#[tokio::test]
async fn concurrent_checks_reuse_one_connection_after_warm_up() {
    let redis = FakeRedis::start().await;
    let limiter = redis.limiter();
    limiter.check("warm-up").await.unwrap();

    let checks: Vec<_> = (0..32)
        .map(|index| {
            let limiter = limiter.clone();
            tokio::spawn(async move { limiter.check(&format!("client-{index}")).await })
        })
        .collect();
    for check in checks {
        assert!(check.await.unwrap().unwrap().allowed);
    }

    assert_eq!(redis.accepted(), 1);
}

#[tokio::test]
async fn a_broken_connection_is_replaced() {
    let redis = FakeRedis::start().await;
    let limiter = redis.limiter();
    limiter.check("before").await.unwrap();
    assert_eq!(redis.accepted(), 1);

    redis.drop_connections();
    let mut recovered = false;
    for _ in 0..5 {
        if limiter.check("after").await.is_ok() {
            recovered = true;
            break;
        }
    }

    assert!(
        recovered,
        "the limiter must reconnect after a dropped connection"
    );
    assert_eq!(redis.accepted(), 2);
    limiter.check("again").await.unwrap();
    assert_eq!(redis.accepted(), 2);
}

#[tokio::test]
async fn window_replies_without_a_count_or_an_expiry_are_rejected() {
    let redis = FakeRedis::start_with(&[
        // A counter without an expiry.
        b"*2\r\n:1\r\n:-1\r\n",
        // A counter that was not incremented.
        b"*2\r\n:0\r\n:60000\r\n",
        // A window that expires right now is still valid.
        b"*2\r\n:1\r\n:0\r\n",
    ])
    .await;
    let limiter = redis.limiter();
    for _ in 0..2 {
        assert_eq!(
            limiter.check("client").await.err(),
            Some(RateLimitError::InvalidBackendResponse)
        );
    }
    assert_eq!(
        limiter.check("client").await.unwrap(),
        RateLimitDecision {
            allowed: true,
            remaining: 999,
            retry_after: Duration::ZERO,
        }
    );
    // A well-formed but invalid reply keeps the connection.
    assert_eq!(redis.accepted(), 1);
}

#[tokio::test]
async fn an_unparsable_reply_replaces_the_connection() {
    // `?` starts no RESP type, so the client fails with a parse error that
    // leaves the connection unusable without dropping it.
    let redis = FakeRedis::start_with(&[b"?\r\n"]).await;
    let limiter = redis.limiter();
    assert!(matches!(
        limiter.check("client").await,
        Err(RateLimitError::Backend(_))
    ));
    assert!(limiter.check("client").await.unwrap().allowed);
    assert_eq!(redis.accepted(), 2);
}
