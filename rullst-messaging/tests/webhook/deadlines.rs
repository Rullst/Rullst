use super::{server::Receiver, support::*};
use std::sync::atomic::AtomicUsize;

/// Shares the receiver's time and moves it forward once, at one chosen reading.
#[derive(Clone)]
struct JumpClock {
    time: ManualClock,
    readings: Arc<AtomicUsize>,
    jump_at: Arc<AtomicUsize>,
}

impl Clock for JumpClock {
    fn now_millis(&self) -> rullst_messaging::Result<i64> {
        if self.readings.fetch_add(1, Ordering::SeqCst) == self.jump_at.load(Ordering::SeqCst) {
            // Two days: past the one-day delivery window and the 30 s lease.
            self.time.advance(2 * 86_400_000);
        }
        self.time.now_millis()
    }
}

/// Whichever clock reading of a dispatch jumps past the delivery window, no
/// request leaves with a signature made after the jump.
#[tokio::test]
async fn no_request_is_signed_after_the_clock_jumps_past_the_window() {
    let time = ManualClock::new();
    let start = time.now_millis().unwrap();
    let receiver = Receiver::start(time.clone(), false).await;
    let (path, url) = fixture();
    let mut jumps = 0;
    for offset in 0..200 {
        time.0.store(start, Ordering::SeqCst);
        let clock = JumpClock {
            time: time.clone(),
            readings: Arc::default(),
            jump_at: Arc::new(AtomicUsize::new(usize::MAX)),
        };
        let store = WebhookOutbox::open(
            &url,
            config(&unique(), receiver.destination()),
            key(),
            storage(),
            clock.clone(),
        )
        .await
        .unwrap();
        store
            .enqueue("jump", "ready", b"{}".to_vec())
            .await
            .unwrap();
        clock.jump_at.store(
            clock.readings.load(Ordering::SeqCst) + offset,
            Ordering::SeqCst,
        );
        let _ = store.dispatch_next("worker").await;
        store.close().await;
        if time.now_millis().unwrap() == start {
            // The dispatch read the clock fewer times than `offset`.
            break;
        }
        jumps += 1;
    }
    assert!(jumps > 3, "{jumps}");
    let requests = receiver.requests.lock().await;
    assert!(!requests.is_empty());
    for request in requests.iter() {
        assert_eq!(
            request.headers["rullst-webhook-timestamp"],
            (start / 1000).to_string()
        );
    }
    drop(requests);
    cleanup(&path);
}

/// The HTTP timeout ends with the delivery window: the slow receiver is
/// abandoned after its last second and the event expires.
#[tokio::test]
async fn the_request_timeout_ends_with_the_delivery_window() {
    let clock = ManualClock::new();
    let receiver = Receiver::start(clock.clone(), false).await;
    let (path, url) = fixture();
    let policy = config(&unique(), receiver.destination())
        .with_delivery_window(Duration::from_secs(60))
        .unwrap();
    let store = WebhookOutbox::open(&url, policy, key(), storage(), clock.clone())
        .await
        .unwrap();
    store
        .enqueue("slow", "ready", b"{}".to_vec())
        .await
        .unwrap();
    clock.advance(59_000);
    receiver.pause.store(true, Ordering::SeqCst);
    let release = receiver.release.clone();
    let releaser = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(4)).await;
        release.notify_one();
    });
    let started = std::time::Instant::now();
    let outcome = store.dispatch_next("worker").await.unwrap();
    assert!(
        matches!(outcome, WebhookDispatch::DeadLettered { attempt: 1, .. }),
        "{outcome:?}"
    );
    assert!(started.elapsed() < Duration::from_secs(4));
    assert_eq!(
        store.failed(1).await.unwrap()[0].failure_code(),
        "webhook.expired"
    );
    releaser.abort();
    store.close().await;
    cleanup(&path);
}
