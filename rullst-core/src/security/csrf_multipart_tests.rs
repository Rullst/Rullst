//! Streaming behaviour of the bounded `_token` reader and its replay body.

#![allow(clippy::expect_used)]

use super::*;
use futures_util::StreamExt;
use std::time::Duration;

/// A complete form holding only `_token=abc`, 70 bytes long.
const TOKEN_FORM: &str =
    "--XyZ\r\nContent-Disposition: form-data; name=\"_token\"\r\n\r\nabc\r\n--XyZ--\r\n";

/// A body that yields `frames` and then never ends.
fn stalled_body(frames: Vec<Vec<u8>>) -> Body {
    let frames = frames
        .into_iter()
        .map(|frame| Ok::<_, std::io::Error>(Bytes::from(frame)));
    Body::from_stream(futures_util::stream::iter(frames).chain(futures_util::stream::pending()))
}

/// `read_token` on a helper thread with its own runtime, so a reader that
/// waits for more input or stops making progress fails the test at the
/// budget instead of hanging the test binary.
fn read_within_budget(body: Body) -> (Option<Vec<u8>>, Body) {
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        let _ = sender.send(runtime.block_on(read_token(body, "XyZ")));
    });
    receiver
        .recv_timeout(Duration::from_secs(10))
        .expect("read_token returns without more input")
        .expect("readable body")
}

async fn next_frame(body: &mut Body) -> Option<Frame<Bytes>> {
    std::future::poll_fn(|cx| Pin::new(&mut *body).poll_frame(cx))
        .await
        .map(|frame| frame.expect("frame"))
}

#[test]
fn a_part_without_headers_is_skipped_as_a_whole() {
    let skipped = "--XyZ\r\n\r\nx\r\n--XyZ\r\nContent-Disposition: form-data; name=\"_token\"\r\n\r\nabc\r\n--XyZ--";
    assert_eq!(
        scan(skipped.as_bytes(), b"XyZ"),
        Scan::Token(b"abc".to_vec())
    );
    // Content that only looks like a delimiter after the empty header block
    // belongs to the headerless part.
    let nested = "--XyZ\r\n\r\n--XyZ\r\nContent-Disposition: form-data; name=\"_token\"\r\n\r\nabc\r\n--XyZ--";
    assert_eq!(scan(nested.as_bytes(), b"XyZ"), Scan::Missing);
}

#[test]
fn an_empty_body_has_no_token() {
    let (token, _) = read_within_budget(Body::empty());
    assert_eq!(token, None);
}

#[test]
fn the_prefix_is_rescanned_once_it_has_doubled() {
    assert_eq!(TOKEN_FORM.len(), 70);
    // The token ends in the second frame, which doubles the 35 bytes scanned
    // after the first one, so no further frame is awaited.
    let (token, _) = read_within_budget(stalled_body(vec![
        TOKEN_FORM.as_bytes()[..35].to_vec(),
        TOKEN_FORM.as_bytes()[35..].to_vec(),
    ]));
    assert_eq!(token, Some(b"abc".to_vec()));
}

#[test]
fn reading_stops_after_sixty_four_kibibytes_of_a_body_that_never_ends() {
    let mut first = b"--XyZ\r\nContent-Disposition: form-data; name=\"file\"\r\n\r\n".to_vec();
    first.resize(1_024, b'a');
    let mut frames = vec![first];
    frames.extend((0..69).map(|_| vec![b'a'; 1_024]));
    let (token, _) = read_within_budget(stalled_body(frames));
    assert_eq!(token, None);
}

#[tokio::test]
async fn the_replay_body_reports_its_buffered_frames() {
    let (token, mut body) = read_token(Body::from(TOKEN_FORM), "XyZ")
        .await
        .expect("readable body");
    assert_eq!(token, Some(b"abc".to_vec()));
    assert_eq!(body.size_hint().exact(), Some(70));
    assert!(!body.is_end_stream());

    let frame = next_frame(&mut body).await.expect("replayed frame");
    assert_eq!(
        frame.data_ref().map(Bytes::as_ref),
        Some(TOKEN_FORM.as_bytes())
    );
    assert!(body.is_end_stream());
    assert_eq!(body.size_hint().exact(), Some(0));
    assert!(next_frame(&mut body).await.is_none());
}
