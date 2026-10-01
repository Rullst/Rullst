//! Bounded projections of queue records for monitoring dashboards.

use super::QueuedJobDetail;

/// One queue record whose payload and error are cut to a byte budget, as
/// returned by [`super::QueueDriver::list_job_previews`].
///
/// `payload` and `error` hold at most the requested number of bytes, cut on a
/// UTF-8 character boundary; invalid UTF-8 read from the store is replaced
/// with U+FFFD within the same budget. `id`, `name`, `status` and the
/// timestamps are not cut ([`super::Queue::dispatch`] bounds job names to 256
/// bytes). Unpublished v13 API.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct QueuedJobPreview {
    /// Unique identifier of the queued job.
    pub id: String,
    /// The name/type of the job.
    pub name: String,
    /// The leading bytes of the job's JSON payload.
    pub payload: String,
    /// Whether `payload` is shorter than the stored payload.
    pub payload_truncated: bool,
    /// Current status, as in [`QueuedJobDetail::status`].
    pub status: String,
    /// The leading bytes of the recorded error, if any.
    pub error: Option<String>,
    /// Whether `error` is shorter than the stored error.
    pub error_truncated: bool,
    /// Number of processing attempts made so far.
    pub attempts: i32,
    /// Time when the job was created, as in [`QueuedJobDetail::created_at`].
    pub created_at: String,
    /// Time of the last status change, as in [`QueuedJobDetail::updated_at`].
    pub updated_at: String,
}

impl QueuedJobPreview {
    /// Projects a complete record, keeping at most `max_field_bytes` bytes of
    /// its payload and of its error.
    pub fn from_detail(detail: QueuedJobDetail, max_field_bytes: u32) -> Self {
        let (payload, payload_truncated) = field_prefix(detail.payload.as_bytes(), max_field_bytes);
        let (error, error_truncated) =
            optional_field_prefix(detail.error.as_deref().map(str::as_bytes), max_field_bytes);
        Self {
            id: detail.id,
            name: detail.name,
            payload,
            payload_truncated,
            status: detail.status,
            error,
            error_truncated,
            attempts: detail.attempts,
            created_at: detail.created_at,
            updated_at: detail.updated_at,
        }
    }
}

/// The number of leading bytes a driver reads from its store to cut a field
/// to `max_field_bytes`: one more, so that a longer stored value shows.
#[cfg(any(feature = "queue-sqlite", feature = "queue-redis"))]
pub(crate) fn stored_prefix_bytes(max_field_bytes: u32) -> i64 {
    i64::from(max_field_bytes) + 1
}

/// Cuts stored field bytes to `max_field_bytes` with [`utf8_prefix`].
pub(crate) fn field_prefix(bytes: &[u8], max_field_bytes: u32) -> (String, bool) {
    utf8_prefix(
        bytes,
        usize::try_from(max_field_bytes).unwrap_or(usize::MAX),
    )
}

/// [`field_prefix`] for an optional field; an absent value is not truncated.
pub(crate) fn optional_field_prefix(
    bytes: Option<&[u8]>,
    max_field_bytes: u32,
) -> (Option<String>, bool) {
    match bytes {
        Some(bytes) => {
            let (text, truncated) = field_prefix(bytes, max_field_bytes);
            (Some(text), truncated)
        }
        None => (None, false),
    }
}

/// Decodes at most `budget` leading bytes of `bytes` as UTF-8 text.
///
/// A multi-byte character cut by the budget (or by a store that returned
/// only a prefix) is dropped rather than replaced, and other invalid bytes
/// become U+FFFD while the result still fits the budget. The flag reports
/// whether anything was left out.
pub(crate) fn utf8_prefix(bytes: &[u8], budget: usize) -> (String, bool) {
    let truncated = bytes.len() > budget;
    let head = &bytes[..bytes.len().min(budget)];
    let mut text = String::with_capacity(head.len());
    let mut chunks = head.utf8_chunks().peekable();
    while let Some(chunk) = chunks.next() {
        // `text` never exceeds the budget, so the room cannot underflow.
        let room = budget - text.len();
        let valid = chunk.valid();
        if valid.len() > room {
            text.push_str(&valid[..valid.floor_char_boundary(room)]);
            return (text, true);
        }
        text.push_str(valid);
        let invalid = chunk.invalid();
        if invalid.is_empty() {
            continue;
        }
        let incomplete_tail = chunks.peek().is_none()
            && std::str::from_utf8(invalid)
                .err()
                .is_some_and(|error| error.error_len().is_none());
        if truncated && incomplete_tail {
            // The cut split a character: drop its leading bytes.
            break;
        }
        if text.len() + char::REPLACEMENT_CHARACTER.len_utf8() > budget {
            return (text, true);
        }
        text.push(char::REPLACEMENT_CHARACTER);
    }
    (text, truncated)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn prefixes_respect_the_budget_and_character_boundaries() {
        assert_eq!(utf8_prefix(b"abc", 3), ("abc".to_string(), false));
        assert_eq!(utf8_prefix(b"abcd", 3), ("abc".to_string(), true));
        assert_eq!(utf8_prefix("ééé".as_bytes(), 3), ("é".to_string(), true));
        assert_eq!(utf8_prefix("ééé".as_bytes(), 4), ("éé".to_string(), true));
        assert_eq!(utf8_prefix(b"", 0), (String::new(), false));
        assert_eq!(utf8_prefix(b"a", 0), (String::new(), true));
        // Invalid stored bytes are replaced, never allowed past the budget.
        assert_eq!(utf8_prefix(b"a\xffb", 8), ("a\u{fffd}b".to_string(), false));
        assert_eq!(utf8_prefix(b"a\xffb", 3), ("a".to_string(), true));
        assert_eq!(utf8_prefix(b"a\xffbc", 4), ("a\u{fffd}".to_string(), true));
        assert_eq!(
            utf8_prefix(b"a\xff\xc3\xa9", 6),
            ("a\u{fffd}é".to_string(), false)
        );
        assert_eq!(
            utf8_prefix(b"a\xff\xc3\xa9", 5),
            ("a\u{fffd}".to_string(), true)
        );
        // An incomplete trailing character that is the whole stored value
        // is invalid data, not a cut.
        assert_eq!(utf8_prefix(b"ab\xc3", 8), ("ab\u{fffd}".to_string(), false));
    }

    #[test]
    fn previews_cut_large_payloads_and_errors() {
        let payload = format!("{{\"blob\":\"{}\"}}", "é".repeat(400_000));
        let detail = QueuedJobDetail {
            id: "job-1".to_string(),
            name: "report".to_string(),
            payload,
            status: "failed".to_string(),
            error: Some("x".repeat(10_000)),
            attempts: 2,
            created_at: "created".to_string(),
            updated_at: "updated".to_string(),
        };
        let preview = QueuedJobPreview::from_detail(detail, 64);
        assert!(preview.payload.len() <= 64 && preview.payload.starts_with("{\"blob\":\"é"));
        assert!(preview.payload_truncated);
        assert_eq!(preview.error.as_deref(), Some("x".repeat(64).as_str()));
        assert!(preview.error_truncated);
        assert_eq!(
            (preview.id.as_str(), preview.name.as_str(), preview.attempts),
            ("job-1", "report", 2)
        );

        let short = QueuedJobPreview::from_detail(
            QueuedJobDetail {
                id: "job-2".to_string(),
                name: "report".to_string(),
                payload: "{}".to_string(),
                status: "pending".to_string(),
                error: None,
                attempts: 0,
                created_at: String::new(),
                updated_at: String::new(),
            },
            64,
        );
        assert_eq!(short.payload, "{}");
        assert!(!short.payload_truncated && short.error.is_none() && !short.error_truncated);
    }

    /// A custom driver that only lists complete records.
    struct ListingDriver;

    #[async_trait::async_trait]
    impl crate::queue::QueueDriver for ListingDriver {
        async fn push(&self, _: &str, _: &str, _: &str) -> Result<(), crate::queue::QueueError> {
            Ok(())
        }
        async fn pop(&self) -> Result<Option<crate::queue::QueuedJob>, crate::queue::QueueError> {
            Ok(None)
        }
        async fn mark_complete(&self, _: &str) -> Result<(), crate::queue::QueueError> {
            Ok(())
        }
        async fn mark_failed(&self, _: &str, _: &str) -> Result<(), crate::queue::QueueError> {
            Ok(())
        }
        async fn pending_count(&self) -> Result<u64, crate::queue::QueueError> {
            Ok(0)
        }
        async fn list_all_jobs(
            &self,
            limit: u32,
        ) -> Result<Vec<QueuedJobDetail>, crate::queue::QueueError> {
            let job = QueuedJobDetail {
                id: "custom".to_string(),
                name: "report".to_string(),
                payload: "p".repeat(1_000),
                status: "failed".to_string(),
                error: Some("e".repeat(1_000)),
                attempts: 1,
                created_at: String::new(),
                updated_at: String::new(),
            };
            Ok(vec![job; usize::try_from(limit).unwrap_or(0).min(2)])
        }
    }

    #[tokio::test]
    async fn custom_drivers_inherit_a_projected_listing() {
        let queue = crate::queue::Queue::custom(Box::new(ListingDriver));
        let previews = queue.list_job_previews(5, 10).await.unwrap();
        assert_eq!(previews.len(), 2);
        assert!(previews.iter().all(|job| {
            job.payload == "p".repeat(10)
                && job.payload_truncated
                && job.error.as_deref() == Some("eeeeeeeeee")
                && job.error_truncated
        }));
    }
}
