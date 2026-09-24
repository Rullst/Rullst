#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use axum::body::{Body, to_bytes};
use axum::http::Request as HttpRequest;
use axum::routing::get;
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use std::task::{Context, Poll, Wake, Waker};
use tower::ServiceExt;

struct WakeSignal(AtomicBool);
impl Wake for WakeSignal {
    fn wake(self: Arc<Self>) {
        self.0.store(true, AtomicOrdering::SeqCst);
    }
}

// A Tokio timeout cannot interrupt a synchronous transition loop. A separate
// OS thread lets the test fail within a bounded wait even under that mutation;
// any stuck test-only worker ends when the test process exits.
pub(crate) fn bounded_begin_draining(
    lifecycle: &ApplicationLifecycle,
) -> Result<(), ApplicationLifecycleError> {
    let lifecycle = lifecycle.clone();
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || {
        let _ = send.send(lifecycle.begin_draining());
    });
    let result = receive
        .recv_timeout(Duration::from_secs(5))
        .expect("drain transition must return without looping");
    worker.join().expect("transition worker must not panic");
    result
}

#[test]
fn registry_is_immutable_bounded_and_validated() {
    assert!(matches!(
        ApplicationLifecycle::with_required_components([""]),
        Err(ApplicationLifecycleError::InvalidComponentLabel(_))
    ));
    assert!(ApplicationLifecycle::with_required_components(["a".repeat(64)]).is_ok());
    assert!(matches!(
        ApplicationLifecycle::with_required_components(["a".repeat(65)]),
        Err(ApplicationLifecycleError::InvalidComponentLabel(_))
    ));
    assert!(matches!(
        ApplicationLifecycle::with_required_components(["database/primary"]),
        Err(ApplicationLifecycleError::InvalidComponentLabel(_))
    ));
    assert!(matches!(
        ApplicationLifecycle::with_required_components(["db", "db"]),
        Err(ApplicationLifecycleError::DuplicateComponent(_))
    ));
    let too_many = (0..=MAX_REQUIRED_COMPONENTS).map(|index| format!("component-{index}"));
    assert!(matches!(
        ApplicationLifecycle::with_required_components(too_many),
        Err(ApplicationLifecycleError::TooManyComponents)
    ));

    let lifecycle = ApplicationLifecycle::with_required_components(["database"]).unwrap();
    assert!(matches!(
        lifecycle.set_component_ready("cache", true),
        Err(ApplicationLifecycleError::UnknownComponent(_))
    ));
    let debug = format!("{lifecycle:?}");
    assert!(!debug.contains("database"));
    for field in ["ApplicationLifecycle", "Starting", "required_components: 1"] {
        assert!(debug.contains(field), "missing public diagnostic {field}");
    }
}

#[test]
fn readiness_requires_phase_and_every_component() {
    let lifecycle = ApplicationLifecycle::with_required_components(["database", "queue"]).unwrap();
    assert_eq!(lifecycle.phase(), ApplicationPhase::Starting);
    assert!(!lifecycle.snapshot().ready);

    lifecycle.mark_ready().unwrap();
    assert!(!lifecycle.snapshot().ready);
    lifecycle.set_component_ready("database", true).unwrap();
    lifecycle.set_component_ready("queue", true).unwrap();
    assert!(lifecycle.snapshot().ready);

    lifecycle.set_component_ready("queue", false).unwrap();
    assert!(!lifecycle.snapshot().ready);
    bounded_begin_draining(&lifecycle).unwrap();
    assert_eq!(lifecycle.phase(), ApplicationPhase::Draining);
    assert!(matches!(
        lifecycle.mark_ready(),
        Err(ApplicationLifecycleError::InvalidTransition { .. })
    ));
    lifecycle.mark_stopped();
    assert_eq!(lifecycle.phase(), ApplicationPhase::Stopped);
}

#[tokio::test]
async fn drain_rejects_new_requests_and_waits_for_an_admitted_one() {
    // TM-CORE-02: draining must close admission without cancelling work that
    // was already accepted, and the wait must remain bounded.
    let lifecycle = ApplicationLifecycle::new();
    lifecycle.mark_ready().unwrap();
    let entered = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let handler_entered = Arc::clone(&entered);
    let handler_release = Arc::clone(&release);
    let app = apply_lifecycle(
        Router::new().route(
            "/work",
            get(move || {
                let entered = Arc::clone(&handler_entered);
                let release = Arc::clone(&handler_release);
                async move {
                    entered.notify_one();
                    release.notified().await;
                    "done"
                }
            }),
        ),
        lifecycle.clone(),
    );

    let first_app = app.clone();
    let first = tokio::spawn(async move {
        first_app
            .oneshot(HttpRequest::get("/work").body(Body::empty()).unwrap())
            .await
            .unwrap()
    });
    tokio::time::timeout(Duration::from_secs(5), entered.notified())
        .await
        .expect("the admitted handler must start");
    assert_eq!(lifecycle.in_flight_requests(), 1);
    bounded_begin_draining(&lifecycle).unwrap();

    // A broken admission gate can enter the blocked handler a second time.
    // Bound that request independently of the drain's own timeout.
    let rejected = tokio::time::timeout(
        Duration::from_secs(5),
        app.oneshot(HttpRequest::get("/work").body(Body::empty()).unwrap()),
    )
    .await
    .expect("draining must reject the new request without entering its handler")
    .unwrap();
    assert_eq!(rejected.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(rejected.headers()[header::RETRY_AFTER], "1");
    assert!(matches!(
        lifecycle.wait_for_drain(Duration::from_millis(1)).await,
        Err(ApplicationLifecycleError::DrainTimedOut { in_flight: 1 })
    ));

    release.notify_one();
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(5), first)
            .await
            .expect("the released handler must finish")
            .unwrap()
            .status(),
        StatusCode::OK
    );
    lifecycle
        .wait_for_drain(Duration::from_millis(100))
        .await
        .unwrap();
    assert_eq!(lifecycle.in_flight_requests(), 0);
}

#[tokio::test]
async fn response_headers_do_not_complete_an_unconsumed_body() {
    // TM-CORE-02: a returned response can still own unfinished application work.
    let lifecycle = ApplicationLifecycle::new();
    lifecycle.mark_ready().unwrap();
    let app = apply_lifecycle(
        Router::new().route("/body", get(|| async { "accepted response" })),
        lifecycle.clone(),
    );
    let response = app
        .oneshot(HttpRequest::get("/body").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    bounded_begin_draining(&lifecycle).unwrap();
    assert!(matches!(
        lifecycle.wait_for_drain(Duration::from_millis(1)).await,
        Err(ApplicationLifecycleError::DrainTimedOut { in_flight: 1 })
    ));
    let bytes = to_bytes(response.into_body(), 1024).await.unwrap();
    assert_eq!(bytes.as_ref(), b"accepted response");
    lifecycle
        .wait_for_drain(Duration::from_millis(100))
        .await
        .unwrap();
    assert_eq!(lifecycle.in_flight_requests(), 0);
}

#[tokio::test]
async fn probes_bypass_application_admission_without_exposing_components() {
    let lifecycle = ApplicationLifecycle::with_required_components(["private-db"]).unwrap();
    let app = apply_lifecycle(
        Router::new()
            .route("/health", get(|| async { "health" }))
            .route("/ready", get(|| async { "ready" }))
            .route("/work", get(|| async { "work" })),
        lifecycle,
    );

    for request in [
        HttpRequest::get("/health").body(Body::empty()).unwrap(),
        HttpRequest::head("/ready").body(Body::empty()).unwrap(),
    ] {
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
    let response = app
        .oneshot(HttpRequest::get("/work").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = to_bytes(response.into_body(), 1024).await.unwrap();
    assert!(
        !String::from_utf8(body.to_vec())
            .unwrap()
            .contains("private-db")
    );
}

#[test]
fn poisoned_component_state_fails_closed() {
    let lifecycle = ApplicationLifecycle::with_required_components(["database"]).unwrap();
    lifecycle.mark_ready().unwrap();
    let inner = Arc::clone(&lifecycle.inner);
    let panicked = Arc::new(AtomicBool::new(false));
    let panicked_in_thread = Arc::clone(&panicked);
    let _ = std::thread::spawn(move || {
        let _guard = inner.components.write().unwrap();
        panicked_in_thread.store(true, AtomicOrdering::Release);
        panic!("poison test-only component lock");
    })
    .join();
    assert!(panicked.load(AtomicOrdering::Acquire));
    let snapshot = lifecycle.snapshot();
    assert!(!snapshot.ready);
    assert!(!snapshot.state_available);
    assert_eq!(
        lifecycle.set_component_ready("database", true),
        Err(ApplicationLifecycleError::StateUnavailable)
    );
}

#[test]
fn draining_is_idempotent_and_cannot_reverse_a_terminal_stop() {
    for ready in [false, true] {
        let lifecycle = ApplicationLifecycle::new();
        if ready {
            lifecycle.mark_ready().unwrap();
        }
        bounded_begin_draining(&lifecycle).unwrap();
        bounded_begin_draining(&lifecycle).unwrap();
        assert_eq!(lifecycle.phase(), ApplicationPhase::Draining);
        lifecycle.mark_stopped();
        assert_eq!(
            bounded_begin_draining(&lifecycle),
            Err(ApplicationLifecycleError::InvalidTransition {
                from: ApplicationPhase::Stopped,
                to: ApplicationPhase::Draining,
            })
        );
        assert_eq!(lifecycle.phase(), ApplicationPhase::Stopped);
    }
}

#[tokio::test]
async fn drain_wait_rejects_zero_and_accepts_the_inclusive_ten_minute_limit() {
    let lifecycle = ApplicationLifecycle::new();
    bounded_begin_draining(&lifecycle).unwrap();
    for invalid in [
        Duration::ZERO,
        Duration::from_secs(600) + Duration::from_nanos(1),
    ] {
        assert_eq!(
            lifecycle.wait_for_drain(invalid).await,
            Err(ApplicationLifecycleError::InvalidDrainWait)
        );
    }
    for valid in [Duration::from_nanos(1), Duration::from_secs(600)] {
        tokio::time::timeout(Duration::from_secs(5), lifecycle.wait_for_drain(valid))
            .await
            .expect("an empty drain must finish without waiting for its deadline")
            .unwrap();
    }
}

#[tokio::test]
async fn last_release_wakes_an_already_pending_drain_before_its_deadline() {
    for stopped in [false, true] {
        let lifecycle = ApplicationLifecycle::new();
        lifecycle.mark_ready().unwrap();
        let first = lifecycle.try_admit().unwrap();
        let last = lifecycle.try_admit().unwrap();
        bounded_begin_draining(&lifecycle).unwrap();
        if stopped {
            lifecycle.mark_stopped();
        }
        let signal = Arc::new(WakeSignal(AtomicBool::new(false)));
        let waker = Waker::from(Arc::clone(&signal));
        let mut context = Context::from_waker(&waker);
        let mut waiting = std::pin::pin!(lifecycle.wait_for_drain(Duration::from_secs(600)));
        assert!(waiting.as_mut().poll(&mut context).is_pending());
        drop(first);
        assert_eq!(lifecycle.in_flight_requests(), 1);
        assert!(waiting.as_mut().poll(&mut context).is_pending());
        // Spurious wakes are allowed, but the final release must wake a waiter.
        signal.0.store(false, AtomicOrdering::SeqCst);
        drop(last);
        assert!(signal.0.load(AtomicOrdering::SeqCst));
        assert_eq!(waiting.as_mut().poll(&mut context), Poll::Ready(Ok(())));
        assert_eq!(lifecycle.in_flight_requests(), 0);
    }
}

#[tokio::test]
async fn cancelling_the_handler_before_response_headers_releases_admission() {
    let lifecycle = ApplicationLifecycle::new();
    lifecycle.mark_ready().unwrap();
    let entered = Arc::new(AtomicBool::new(false));
    let handler_entered = Arc::clone(&entered);
    let app = apply_lifecycle(
        Router::new().route(
            "/pending",
            get(move || {
                let entered = Arc::clone(&handler_entered);
                async move {
                    entered.store(true, AtomicOrdering::SeqCst);
                    std::future::pending::<&'static str>().await
                }
            }),
        ),
        lifecycle.clone(),
    );
    let mut request =
        Box::pin(app.oneshot(HttpRequest::get("/pending").body(Body::empty()).unwrap()));
    let mut context = Context::from_waker(Waker::noop());
    assert!(request.as_mut().poll(&mut context).is_pending());
    assert!(entered.load(AtomicOrdering::SeqCst));
    assert_eq!(lifecycle.in_flight_requests(), 1);
    bounded_begin_draining(&lifecycle).unwrap();
    // Drop the owned future, not just a Pin reference to it.
    drop(request);
    assert_eq!(lifecycle.in_flight_requests(), 0);
    lifecycle
        .wait_for_drain(Duration::from_secs(1))
        .await
        .unwrap();
}

#[tokio::test]
async fn last_request_release_wakes_every_registered_drain_waiter() {
    let lifecycle = ApplicationLifecycle::new();
    lifecycle.mark_ready().unwrap();
    let request = lifecycle.try_admit().unwrap();
    bounded_begin_draining(&lifecycle).unwrap();
    let signals: Vec<_> = (0..3)
        .map(|_| Arc::new(WakeSignal(AtomicBool::new(false))))
        .collect();
    let wakers: Vec<_> = signals.iter().map(|s| Waker::from(Arc::clone(s))).collect();
    let mut waiters: Vec<_> = (0..3)
        .map(|_| Box::pin(lifecycle.wait_for_drain(Duration::from_secs(600))))
        .collect();
    for (waiter, waker) in waiters.iter_mut().zip(&wakers) {
        assert!(
            waiter
                .as_mut()
                .poll(&mut Context::from_waker(waker))
                .is_pending()
        );
    }
    for signal in &signals {
        signal.0.store(false, AtomicOrdering::SeqCst);
    }
    drop(request);
    for ((waiter, waker), signal) in waiters.iter_mut().zip(&wakers).zip(&signals) {
        assert!(
            signal.0.load(AtomicOrdering::SeqCst),
            "a registered waiter was stranded"
        );
        assert_eq!(
            waiter.as_mut().poll(&mut Context::from_waker(waker)),
            Poll::Ready(Ok(()))
        );
    }
}

#[tokio::test]
async fn cancelling_one_drain_wait_does_not_release_work_or_strand_another_waiter() {
    let lifecycle = ApplicationLifecycle::new();
    lifecycle.mark_ready().unwrap();
    let request = lifecycle.try_admit().unwrap();
    bounded_begin_draining(&lifecycle).unwrap();
    let mut abandoned = Box::pin(lifecycle.wait_for_drain(Duration::from_secs(600)));
    let mut surviving = Box::pin(lifecycle.wait_for_drain(Duration::from_secs(600)));
    let signal = Arc::new(WakeSignal(AtomicBool::new(false)));
    let waker = Waker::from(Arc::clone(&signal));
    let mut context = Context::from_waker(&waker);
    assert!(abandoned.as_mut().poll(&mut context).is_pending());
    assert!(surviving.as_mut().poll(&mut context).is_pending());
    drop(abandoned);
    assert_eq!(lifecycle.in_flight_requests(), 1);
    assert!(surviving.as_mut().poll(&mut context).is_pending());
    signal.0.store(false, AtomicOrdering::SeqCst);
    drop(request);
    assert!(signal.0.load(AtomicOrdering::SeqCst));
    assert_eq!(surviving.as_mut().poll(&mut context), Poll::Ready(Ok(())));
}

#[test]
fn readiness_withdrawal_closes_admission_without_releasing_existing_work() {
    let lifecycle = ApplicationLifecycle::with_required_components(["database"]).unwrap();
    lifecycle.set_component_ready("database", true).unwrap();
    lifecycle.mark_ready().unwrap();
    let first = lifecycle.try_admit().unwrap();
    lifecycle.set_component_ready("database", false).unwrap();
    assert!(matches!(
        lifecycle.try_admit(),
        Err(ApplicationLifecycleError::RequestNotAdmitted { .. })
    ));
    assert_eq!(lifecycle.in_flight_requests(), 1);
    lifecycle.set_component_ready("database", true).unwrap();
    let second = lifecycle.try_admit().unwrap();
    assert_eq!(lifecycle.in_flight_requests(), 2);
    bounded_begin_draining(&lifecycle).unwrap();
    // A healthy dependency cannot reopen application admission during shutdown.
    lifecycle.set_component_ready("database", true).unwrap();
    assert!(matches!(
        lifecycle.try_admit(),
        Err(ApplicationLifecycleError::RequestNotAdmitted { .. })
    ));
    assert_eq!(lifecycle.in_flight_requests(), 2);
    drop(first);
    drop(second);
    assert_eq!(lifecycle.in_flight_requests(), 0);
}
