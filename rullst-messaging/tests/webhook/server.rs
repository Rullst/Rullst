use super::support::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::atomic::{AtomicBool, AtomicU16},
};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::TcpListener,
    sync::{Mutex, Notify},
};
use tokio_rustls::{TlsAcceptor, rustls};

#[derive(Clone, Debug)]
pub struct Request {
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
}
pub struct Receiver {
    pub url: String,
    pub root: Option<Vec<u8>>,
    pub requests: Arc<Mutex<Vec<Request>>>,
    pub effects: Arc<Mutex<BTreeSet<String>>>,
    pub status: Arc<AtomicU16>,
    pub drop_once: Arc<AtomicBool>,
    pub pause: Arc<AtomicBool>,
    pub received: Arc<Notify>,
    pub release: Arc<Notify>,
    task: tokio::task::JoinHandle<()>,
}
#[derive(Clone)]
struct State {
    requests: Arc<Mutex<Vec<Request>>>,
    effects: Arc<Mutex<BTreeSet<String>>>,
    status: Arc<AtomicU16>,
    drop_once: Arc<AtomicBool>,
    pause: Arc<AtomicBool>,
    received: Arc<Notify>,
    release: Arc<Notify>,
    clock: ManualClock,
}
impl Receiver {
    pub async fn start(clock: ManualClock, tls: bool) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let (acceptor, root) = if tls {
            let cert = rcgen::generate_simple_self_signed(vec!["127.0.0.1".into()]).unwrap();
            let root = cert.cert.pem().into_bytes();
            let private =
                rustls::pki_types::PrivatePkcs8KeyDer::from(cert.signing_key.serialize_der());
            let config = rustls::ServerConfig::builder_with_provider(Arc::new(
                rustls::crypto::aws_lc_rs::default_provider(),
            ))
            .with_safe_default_protocol_versions()
            .unwrap()
            .with_no_client_auth()
            .with_single_cert(vec![cert.cert.der().clone()], private.into())
            .unwrap();
            (Some(TlsAcceptor::from(Arc::new(config))), Some(root))
        } else {
            (None, None)
        };
        let state = State {
            requests: Arc::default(),
            effects: Arc::default(),
            status: Arc::new(AtomicU16::new(200)),
            drop_once: Arc::new(AtomicBool::new(false)),
            pause: Arc::new(AtomicBool::new(false)),
            received: Arc::default(),
            release: Arc::default(),
            clock,
        };
        let owner = state.clone();
        let task = tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    break;
                };
                let state = owner.clone();
                let acceptor = acceptor.clone();
                tokio::spawn(async move {
                    if let Some(acceptor) = acceptor {
                        if let Ok(Ok(stream)) =
                            tokio::time::timeout(Duration::from_secs(5), acceptor.accept(stream))
                                .await
                        {
                            handle(stream, state).await;
                        }
                    } else {
                        handle(stream, state).await;
                    }
                });
            }
        });
        Self {
            url: format!(
                "{}://127.0.0.1:{port}/events",
                if tls { "https" } else { "http" }
            ),
            root,
            requests: state.requests,
            effects: state.effects,
            status: state.status,
            drop_once: state.drop_once,
            pause: state.pause,
            received: state.received,
            release: state.release,
            task,
        }
    }
    pub fn destination(&self) -> WebhookDestination {
        let destination = WebhookDestination::loopback_test(&self.url).unwrap();
        if let Some(root) = &self.root {
            destination.with_test_root_pem(root.clone()).unwrap()
        } else {
            destination
        }
    }
}
impl Drop for Receiver {
    fn drop(&mut self) {
        self.task.abort();
        self.release.notify_waiters();
    }
}
async fn handle<S: AsyncRead + AsyncWrite + Unpin>(mut stream: S, state: State) {
    let request = tokio::time::timeout(Duration::from_secs(5), read(&mut stream)).await;
    let Ok(Some(request)) = request else {
        return;
    };
    let get = |name: &str| request.headers.get(name).unwrap().as_str();
    let signature = WebhookSignature::from_headers(
        get("rullst-webhook-id"),
        get("rullst-webhook-type"),
        get("rullst-webhook-timestamp"),
        get("rullst-webhook-key-id"),
        get("rullst-webhook-signature"),
    )
    .unwrap();
    let verified = key()
        .verify(
            &signature,
            &request.body,
            &state.clock,
            Duration::from_secs(300),
        )
        .unwrap();
    let status = state.status.load(Ordering::SeqCst);
    if status == 200 {
        state
            .effects
            .lock()
            .await
            .insert(verified.delivery_id().to_owned());
    }
    state.requests.lock().await.push(request);
    state.received.notify_one();
    if state.pause.load(Ordering::SeqCst) {
        let _ = tokio::time::timeout(Duration::from_secs(10), state.release.notified()).await;
    }
    if state.drop_once.swap(false, Ordering::SeqCst) {
        return;
    }
    let headers = if status == 302 {
        "Location: http://169.254.169.254/metadata\r\n"
    } else if status == 429 {
        "Retry-After: 3\r\n"
    } else {
        ""
    };
    let response = format!(
        "HTTP/1.1 {status} Fixture\r\n{headers}Content-Length: 7\r\nConnection: close\r\n\r\nprivate"
    );
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.shutdown().await;
}
async fn read<S: AsyncRead + Unpin>(stream: &mut S) -> Option<Request> {
    let mut bytes = Vec::new();
    let mut buffer = [0; 2048];
    let end = loop {
        let count = stream.read(&mut buffer).await.ok()?;
        if count == 0 {
            return None;
        }
        bytes.extend_from_slice(&buffer[..count]);
        if let Some(index) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
            break index + 4;
        }
        if bytes.len() > 16384 {
            return None;
        }
    };
    let mut headers = BTreeMap::new();
    for line in std::str::from_utf8(&bytes[..end])
        .ok()?
        .lines()
        .skip(1)
        .filter(|line| !line.is_empty())
    {
        let (name, value) = line.split_once(':')?;
        if headers
            .insert(name.to_lowercase(), value.trim().to_owned())
            .is_some()
        {
            return None;
        }
    }
    let length = headers.get("content-length")?.parse::<usize>().ok()?;
    if length > 65536 {
        return None;
    }
    while bytes.len() < end + length {
        let count = stream.read(&mut buffer).await.ok()?;
        if count == 0 {
            return None;
        }
        bytes.extend_from_slice(&buffer[..count]);
    }
    Some(Request {
        headers,
        body: bytes[end..end + length].to_vec(),
    })
}
