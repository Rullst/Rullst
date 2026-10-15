//! Incremental `multipart/form-data` body parser.
//!
//! It accepts what browsers send: the body starts with the first boundary
//! (no preamble), each boundary is followed by CRLF or by `--` (the end),
//! and bytes after the closing boundary are never read. Buffered input stays
//! bounded by one pushed chunk plus a delimiter, or by the part header limit.

use super::MultipartError;
use super::headers::MAX_PART_HEADER_BYTES;

/// What the parser found next.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Event {
    /// More input is needed: [`Parser::push`] or [`Parser::finish`].
    NeedMore,
    /// A part starts; its raw header block, without the final empty line.
    PartStart(Vec<u8>),
    /// Content of the current part.
    Data(Vec<u8>),
    /// The last content of the current part, which ends here.
    PartEnd(Vec<u8>),
    /// The closing boundary.
    End,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    /// Before the first boundary.
    Start,
    /// After a boundary: CRLF or `--` follows.
    AfterBoundary,
    /// Inside a part's header block.
    Headers,
    /// Inside a part's content.
    Body,
    /// After the closing boundary.
    Done,
}

/// Content held back until this many bytes are pending, so a body arriving
/// in tiny frames does not become one tiny write per frame.
const DATA_EVENT_BYTES: usize = 16 * 1024;

/// Parser of one body.
pub(super) struct Parser {
    /// `CRLF--boundary`, which ends every part's content.
    delimiter: Vec<u8>,
    /// Pushed bytes; those before `start` are consumed.
    buffer: Vec<u8>,
    start: usize,
    state: State,
    /// Pending bytes (from `start`) already searched without a match.
    searched: usize,
    ended: bool,
    consumed: u64,
}

fn malformed(reason: &'static str) -> MultipartError {
    MultipartError::Malformed(reason)
}

impl Parser {
    pub(super) fn new(boundary: &str) -> Self {
        let mut delimiter = b"\r\n--".to_vec();
        delimiter.extend_from_slice(boundary.as_bytes());
        Self {
            delimiter,
            buffer: Vec::new(),
            start: 0,
            state: State::Start,
            searched: 0,
            ended: false,
            consumed: 0,
        }
    }

    /// Adds body bytes. Consumed bytes are dropped once they are at least
    /// half of the buffer, so each byte is moved a bounded number of times.
    pub(super) fn push(&mut self, bytes: &[u8]) {
        if self.start >= self.buffer.len() / 2 {
            self.buffer.drain(..self.start);
            self.start = 0;
        }
        self.buffer.extend_from_slice(bytes);
    }

    /// Marks the end of the body.
    pub(super) fn finish(&mut self) {
        self.ended = true;
    }

    /// Body bytes consumed so far, up to the end of the last event.
    pub(super) fn consumed(&self) -> u64 {
        self.consumed
    }

    fn pending(&self) -> &[u8] {
        &self.buffer[self.start..]
    }

    fn need_more(&self) -> Result<Event, MultipartError> {
        if self.ended {
            return Err(malformed("the body ended before the closing boundary"));
        }
        Ok(Event::NeedMore)
    }

    /// Consumes the next `count` pending bytes.
    fn skip(&mut self, count: usize) {
        self.start += count;
        self.consumed += count as u64;
    }

    /// Consumes and returns the next `count` pending bytes.
    fn take(&mut self, count: usize) -> Vec<u8> {
        let data = self.pending()[..count].to_vec();
        self.skip(count);
        data
    }

    /// The next event; call again after [`Event::NeedMore`] once more input
    /// was pushed or the body finished.
    pub(super) fn next_event(&mut self) -> Result<Event, MultipartError> {
        loop {
            match self.state {
                State::Start => {
                    // The first boundary has no leading CRLF.
                    let first = &self.delimiter[2..];
                    let pending = self.pending();
                    let available = pending.len().min(first.len());
                    if pending[..available] != first[..available] {
                        return Err(malformed("the body does not start with the boundary"));
                    }
                    if available < first.len() {
                        return self.need_more();
                    }
                    let length = first.len();
                    self.skip(length);
                    self.state = State::AfterBoundary;
                }
                State::AfterBoundary => {
                    let Some(next) = self.pending().get(..2) else {
                        return self.need_more();
                    };
                    if next == b"--" {
                        self.skip(2);
                        self.state = State::Done;
                        return Ok(Event::End);
                    }
                    if next != b"\r\n" {
                        return Err(malformed("a boundary is followed by unexpected bytes"));
                    }
                    self.skip(2);
                    self.state = State::Headers;
                }
                State::Headers => return self.headers(),
                State::Body => return self.body(),
                State::Done => return Ok(Event::End),
            }
        }
    }

    fn headers(&mut self) -> Result<Event, MultipartError> {
        if self.pending().starts_with(b"\r\n") {
            // A part without headers; `headers::parse` rejects it.
            self.skip(2);
            self.state = State::Body;
            return Ok(Event::PartStart(Vec::new()));
        }
        let pending = self.pending();
        let Some(found) = find(&pending[self.searched..], b"\r\n\r\n") else {
            if pending.len() > MAX_PART_HEADER_BYTES + 3 {
                return Err(MultipartError::PartHeadersTooLarge);
            }
            // The end of the block may straddle the next chunk.
            self.searched = pending.len().saturating_sub(3);
            return self.need_more();
        };
        let end = self.searched + found;
        if end > MAX_PART_HEADER_BYTES {
            return Err(MultipartError::PartHeadersTooLarge);
        }
        self.searched = 0;
        let headers = self.take(end);
        self.skip(4);
        self.state = State::Body;
        Ok(Event::PartStart(headers))
    }

    fn body(&mut self) -> Result<Event, MultipartError> {
        let pending = self.pending();
        if let Some(found) = find(&pending[self.searched..], &self.delimiter) {
            let end = self.searched + found;
            self.searched = 0;
            let data = self.take(end);
            let length = self.delimiter.len();
            self.skip(length);
            self.state = State::AfterBoundary;
            return Ok(Event::PartEnd(data));
        }
        // The last bytes may start a delimiter that the next chunk ends.
        let keep = self.delimiter.len() - 1;
        let settled = pending.len().saturating_sub(keep);
        if settled >= DATA_EVENT_BYTES {
            self.searched = 0;
            return Ok(Event::Data(self.take(settled)));
        }
        self.searched = settled;
        self.need_more()
    }
}

/// First position of `needle` in `haystack`. Candidates are found by the
/// first byte and compared until the first mismatch; a delimiter's first
/// byte (CR) does not occur again in it, so this is linear.
fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    let (&first, rest) = needle.split_first()?;
    let mut start = 0;
    while let Some(offset) = haystack
        .get(start..)?
        .iter()
        .position(|&byte| byte == first)
    {
        let at = start + offset;
        let candidate = haystack.get(at + 1..at + needle.len())?;
        if candidate
            .iter()
            .zip(rest)
            .all(|(left, right)| left == right)
        {
            return Some(at);
        }
        start = at + 1;
    }
    None
}

#[cfg(test)]
#[path = "parser_tests.rs"]
mod tests;
