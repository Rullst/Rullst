//! The access log's background writer.
//!
//! Request tasks hand each line to one writer thread through a bounded queue
//! and never wait for stdout. Until 13.0.0-alpha.6 every line was a locked,
//! synchronous write to stdout from the request's task, so a slow reader (a
//! pipe, a container log driver, a terminal) held every request that finished
//! meanwhile, and the stdout lock serialized the Tokio workers. The writer
//! now writes the queued lines in batches. When the queue is full a line is
//! dropped and counted instead, and the writer reports the count on stdout
//! with the next batch it writes.

use std::io::Write;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, TrySendError};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

/// Lines the queue holds.
const QUEUED_LINES: usize = 8192;
/// Bytes the queued and not yet written lines may hold together: 2 MiB.
const QUEUED_BYTES: usize = 2_097_152;
/// Pause after each batch, so a busy server hands over its lines in batches
/// instead of waking the writer for every line. It bounds how late a line
/// appears.
const BATCH_INTERVAL: Duration = Duration::from_millis(2);

enum Message {
    Line(String),
    Flush(SyncSender<()>),
}

/// What the writer thread and the request tasks share.
struct Shared {
    /// Bytes of the lines queued or in the batch being written.
    bytes: AtomicUsize,
    /// Lines dropped since the writer last reported them.
    dropped: AtomicU64,
    /// Limit of `bytes`.
    byte_limit: usize,
}

/// A queue of access-log lines and the thread that writes them.
pub(crate) struct AccessLogWriter {
    sender: SyncSender<Message>,
    shared: Arc<Shared>,
}

impl AccessLogWriter {
    /// Starts a writer thread that writes to `output`, with a queue of at most
    /// `lines` lines holding at most `byte_limit` bytes.
    pub(crate) fn spawn(
        lines: usize,
        byte_limit: usize,
        output: impl Write + Send + 'static,
    ) -> std::io::Result<Self> {
        let (sender, receiver) = std::sync::mpsc::sync_channel(lines);
        let shared = Arc::new(Shared {
            bytes: AtomicUsize::new(0),
            dropped: AtomicU64::new(0),
            byte_limit,
        });
        let writer_shared = shared.clone();
        std::thread::Builder::new()
            .name("rullst-access-log".to_string())
            .spawn(move || run(&receiver, &writer_shared, output))?;
        Ok(Self { sender, shared })
    }

    /// Queues `line`, without its newline. Returns `false`, and counts the
    /// line as dropped, when the queue has no room for it.
    pub(crate) fn submit(&self, line: String) -> bool {
        let length = line.len();
        let queued = self.shared.bytes.fetch_add(length, Ordering::Relaxed);
        if queued + length > self.shared.byte_limit {
            return self.refuse(length);
        }
        match self.sender.try_send(Message::Line(line)) {
            Ok(()) => true,
            Err(_) => self.refuse(length),
        }
    }

    fn refuse(&self, length: usize) -> bool {
        self.shared.bytes.fetch_sub(length, Ordering::Relaxed);
        self.shared.dropped.fetch_add(1, Ordering::Relaxed);
        false
    }

    /// Waits until every line queued before this call is written, for at
    /// most `timeout`. Returns whether they were.
    pub(crate) fn flush(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        let (done, written) = std::sync::mpsc::sync_channel(1);
        let mut message = Message::Flush(done);
        loop {
            let waited = deadline.saturating_duration_since(Instant::now()).is_zero();
            match self.sender.try_send(message) {
                Ok(()) => break,
                Err(TrySendError::Full(returned)) if !waited => {
                    message = returned;
                    std::thread::sleep(Duration::from_millis(1));
                }
                Err(_) => return false,
            }
        }
        written
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .is_ok()
    }
}

/// Lines taken from the queue and not yet written.
#[derive(Default)]
struct Batch {
    text: Vec<u8>,
    /// The length the lines counted when they were queued.
    counted: usize,
}

/// The writer thread: waits for a line, takes every line queued behind it,
/// writes them as one batch, then pauses for [`BATCH_INTERVAL`]. A batch
/// holds at most the queue's byte limit, because its lines keep their bytes
/// counted until they are written. Write errors are ignored, like the rest
/// of the console output.
fn run(receiver: &Receiver<Message>, shared: &Shared, mut output: impl Write) {
    let mut batch = Batch::default();
    while let Ok(first) = receiver.recv() {
        let mut next = Some(first);
        while let Some(message) = next {
            match message {
                Message::Line(line) => {
                    batch.text.extend_from_slice(line.as_bytes());
                    batch.text.push(b'\n');
                    batch.counted += line.len();
                }
                Message::Flush(done) => {
                    write_batch(&mut output, &mut batch, shared);
                    let _ = done.try_send(());
                }
            }
            next = receiver.try_recv().ok();
        }
        write_batch(&mut output, &mut batch, shared);
        std::thread::sleep(BATCH_INTERVAL);
    }
}

/// Writes `batch`, then the count of lines dropped since the last report, if
/// any, and releases the bytes the batch's lines counted.
fn write_batch(output: &mut impl Write, batch: &mut Batch, shared: &Shared) {
    let dropped = shared.dropped.swap(0, Ordering::Relaxed);
    if dropped > 0 {
        let _ = writeln!(
            batch.text,
            "[HTTP] access log: {dropped} lines dropped because stdout did not keep up"
        );
    }
    if !batch.text.is_empty() {
        let _ = output.write_all(&batch.text).and_then(|()| output.flush());
        batch.text.clear();
    }
    shared.bytes.fetch_sub(batch.counted, Ordering::Relaxed);
    batch.counted = 0;
}

/// Stdout, locked for each batch only, so other console output (start-up
/// and shutdown lines, `println!`) is never held up for longer.
struct Stdout;

impl Write for Stdout {
    // The writer only calls `write_all`, which the child-process tests of
    // `console` read back; `write` and `flush` complete the trait.
    #[cfg_attr(mutants, mutants::skip)]
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        std::io::stdout().lock().write(buf)
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        std::io::stdout().lock().write_all(buf)
    }

    #[cfg_attr(mutants, mutants::skip)]
    fn flush(&mut self) -> std::io::Result<()> {
        std::io::stdout().lock().flush()
    }
}

static WRITER: OnceLock<Option<AccessLogWriter>> = OnceLock::new();

#[cfg(test)]
thread_local! {
    /// Lines this thread handed to [`write_line`], for tests that check
    /// whether a stack has the access log.
    pub(crate) static QUEUED_HERE: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
thread_local! {
    /// The lines themselves, for tests that read what a stack wrote.
    pub(crate) static LINES_HERE: std::cell::RefCell<Vec<String>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// Writes `line` to stdout through the process's writer thread, started on
/// first use. Without a writer thread (the operating system refused one),
/// the line is written directly.
pub(crate) fn write_line(line: String) {
    #[cfg(test)]
    QUEUED_HERE.with(|count| count.set(count.get() + 1));
    #[cfg(test)]
    LINES_HERE.with(|lines| lines.borrow_mut().push(line.clone()));
    let writer =
        WRITER.get_or_init(|| AccessLogWriter::spawn(QUEUED_LINES, QUEUED_BYTES, Stdout).ok());
    match writer {
        Some(writer) => {
            writer.submit(line);
        }
        None => super::console::stdout_line(format_args!("{line}")),
    }
}

/// Waits, for at most `timeout`, until the lines queued so far are on stdout.
/// Does nothing when no line was ever queued.
// Whether it waited is not observable with stdout as the output; the
// writer's own `flush` is tested with an output that blocks.
#[cfg_attr(mutants, mutants::skip)]
pub(crate) fn flush(timeout: Duration) -> bool {
    WRITER
        .get()
        .and_then(Option::as_ref)
        .is_none_or(|writer| writer.flush(timeout))
}

#[cfg(test)]
#[path = "access_log_writer_tests.rs"]
mod tests;
