//! Revocation quota policy shared by the bundled JWT revocation stores.
//!
//! One global `max_entries` quota covers token (`jti`) rows and subject rows.
//! Token rows may use at most three quarters of it, and one subject at most
//! [`MAX_TOKEN_REVOCATIONS_PER_SUBJECT`] active token rows. When a new token
//! row would exceed either bound, or the store is full, `revoke_token` widens
//! instead: it records a subject cutoff that rejects every token of that
//! subject issued no later than the revoked token. The token is still revoked
//! (more broadly, which is safe), and one subject cannot exhaust the quota for
//! everyone else. The remaining quarter stays available for subject rows.

/// Active token revocations one subject may hold before revocation widens.
pub(super) const MAX_TOKEN_REVOCATIONS_PER_SUBJECT: usize = 64;

/// Token rows allowed for a quota; the rest is reserved for subject rows.
pub(super) const fn token_revocation_limit(max_entries: usize) -> usize {
    max_entries - max_entries.div_ceil(4)
}

/// Whether a new token row must widen to a subject cutoff instead.
pub(super) const fn must_widen(
    subject_tokens: usize,
    tokens: usize,
    subjects: usize,
    max_entries: usize,
) -> bool {
    subject_tokens >= MAX_TOKEN_REVOCATIONS_PER_SUBJECT
        || tokens >= token_revocation_limit(max_entries)
        || tokens.saturating_add(subjects) >= max_entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_rows_leave_a_subject_reserve() {
        assert_eq!(token_revocation_limit(1), 0);
        assert_eq!(token_revocation_limit(4), 3);
        assert_eq!(token_revocation_limit(16), 12);
        assert_eq!(token_revocation_limit(1_000_000), 750_000);
        assert!(!must_widen(0, 0, 0, 16));
        assert!(must_widen(MAX_TOKEN_REVOCATIONS_PER_SUBJECT, 0, 0, 1_000));
        assert!(must_widen(0, 12, 0, 16));
        assert!(must_widen(0, 2, 14, 16));
        assert!(!must_widen(
            MAX_TOKEN_REVOCATIONS_PER_SUBJECT - 1,
            11,
            4,
            16
        ));
    }
}
