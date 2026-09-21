use axum::{
    Router,
    body::{Body, Bytes},
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, Response},
    routing::post,
};
use opentelemetry_proto::tonic::collector::trace::v1::{
    ExportTracePartialSuccess, ExportTraceServiceResponse,
};
use prost::Message;
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;

#[derive(Clone, Debug)]
pub enum Reply {
    Success,
    Malformed,
    Oversized,
    Chunked,
    Partial,
    Unavailable,
    Redirect,
    Slow,
    Held,
}
pub struct Received {
    pub headers: HeaderMap,
    pub body: Bytes,
}
struct Inner {
    reply: Reply,
    received: Mutex<Vec<Received>>,
    released: tokio::sync::watch::Sender<bool>,
}
pub struct Collector {
    inner: Arc<Inner>,
    pub endpoint: String,
    task: tokio::task::JoinHandle<()>,
}
impl Collector {
    pub async fn start(reply: Reply) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}/v1/traces", listener.local_addr().unwrap());
        let inner = Arc::new(Inner {
            reply,
            received: Mutex::new(Vec::new()),
            released: tokio::sync::watch::channel(false).0,
        });
        let app = Router::new()
            .route("/v1/traces", post(collect))
            .layer(DefaultBodyLimit::max(1024 * 1024))
            .with_state(inner.clone());
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        Self {
            inner,
            endpoint,
            task,
        }
    }
    pub async fn take(&self) -> Vec<Received> {
        std::mem::take(&mut *self.inner.received.lock().await)
    }
    pub fn release(&self) {
        self.inner.released.send_replace(true);
    }
}
impl Drop for Collector {
    fn drop(&mut self) {
        self.task.abort();
    }
}
async fn collect(
    State(inner): State<Arc<Inner>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response<Body> {
    let mut received = inner.received.lock().await;
    assert!(received.len() < 32, "fixture request budget");
    received.push(Received { headers, body });
    drop(received);
    let mut response = Response::builder().header("content-type", "application/x-protobuf");
    let bytes = match inner.reply {
        Reply::Success => Vec::new(),
        Reply::Malformed => b"not-protobuf-private-diagnostic".to_vec(),
        Reply::Oversized => vec![0; 16385],
        Reply::Chunked => {
            return response
                .body(Body::from_stream(futures_util::stream::iter((0..5).map(
                    |_| Ok::<_, std::io::Error>(Bytes::from(vec![0; 4096])),
                ))))
                .unwrap();
        }
        Reply::Partial => ExportTraceServiceResponse {
            partial_success: Some(ExportTracePartialSuccess {
                rejected_spans: 1,
                error_message: "private-collector-diagnostic".into(),
            }),
        }
        .encode_to_vec(),
        Reply::Unavailable => {
            response = response.status(503);
            Vec::new()
        }
        Reply::Redirect => {
            response = response.status(307).header("location", "/v1/traces");
            Vec::new()
        }
        Reply::Slow => {
            tokio::time::sleep(Duration::from_secs(2)).await;
            Vec::new()
        }
        Reply::Held => {
            let mut ready = inner.released.subscribe();
            ready.wait_for(|released| *released).await.unwrap();
            Vec::new()
        }
    };
    response.body(Body::from(bytes)).unwrap()
}
