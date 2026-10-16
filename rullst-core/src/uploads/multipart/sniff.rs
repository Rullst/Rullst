//! Streaming recognition of an uploaded file's content.
//!
//! It reaches the verdict of `UploadPolicy`'s whole-object check while the
//! file streams past: the binary signatures need only the first bytes, and
//! the plain-text rules are checked character by character across chunks.

use super::MultipartPolicy;
use crate::uploads::{
    ACTIVE_SCHEME, BINARY_SIGNATURE_BYTES, UploadKind, detect_binary_kind,
    is_denied_text_character, is_insignificant_lead, opens_markup,
};

/// Bytes kept from the start of a file: every binary signature and every
/// expected signature fits.
const HEAD_BYTES: usize = MultipartPolicy::MAX_SIGNATURE_BYTES;

/// Content recognition of one file.
#[derive(Default)]
pub(super) struct Sniffer {
    head: Vec<u8>,
    text: TextScan,
}

impl Sniffer {
    pub(super) fn update(&mut self, data: &[u8]) {
        let room = HEAD_BYTES.saturating_sub(self.head.len());
        self.head.extend_from_slice(&data[..data.len().min(room)]);
        self.text.update(data);
    }

    /// The kind of the complete content.
    pub(super) fn kind(&self) -> Option<UploadKind> {
        detect_binary_kind(&self.head)
            .or_else(|| self.text.is_plain_at_end().then_some(UploadKind::PlainText))
    }

    /// Whether `policy`'s content checks admit the content read so far:
    /// `None` while that depends on bytes not read yet, which only happens
    /// before the content has `ended`.
    pub(super) fn admits(&self, policy: &MultipartPolicy, ended: bool) -> Option<bool> {
        match (
            self.signature_admits(&policy.signatures, ended),
            self.kind_admits(&policy.allowed_kinds, ended),
        ) {
            (Some(false), _) | (_, Some(false)) => Some(false),
            (Some(true), Some(true)) => Some(true),
            _ => None,
        }
    }

    fn signature_admits(&self, signatures: &[Vec<u8>], ended: bool) -> Option<bool> {
        if signatures.is_empty()
            || signatures
                .iter()
                .any(|signature| self.head.starts_with(signature))
        {
            return Some(true);
        }
        // A signature that the bytes read so far start may still match. One
        // no longer than them would already have matched above.
        let pending = !ended
            && signatures
                .iter()
                .any(|signature| signature.starts_with(&self.head));
        if pending { None } else { Some(false) }
    }

    fn kind_admits(&self, allowed: &[UploadKind], ended: bool) -> Option<bool> {
        if allowed.is_empty() {
            return Some(true);
        }
        if !ended && self.head.len() < BINARY_SIGNATURE_BYTES {
            return None;
        }
        if let Some(kind) = detect_binary_kind(&self.head) {
            return Some(allowed.contains(&kind));
        }
        if !allowed.contains(&UploadKind::PlainText) || self.text.denied {
            return Some(false);
        }
        if ended {
            Some(self.text.is_plain_at_end())
        } else {
            None
        }
    }
}

/// Where the first significant character stands.
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
enum Lead {
    /// Only insignificant characters so far.
    #[default]
    Skipping,
    /// The first significant character was `<`.
    AfterLt,
    /// The text does not start with markup.
    Done,
}

/// Incremental form of the plain-text rules: valid UTF-8, no NUL or denied
/// control character, no leading markup, no `javascript:` in any ASCII case.
#[derive(Default, Debug)]
struct TextScan {
    denied: bool,
    /// The start of a character split across chunks (at most 3 bytes).
    carry: Vec<u8>,
    lead: Lead,
    /// How many bytes of [`ACTIVE_SCHEME`] the text currently ends with.
    scheme_matched: usize,
}

impl TextScan {
    fn is_plain_at_end(&self) -> bool {
        !self.denied && self.carry.is_empty()
    }

    fn update(&mut self, mut data: &[u8]) {
        if self.denied {
            return;
        }
        if let Some(&first) = self.carry.first() {
            let width = utf8_width(first);
            let take = width.saturating_sub(self.carry.len()).min(data.len());
            self.carry.extend_from_slice(&data[..take]);
            data = &data[take..];
            if self.carry.len() < width {
                return;
            }
            let carry = std::mem::take(&mut self.carry);
            match std::str::from_utf8(&carry) {
                Ok(text) => self.scan(text),
                Err(_) => self.denied = true,
            }
            if self.denied {
                return;
            }
        }
        match std::str::from_utf8(data) {
            Ok(text) => self.scan(text),
            Err(error) => {
                let (valid, rest) = data.split_at(error.valid_up_to());
                if let Ok(text) = std::str::from_utf8(valid) {
                    self.scan(text);
                }
                if error.error_len().is_some() {
                    self.denied = true;
                } else if !self.denied {
                    self.carry = rest.to_vec();
                }
            }
        }
    }

    fn scan(&mut self, text: &str) {
        for character in text.chars() {
            if is_denied_text_character(character) {
                self.denied = true;
                return;
            }
            match self.lead {
                Lead::Skipping if is_insignificant_lead(character) => {}
                Lead::Skipping if character == '<' => self.lead = Lead::AfterLt,
                Lead::AfterLt if opens_markup(character) => {
                    self.denied = true;
                    return;
                }
                Lead::Skipping | Lead::AfterLt => self.lead = Lead::Done,
                Lead::Done => {}
            }
            if self.matches_scheme(character) {
                self.denied = true;
                return;
            }
        }
    }

    /// Advances the `javascript:` match; `true` once it is complete. No
    /// proper prefix of the scheme is also its suffix, so a mismatch
    /// restarts the match at the current character.
    fn matches_scheme(&mut self, character: char) -> bool {
        let scheme = ACTIVE_SCHEME.as_bytes();
        let lowered = character.to_ascii_lowercase();
        let continues = scheme
            .get(self.scheme_matched)
            .is_some_and(|&expected| lowered == char::from(expected));
        self.scheme_matched = if continues {
            self.scheme_matched + 1
        } else {
            usize::from(lowered == char::from(scheme[0]))
        };
        self.scheme_matched == scheme.len()
    }
}

/// Encoded length of the UTF-8 character starting with the valid lead byte
/// `first` of an incomplete sequence.
fn utf8_width(first: u8) -> usize {
    match first {
        0xc0..=0xdf => 2,
        0xe0..=0xef => 3,
        _ => 4,
    }
}

#[cfg(test)]
#[path = "sniff_tests.rs"]
mod tests;
