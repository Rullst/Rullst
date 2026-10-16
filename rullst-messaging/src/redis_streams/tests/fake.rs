//! In-process loopback server speaking just enough RESP2 for the adapter:
//! `AUTH` succeeds and each `EVAL` is answered by the next queued responder,
//! so a test can return replies that the real scripts never produce.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

type Responder = Box<dyn Fn(&[Vec<u8>]) -> Vec<u8> + Send>;

pub(super) enum Resp {
    Bulk(Vec<u8>),
    Int(i64),
    Array(Vec<Resp>),
}

pub(super) fn bulk(value: impl AsRef<[u8]>) -> Resp {
    Resp::Bulk(value.as_ref().to_vec())
}

fn encode(value: &Resp, output: &mut Vec<u8>) {
    match value {
        Resp::Bulk(bytes) => {
            output.extend_from_slice(format!("${}\r\n", bytes.len()).as_bytes());
            output.extend_from_slice(bytes);
            output.extend_from_slice(b"\r\n");
        }
        Resp::Int(value) => output.extend_from_slice(format!(":{value}\r\n").as_bytes()),
        Resp::Array(values) => {
            output.extend_from_slice(format!("*{}\r\n", values.len()).as_bytes());
            for value in values {
                encode(value, output);
            }
        }
    }
}

/// A script reply: its status tag followed by `values`.
pub(super) fn tagged(tag: &str, values: Vec<Resp>) -> Vec<u8> {
    let mut reply = vec![bulk(tag)];
    reply.extend(values);
    let mut output = Vec::new();
    encode(&Resp::Array(reply), &mut output);
    output
}

pub(super) fn ok(values: Vec<Resp>) -> Vec<u8> {
    tagged("ok", values)
}

pub(super) struct FakeRedis {
    pub(super) url: String,
    responders: Arc<Mutex<VecDeque<Responder>>>,
    task: tokio::task::JoinHandle<()>,
}

impl FakeRedis {
    pub(super) async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("redis://{}", listener.local_addr().unwrap());
        let responders: Arc<Mutex<VecDeque<Responder>>> = Arc::default();
        let queue = responders.clone();
        let task = tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let queue = queue.clone();
                tokio::spawn(async move {
                    let (read, mut write) = stream.into_split();
                    let mut reader = BufReader::new(read);
                    while let Some(command) = read_command(&mut reader).await {
                        let reply = if command[0].eq_ignore_ascii_case(b"EVAL") {
                            let responder = queue.lock().unwrap().pop_front();
                            responder.map_or_else(
                                || b"-ERR no queued reply\r\n".to_vec(),
                                |respond| respond(&command),
                            )
                        } else {
                            b"+OK\r\n".to_vec()
                        };
                        if write.write_all(&reply).await.is_err() {
                            break;
                        }
                    }
                });
            }
        });
        Self {
            url,
            responders,
            task,
        }
    }

    /// Answers the next `EVAL` with `respond(command)`.
    pub(super) fn reply(&self, respond: impl Fn(&[Vec<u8>]) -> Vec<u8> + Send + 'static) {
        self.responders.lock().unwrap().push_back(Box::new(respond));
    }
}

impl Drop for FakeRedis {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn read_command<R: tokio::io::AsyncRead + Unpin>(
    reader: &mut BufReader<R>,
) -> Option<Vec<Vec<u8>>> {
    let mut line = String::new();
    if reader.read_line(&mut line).await.ok()? == 0 {
        return None;
    }
    let count: usize = line.trim_end().strip_prefix('*')?.parse().ok()?;
    let mut command = Vec::with_capacity(count.min(64));
    for _ in 0..count {
        line.clear();
        reader.read_line(&mut line).await.ok()?;
        let length: usize = line.trim_end().strip_prefix('$')?.parse().ok()?;
        if length > 1 << 20 {
            return None;
        }
        let mut bytes = vec![0; length + 2];
        reader.read_exact(&mut bytes).await.ok()?;
        bytes.truncate(length);
        command.push(bytes);
    }
    (!command.is_empty()).then_some(command)
}
