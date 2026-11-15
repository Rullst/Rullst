#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use std::sync::Mutex;
use std::sync::mpsc::{Sender, channel};

const DROPPED_ONE: &str = "[HTTP] access log: 1 lines dropped because stdout did not keep up\n";

/// An output that records each `write_all` and, when gated, holds the first
/// one until the test opens the gate.
struct Recorder {
    writes: Arc<Mutex<Vec<String>>>,
    gate: Option<Receiver<()>>,
    started: Option<Sender<()>>,
}

impl Write for Recorder {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.write_all(buf).map(|()| buf.len())
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        if let Some(started) = self.started.take() {
            let _ = started.send(());
        }
        if let Some(gate) = self.gate.take() {
            let _ = gate.recv_timeout(Duration::from_secs(30));
        }
        self.writes
            .lock()
            .unwrap()
            .push(String::from_utf8(buf.to_vec()).unwrap());
        Ok(())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct Gated {
    writer: AccessLogWriter,
    writes: Arc<Mutex<Vec<String>>>,
    open: Sender<()>,
}

/// A writer whose output holds its first write until `open` is sent. Returns
/// once the writer thread is inside that write with `first`.
fn gated(lines: usize, byte_limit: usize, first: &str) -> Gated {
    let writes = Arc::new(Mutex::new(Vec::new()));
    let (open, gate) = channel();
    let (started, inside) = channel();
    let writer = AccessLogWriter::spawn(
        lines,
        byte_limit,
        Recorder {
            writes: writes.clone(),
            gate: Some(gate),
            started: Some(started),
        },
    )
    .unwrap();
    assert!(writer.submit(first.to_string()));
    inside.recv_timeout(Duration::from_secs(30)).unwrap();
    Gated {
        writer,
        writes,
        open,
    }
}

fn written(writes: &Mutex<Vec<String>>) -> Vec<String> {
    writes.lock().unwrap().clone()
}

#[test]
fn lines_queued_while_stdout_is_busy_are_written_as_one_batch() {
    let log = gated(8, 1024, "first");
    for line in ["second", "third", "fourth"] {
        assert!(log.writer.submit(line.to_string()));
    }
    log.open.send(()).unwrap();
    assert!(log.writer.flush(Duration::from_secs(30)));
    assert_eq!(written(&log.writes), ["first\n", "second\nthird\nfourth\n"]);
    // Everything written released its bytes.
    assert_eq!(log.writer.shared.bytes.load(Ordering::Relaxed), 0);
}

#[test]
fn a_full_queue_drops_and_reports_lines_instead_of_waiting() {
    let log = gated(2, 1024, "first");
    assert!(log.writer.submit("second".to_string()));
    assert!(log.writer.submit("third".to_string()));
    // The queue holds two lines; the request goes on without its line.
    assert!(!log.writer.submit("fourth".to_string()));
    assert_eq!(log.writer.shared.dropped.load(Ordering::Relaxed), 1);
    log.open.send(()).unwrap();
    assert!(log.writer.flush(Duration::from_secs(30)));
    assert_eq!(
        written(&log.writes),
        [
            "first\n".to_string(),
            format!("second\nthird\n{DROPPED_ONE}")
        ]
    );
    assert_eq!(log.writer.shared.dropped.load(Ordering::Relaxed), 0);
    assert_eq!(log.writer.shared.bytes.load(Ordering::Relaxed), 0);
}

#[test]
fn the_byte_limit_counts_lines_until_they_are_written() {
    // "x" stays counted while its write is held: 1 + 9 bytes fill the limit.
    let log = gated(8, 10, "x");
    assert_eq!(log.writer.shared.bytes.load(Ordering::Relaxed), 1);
    assert!(log.writer.submit("123456789".to_string()));
    assert!(!log.writer.submit("y".to_string()));
    assert_eq!(log.writer.shared.bytes.load(Ordering::Relaxed), 10);
    log.open.send(()).unwrap();
    assert!(log.writer.flush(Duration::from_secs(30)));
    assert_eq!(
        written(&log.writes),
        ["x\n".to_string(), format!("123456789\n{DROPPED_ONE}")]
    );
    assert_eq!(log.writer.shared.bytes.load(Ordering::Relaxed), 0);
    // A line longer than the limit is never queued.
    assert!(!log.writer.submit("12345678901".to_string()));
    assert!(log.writer.submit("1234567890".to_string()));
}

#[test]
fn flush_waits_for_room_in_a_full_queue() {
    let log = gated(1, 1024, "first");
    assert!(log.writer.submit("second".to_string()));
    let open = log.open.clone();
    let opener = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        open.send(()).unwrap();
    });
    assert!(log.writer.flush(Duration::from_secs(30)));
    opener.join().unwrap();
    assert_eq!(written(&log.writes), ["first\n", "second\n"]);
}

#[test]
fn flush_gives_up_at_its_deadline_while_stdout_is_blocked() {
    let log = gated(8, 1024, "first");
    assert!(log.writer.submit("second".to_string()));
    let started = Instant::now();
    assert!(!log.writer.flush(Duration::from_millis(50)));
    assert!(started.elapsed() >= Duration::from_millis(50));
    // A full queue gives up too.
    let full = gated(1, 1024, "first");
    assert!(full.writer.submit("second".to_string()));
    assert!(!full.writer.flush(Duration::from_millis(50)));
    log.open.send(()).unwrap();
    full.open.send(()).unwrap();
    assert!(log.writer.flush(Duration::from_secs(30)));
    assert_eq!(written(&log.writes), ["first\n", "second\n"]);
}

#[test]
fn write_line_counts_the_lines_this_thread_queued() {
    let before = QUEUED_HERE.with(std::cell::Cell::get);
    write_line("[HTTP] GET /queued-here -> 200 (0.01 ms)".to_string());
    assert_eq!(QUEUED_HERE.with(std::cell::Cell::get), before + 1);
}
