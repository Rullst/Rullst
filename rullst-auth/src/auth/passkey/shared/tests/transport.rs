use super::*;
use std::{sync::atomic::AtomicBool, time::Duration};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

async fn forward(
    mut read: impl AsyncRead + Unpin,
    mut write: impl AsyncWrite + Unpin,
    blocked: Arc<AtomicBool>,
    armed: Option<Arc<AtomicBool>>,
) -> std::io::Result<()> {
    let mut buffer = [0u8; 4096];
    let marker = b"BEGIN ISOLATION LEVEL";
    let mut window = Vec::new();
    loop {
        let count = read.read(&mut buffer).await?;
        if count == 0 {
            return Ok(());
        }
        // Let pool acquisition finish, then interrupt the transaction request.
        // Retain only a short suffix so split TCP reads cannot hide the marker.
        if armed
            .as_ref()
            .is_some_and(|flag| flag.load(Ordering::SeqCst))
        {
            window.extend_from_slice(&buffer[..count]);
            if window.windows(marker.len()).any(|bytes| bytes == marker) {
                blocked.store(true, Ordering::SeqCst);
            }
            window.drain(..window.len().saturating_sub(marker.len()));
        }
        if !blocked.load(Ordering::SeqCst) {
            write.write_all(&buffer[..count]).await?;
        }
    }
}

pub(super) async fn stalled_transport_has_a_whole_operation_deadline(url: &str) {
    let (raw, a, b, clock) = reset(url).await;
    a.issue(&intent(66)).await.unwrap();
    let target = url::Url::parse(url).unwrap();
    let port = target.port().unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_url = format!(
        "postgres://postgres@127.0.0.1:{}/rullst_passkey_contract?sslmode=disable",
        listener.local_addr().unwrap().port()
    );
    let blocked = Arc::new(AtomicBool::new(false));
    let armed = Arc::new(AtomicBool::new(false));
    let arm = armed.clone();
    let switch = blocked.clone();
    let proxy = tokio::spawn(async move {
        let mut connections = tokio::task::JoinSet::new();
        loop {
            tokio::select! {
                accepted=listener.accept()=>{
                    let (client,_)=accepted.unwrap(); let blocked=switch.clone(); let armed=arm.clone();
                    connections.spawn(async move {
                        let upstream=tokio::net::TcpStream::connect(("127.0.0.1",port)).await.unwrap();
                        let (client_read,client_write)=client.into_split();
                        let (server_read,server_write)=upstream.into_split();
                        let _=tokio::try_join!(forward(client_read,server_write,blocked.clone(),Some(armed)),forward(server_read,client_write,blocked,None));
                    });
                },
                _=connections.join_next(), if !connections.is_empty()=>{},
            }
        }
    });
    let store = Store::connect_with_clock(proxy_url, config(), clock)
        .await
        .unwrap();
    armed.store(true, Ordering::SeqCst);
    let result = tokio::time::timeout(
        Duration::from_secs(20),
        store.consume([66; 32], [7; 32], CeremonyKind::Registration),
    )
    .await;
    // Stop the owned proxy and its scoped connections even if the assertion fails.
    proxy.abort();
    let _ = proxy.await;
    store.close().await;
    assert!(
        blocked.load(Ordering::SeqCst),
        "store query must reach the proxy"
    );
    assert!(
        matches!(result, Ok(Err(Error::UncertainCommit))),
        "stalled transport must fail within the adapter deadline: {result:?}"
    );
    b.consume([66; 32], [7; 32], CeremonyKind::Registration)
        .await
        .unwrap();
    a.close().await;
    b.close().await;
    raw.close().await;
}
