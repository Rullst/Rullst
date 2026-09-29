//! Connection-reuse tests against a minimal in-process RESP server.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;

/// Speaks just enough RESP2 for the cache and queue drivers and counts the
/// TCP connections it accepts.
struct FakeRedis {
    address: SocketAddr,
    accepted: Arc<AtomicUsize>,
    disconnect: watch::Sender<u64>,
    server: tokio::task::JoinHandle<()>,
}

impl FakeRedis {
    async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let accepted = Arc::new(AtomicUsize::new(0));
        let (disconnect, epochs) = watch::channel(0_u64);
        let counter = Arc::clone(&accepted);
        let server = tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                counter.fetch_add(1, Ordering::SeqCst);
                tokio::spawn(serve(stream, epochs.clone()));
            }
        });
        Self {
            address,
            accepted,
            disconnect,
            server,
        }
    }

    fn url(&self) -> String {
        format!("redis://{}/", self.address)
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

async fn serve(stream: TcpStream, mut epochs: watch::Receiver<u64>) {
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
        if writer.write_all(reply(&command)).await.is_err() {
            return;
        }
    }
}

async fn read_command(
    reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>,
) -> Option<Vec<String>> {
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .await
        .ok()
        .filter(|read| *read > 0)?;
    let count: usize = line.trim_end().strip_prefix('*')?.parse().ok()?;
    let mut arguments = Vec::with_capacity(count);
    for _ in 0..count {
        line.clear();
        reader.read_line(&mut line).await.ok()?;
        let length: usize = line.trim_end().strip_prefix('$')?.parse().ok()?;
        let mut bytes = vec![0; length + 2];
        reader.read_exact(&mut bytes).await.ok()?;
        bytes.truncate(length);
        arguments.push(String::from_utf8(bytes).ok()?);
    }
    Some(arguments)
}

fn reply(command: &[String]) -> &'static [u8] {
    let name = command.first().map(|name| name.to_ascii_uppercase());
    match name.as_deref() {
        Some("GET") => b"$-1\r\n",
        Some("EXISTS" | "UNLINK" | "RPUSH") => b":0\r\n",
        Some("EVAL") => {
            let script = command.get(1).map_or("", String::as_str);
            if script.contains("LPOP") {
                b"$-1\r\n"
            } else {
                b":0\r\n"
            }
        }
        _ => b"+OK\r\n",
    }
}

#[cfg(feature = "cache-redis")]
#[tokio::test]
async fn cache_operations_reuse_one_connection() {
    use crate::cache::CacheDriver;
    use crate::cache::redis_driver::RedisDriver;

    let redis = FakeRedis::start().await;
    let driver = RedisDriver::new(redis.url()).unwrap();
    assert_eq!(redis.accepted(), 0, "construction must stay lazy");

    for index in 0..10 {
        let key = format!("key-{index}");
        assert_eq!(driver.get(&key).await.unwrap(), None);
        driver.put(&key, "value", Some(60)).await.unwrap();
        assert!(!driver.has(&key).await.unwrap());
        driver.forget(&key).await.unwrap();
    }

    assert_eq!(redis.accepted(), 1);
}

#[cfg(feature = "cache-redis")]
#[tokio::test]
async fn concurrent_cache_operations_reuse_one_connection_after_warm_up() {
    use crate::cache::CacheDriver;
    use crate::cache::redis_driver::RedisDriver;

    let redis = FakeRedis::start().await;
    let driver = Arc::new(RedisDriver::new(redis.url()).unwrap());
    driver.get("warm-up").await.unwrap();

    let operations: Vec<_> = (0..32)
        .map(|index| {
            let driver = Arc::clone(&driver);
            tokio::spawn(async move { driver.get(&format!("key-{index}")).await })
        })
        .collect();
    for operation in operations {
        assert_eq!(operation.await.unwrap().unwrap(), None);
    }

    assert_eq!(redis.accepted(), 1);
}

#[cfg(feature = "cache-redis")]
#[tokio::test]
async fn a_broken_cache_connection_is_replaced() {
    use crate::cache::CacheDriver;
    use crate::cache::redis_driver::RedisDriver;

    let redis = FakeRedis::start().await;
    let driver = RedisDriver::new(redis.url()).unwrap();
    driver.get("before").await.unwrap();
    assert_eq!(redis.accepted(), 1);

    redis.drop_connections();
    let mut recovered = false;
    for _ in 0..5 {
        if driver.get("after").await.is_ok() {
            recovered = true;
            break;
        }
    }

    assert!(
        recovered,
        "the driver must reconnect after a dropped connection"
    );
    assert_eq!(redis.accepted(), 2);
    driver.get("again").await.unwrap();
    assert_eq!(redis.accepted(), 2);
}

#[cfg(feature = "queue-redis")]
#[tokio::test]
async fn queue_operations_reuse_one_connection() {
    use crate::queue::QueueDriver;
    use crate::queue::RedisDriver;

    let redis = FakeRedis::start().await;
    let driver = RedisDriver::new(redis.url()).unwrap();
    assert_eq!(redis.accepted(), 0, "construction must stay lazy");

    driver.push("job-1", "email", "{}").await.unwrap();
    for _ in 0..10 {
        assert!(driver.pop().await.unwrap().is_none());
    }
    assert_eq!(driver.pending_count().await.unwrap(), 0);

    assert_eq!(redis.accepted(), 1);
}
