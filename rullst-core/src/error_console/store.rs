//! Bounded, expiring store of the panic contexts that
//! `cargo rullst ai fix <error-id>` reads.
//!
//! The development console records what it already shows for a panic: the
//! message, the source location, a short backtrace of project frames and the
//! request method and path (never the query string, headers, cookies or
//! body). Each entry gets a random 128-bit id. At most [`MAX_ENTRIES`] are
//! kept, each bounded in size, and an entry expires after [`TTL`]. The store
//! lives in process memory: a restarted server forgets every id.

use serde::Serialize;
use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Version of the JSON document served for one entry.
pub(crate) const SCHEMA: &str = "rullst.error-context.v1";
/// Most entries kept; the oldest is dropped first.
pub(crate) const MAX_ENTRIES: usize = 32;
/// Lifetime of an entry.
pub(crate) const TTL: Duration = Duration::from_secs(30 * 60);
const MAX_MESSAGE_BYTES: usize = 2 * 1024;
const MAX_FILE_BYTES: usize = 512;
const MAX_PATH_BYTES: usize = 512;
const MAX_METHOD_BYTES: usize = 16;
const MAX_FRAMES: usize = 16;
const MAX_FRAME_BYTES: usize = 320;

/// One recorded panic, as served to the CLI.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct ErrorContext {
    pub(crate) schema: &'static str,
    pub(crate) id: String,
    pub(crate) message: String,
    pub(crate) file: Option<String>,
    pub(crate) line: Option<u32>,
    /// Project frames only (`symbol at file:line:column`), at most [`MAX_FRAMES`].
    pub(crate) backtrace: Vec<String>,
    pub(crate) method: String,
    /// The request path without its query string.
    pub(crate) path: String,
    pub(crate) expires_in_seconds: u64,
}

struct Entry {
    context: ErrorContext,
    captured: Instant,
}

/// The entries of one process (or of one test).
#[derive(Default)]
pub(crate) struct Store {
    entries: VecDeque<Entry>,
}

/// Truncates on a character boundary.
fn bounded(text: &str, max: usize) -> String {
    if text.len() <= max {
        return text.to_string();
    }
    let mut end = max;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &text[..end])
}

/// The frames of a `std::backtrace::Backtrace` display whose location is in
/// the project (not the standard library or a Cargo dependency).
pub(crate) fn project_frames(backtrace: &str) -> Vec<String> {
    let mut frames = Vec::new();
    let mut symbol: Option<&str> = None;
    for line in backtrace.lines() {
        let trimmed = line.trim();
        if let Some(location) = trimmed.strip_prefix("at ") {
            let dependency = super::parser::DEPENDENCY_FRAME_MARKERS
                .iter()
                .any(|marker| location.contains(marker));
            if let Some(name) = symbol.take()
                && !dependency
            {
                frames.push(bounded(&format!("{name} at {location}"), MAX_FRAME_BYTES));
                if frames.len() == MAX_FRAMES {
                    break;
                }
            }
        } else if let Some((index, name)) = trimmed.split_once(": ")
            && !index.is_empty()
            && index.bytes().all(|byte| byte.is_ascii_digit())
        {
            symbol = Some(name);
        }
    }
    frames
}

impl Store {
    fn expire(&mut self, now: Instant) {
        self.entries
            .retain(|entry| now.saturating_duration_since(entry.captured) < TTL);
    }

    /// Records one panic and returns its id.
    pub(crate) fn record(
        &mut self,
        now: Instant,
        message: &str,
        location: Option<(String, u32)>,
        backtrace: Option<&str>,
        method: &str,
        path: &str,
    ) -> String {
        self.expire(now);
        while self.entries.len() >= MAX_ENTRIES {
            self.entries.pop_front();
        }
        let id = uuid::Uuid::new_v4().simple().to_string();
        let (file, line) = match location {
            Some((file, line)) => (Some(bounded(&file, MAX_FILE_BYTES)), Some(line)),
            None => (None, None),
        };
        let context = ErrorContext {
            schema: SCHEMA,
            id: id.clone(),
            message: bounded(message, MAX_MESSAGE_BYTES),
            file,
            line,
            backtrace: backtrace.map(project_frames).unwrap_or_default(),
            method: bounded(method, MAX_METHOD_BYTES),
            path: bounded(path, MAX_PATH_BYTES),
            expires_in_seconds: TTL.as_secs(),
        };
        self.entries.push_back(Entry {
            context,
            captured: now,
        });
        id
    }

    /// The unexpired entry with `id`.
    pub(crate) fn get(&mut self, now: Instant, id: &str) -> Option<ErrorContext> {
        self.expire(now);
        let entry = self.entries.iter().find(|entry| entry.context.id == id)?;
        let mut context = entry.context.clone();
        context.expires_in_seconds = TTL
            .saturating_sub(now.saturating_duration_since(entry.captured))
            .as_secs();
        Some(context)
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }
}

static STORE: Mutex<Store> = Mutex::new(Store {
    entries: VecDeque::new(),
});

fn with_store<T>(action: impl FnOnce(&mut Store) -> T) -> T {
    let mut store = STORE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    action(&mut store)
}

/// Records a panic in the process store and returns its id.
pub(crate) fn record(
    message: &str,
    location: Option<(String, u32)>,
    backtrace: Option<&str>,
    method: &str,
    path: &str,
) -> String {
    with_store(|store| store.record(Instant::now(), message, location, backtrace, method, path))
}

/// The unexpired entry with `id` in the process store.
pub(crate) fn lookup(id: &str) -> Option<ErrorContext> {
    with_store(|store| store.get(Instant::now(), id))
}

/// Whether `id` has the shape of an id this store issues.
pub(crate) fn valid_id(id: &str) -> bool {
    id.len() == 32
        && id
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
