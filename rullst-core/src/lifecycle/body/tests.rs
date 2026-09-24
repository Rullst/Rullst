#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::lifecycle::{ApplicationLifecycle, ApplicationLifecycleError};
use axum::body::Bytes;
use std::{future::poll_fn, io, time::Duration};
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};

struct Frames {
    receiver: UnboundedReceiver<Result<Frame<Bytes>, io::Error>>,
    ended: bool,
}
impl HttpBody for Frames {
    type Data = Bytes;
    type Error = io::Error;
    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, io::Error>>> {
        let frame = self.receiver.poll_recv(cx);
        if matches!(frame, Poll::Ready(None)) {
            self.ended = true;
        }
        frame
    }
    fn is_end_stream(&self) -> bool {
        self.ended
    }
}

fn tracked(body: Body) -> (ApplicationLifecycle, Body) {
    let lifecycle = ApplicationLifecycle::new();
    lifecycle.mark_ready().unwrap();
    let guard = lifecycle.try_admit().unwrap();
    let response = track(Response::new(body), guard);
    crate::lifecycle::tests::bounded_begin_draining(&lifecycle).unwrap();
    (lifecycle, response.into_body())
}

#[tokio::test]
async fn streaming_frames_and_trailers_retain_admission_until_end() {
    // TM-CORE-02: header completion and individual frames are not body completion.
    let (sender, receiver) = unbounded_channel();
    let (lifecycle, mut body) = tracked(Body::new(Frames {
        receiver,
        ended: false,
    }));
    assert!(!body.is_end_stream());
    assert!(
        tokio::time::timeout(
            Duration::from_millis(1),
            poll_fn(|cx| Pin::new(&mut body).poll_frame(cx))
        )
        .await
        .is_err()
    );
    assert_eq!(lifecycle.in_flight_requests(), 1);
    sender
        .send(Ok(Frame::data(Bytes::from_static(b"chunk"))))
        .unwrap();
    let frame = poll_fn(|cx| Pin::new(&mut body).poll_frame(cx))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(frame.into_data().unwrap(), b"chunk"[..]);
    assert!(!body.is_end_stream());
    assert!(matches!(
        lifecycle.wait_for_drain(Duration::from_millis(1)).await,
        Err(ApplicationLifecycleError::DrainTimedOut { in_flight: 1 })
    ));
    let mut trailers = axum::http::HeaderMap::new();
    trailers.insert("x-complete", "yes".parse().unwrap());
    sender.send(Ok(Frame::trailers(trailers.clone()))).unwrap();
    let frame = poll_fn(|cx| Pin::new(&mut body).poll_frame(cx))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(frame.into_trailers().unwrap(), trailers);
    assert!(!body.is_end_stream());
    assert_eq!(lifecycle.in_flight_requests(), 1);
    drop(sender);
    assert!(
        poll_fn(|cx| Pin::new(&mut body).poll_frame(cx))
            .await
            .is_none()
    );
    lifecycle
        .wait_for_drain(Duration::from_millis(100))
        .await
        .unwrap();
    assert_eq!(lifecycle.in_flight_requests(), 0);
    drop(body);
    assert_eq!(lifecycle.in_flight_requests(), 0);
}

#[tokio::test]
async fn body_error_or_cancellation_releases_exactly_once() {
    for error in [false, true] {
        let (sender, receiver) = unbounded_channel();
        let (lifecycle, mut body) = tracked(Body::new(Frames {
            receiver,
            ended: false,
        }));
        assert_eq!(lifecycle.in_flight_requests(), 1);
        if error {
            sender
                .send(Err(io::Error::other("fixture body failure")))
                .unwrap();
            assert!(
                poll_fn(|cx| Pin::new(&mut body).poll_frame(cx))
                    .await
                    .unwrap()
                    .is_err()
            );
            assert_eq!(lifecycle.in_flight_requests(), 0);
        }
        drop(body);
        lifecycle
            .wait_for_drain(Duration::from_millis(100))
            .await
            .unwrap();
        assert_eq!(lifecycle.in_flight_requests(), 0);
    }
}

#[tokio::test]
async fn empty_bodies_complete_immediately_and_full_bodies_preserve_size_hints() {
    let (empty, body) = tracked(Body::empty());
    assert!(body.is_end_stream());
    assert_eq!(empty.in_flight_requests(), 0);
    let (lifecycle, mut body) = tracked(Body::from("complete"));
    assert_eq!(body.size_hint().exact(), Some(8));
    assert_eq!(lifecycle.in_flight_requests(), 1);
    let frame = poll_fn(|cx| Pin::new(&mut body).poll_frame(cx))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(frame.into_data().unwrap(), b"complete"[..]);
    // Hyper may skip a further poll once the last frame sets is_end_stream.
    assert!(body.is_end_stream());
    assert_eq!(lifecycle.in_flight_requests(), 0);
}
