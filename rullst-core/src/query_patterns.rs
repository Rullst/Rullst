//! Shared, pure detection of repeated database operations (possible N+1
//! queries) used by the development dashboard and Studio.
//!
//! The rule is a heuristic: the same operation fingerprint appearing at least
//! [`N_PLUS_ONE_THRESHOLD`] times within one request or trace. Repetition can
//! be intentional, so a finding is evidence to inspect, not proof of an N+1
//! defect. Unpublished v13 API.

use std::collections::HashMap;

/// Repetitions of one operation within one request or trace at which a
/// possible N+1 pattern is reported.
pub const N_PLUS_ONE_THRESHOLD: usize = 3;

/// Longest fingerprint returned by [`orm_fingerprint`], in characters.
pub const MAX_FINGERPRINT_CHARS: usize = 96;

/// One operation fingerprint repeated within a single request or trace.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct RepeatedOperation {
    /// The fingerprint that repeated.
    pub fingerprint: String,
    /// How many times it occurred.
    pub occurrences: usize,
}

/// Groups the fingerprints observed in one request or trace and returns those
/// occurring at least `threshold` times (never fewer than two), most frequent
/// first, then by fingerprint.
pub fn repeated_operations<I, S>(fingerprints: I, threshold: usize) -> Vec<RepeatedOperation>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let threshold = threshold.max(2);
    let mut counts: HashMap<String, usize> = HashMap::new();
    for fingerprint in fingerprints {
        let count = counts.entry(fingerprint.as_ref().to_string()).or_default();
        *count = count.saturating_add(1);
    }
    let mut repeated = counts
        .into_iter()
        .filter(|(_, occurrences)| *occurrences >= threshold)
        .map(|(fingerprint, occurrences)| RepeatedOperation {
            fingerprint,
            occurrences,
        })
        .collect::<Vec<_>>();
    repeated.sort_by(|left, right| {
        right
            .occurrences
            .cmp(&left.occurrences)
            .then_with(|| left.fingerprint.cmp(&right.fingerprint))
    });
    repeated
}

/// Fingerprint of an ORM operation from its static span labels, such as
/// `Post.find (posts)`.
///
/// ORM spans never carry SQL text or bindings, so the labels are the
/// statement's identity: literal values cannot appear in the fingerprint.
/// Returns `None` without a model or table label (for example `Orm::raw`
/// statements), whose repetitions could be different statements. Control
/// characters are replaced and the result is cut to
/// [`MAX_FINGERPRINT_CHARS`].
pub fn orm_fingerprint(
    operation: &str,
    model: Option<&str>,
    table: Option<&str>,
) -> Option<String> {
    let fingerprint = match (model, table) {
        (Some(model), Some(table)) => format!("{model}.{operation} ({table})"),
        (Some(model), None) => format!("{model}.{operation}"),
        (None, Some(table)) => format!("{table}.{operation}"),
        (None, None) => return None,
    };
    Some(
        fingerprint
            .chars()
            .map(|character| {
                if character.is_control() {
                    '?'
                } else {
                    character
                }
            })
            .take(MAX_FINGERPRINT_CHARS)
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn repetitions_at_the_threshold_are_reported_most_frequent_first() {
        let observed = [
            "Post.find (posts)",
            "User.find (users)",
            "Post.find (posts)",
            "Comment.all (comments)",
            "Post.find (posts)",
            "User.find (users)",
            "User.find (users)",
            "User.find (users)",
        ];
        assert_eq!(
            repeated_operations(observed, N_PLUS_ONE_THRESHOLD),
            vec![
                RepeatedOperation {
                    fingerprint: "User.find (users)".to_string(),
                    occurrences: 4,
                },
                RepeatedOperation {
                    fingerprint: "Post.find (posts)".to_string(),
                    occurrences: 3,
                },
            ]
        );
    }

    #[test]
    fn below_the_threshold_or_distinct_operations_report_nothing() {
        assert!(repeated_operations(["a", "a", "b", "c"], N_PLUS_ONE_THRESHOLD).is_empty());
        assert!(repeated_operations(Vec::<&str>::new(), N_PLUS_ONE_THRESHOLD).is_empty());
        // A threshold below two would flag every single operation.
        assert!(repeated_operations(["a", "b"], 0).is_empty());
        assert_eq!(repeated_operations(["a", "a"], 1).len(), 1);
    }

    #[test]
    fn orm_fingerprints_use_labels_and_skip_unlabelled_statements() {
        assert_eq!(
            orm_fingerprint("find", Some("Post"), Some("posts")).as_deref(),
            Some("Post.find (posts)")
        );
        assert_eq!(
            orm_fingerprint("select", None, Some("posts")).as_deref(),
            Some("posts.select")
        );
        assert_eq!(
            orm_fingerprint("find", Some("Post"), None).as_deref(),
            Some("Post.find")
        );
        assert_eq!(orm_fingerprint("raw.select", None, None), None);
        let long = "m".repeat(200);
        let cut = orm_fingerprint("find\u{1b}", Some(&long), None).unwrap();
        assert_eq!(cut.chars().count(), MAX_FINGERPRINT_CHARS);
        assert!(
            !orm_fingerprint("find\u{1b}", Some("Post"), None)
                .unwrap()
                .contains('\u{1b}')
        );
    }
}
