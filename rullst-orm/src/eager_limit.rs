//! Row-cap guard for generated eager-load queries.
//!
//! One eager-load query fetches the related rows of every parent in the batch.
//! If the global row cap truncated that query, some parents would silently
//! receive empty or partial relations. Generated code therefore fetches one
//! row beyond the cap and fails closed instead.

/// Prepares one generated eager-load query and returns the cap to verify.
///
/// Only a query that still carries the global cap is guarded: it is widened by
/// one row so truncation becomes observable. A smaller explicit `limit(...)`
/// or `unsafe_unlimited()` chosen in a constrained eager load is an explicit
/// caller decision and is left unchanged.
pub fn guard(limit: &mut Option<usize>) -> Option<usize> {
    guard_with_cap(limit, crate::schema::get_max_query_limit())
}

fn guard_with_cap(limit: &mut Option<usize>, cap: Option<usize>) -> Option<usize> {
    let cap = cap?;
    if *limit != Some(cap) {
        return None;
    }
    *limit = Some(cap.saturating_add(1));
    Some(cap)
}

/// Fails instead of assigning a relation from a truncated eager-load query.
pub fn ensure_complete(
    loaded: usize,
    cap: Option<usize>,
    model: &str,
    relation: &str,
) -> Result<(), crate::Error> {
    match cap {
        Some(cap) if loaded > cap => Err(crate::Error::Validation(format!(
            "eager loading `{relation}` for `{model}` matched more than the {cap}-row query limit; \
             load fewer parents per query, constrain the relation with \
             `with_{relation}_constrained(...)`, or raise `Orm::set_max_query_limit`"
        ))),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::{ensure_complete, guard_with_cap};

    #[test]
    fn only_the_default_cap_is_widened_and_checked() {
        let mut default_limit = Some(1000);
        assert_eq!(guard_with_cap(&mut default_limit, Some(1000)), Some(1000));
        assert_eq!(default_limit, Some(1001));

        let mut explicit_limit = Some(10);
        assert_eq!(guard_with_cap(&mut explicit_limit, Some(1000)), None);
        assert_eq!(explicit_limit, Some(10));

        let mut unlimited = None;
        assert_eq!(guard_with_cap(&mut unlimited, Some(1000)), None);
        assert_eq!(unlimited, None);

        let mut disabled_cap = Some(1000);
        assert_eq!(guard_with_cap(&mut disabled_cap, None), None);
        assert_eq!(disabled_cap, Some(1000));

        let mut saturated = Some(usize::MAX);
        assert_eq!(
            guard_with_cap(&mut saturated, Some(usize::MAX)),
            Some(usize::MAX)
        );
        assert_eq!(saturated, Some(usize::MAX));
    }

    #[test]
    fn truncation_beyond_the_cap_is_a_validation_error() {
        assert!(ensure_complete(3, Some(3), "Post", "comments").is_ok());
        assert!(ensure_complete(4, None, "Post", "comments").is_ok());
        let error = ensure_complete(4, Some(3), "Post", "comments")
            .expect_err("a truncated eager load must fail");
        assert!(matches!(error, crate::Error::Validation(_)));
        let message = error.to_string();
        assert!(message.contains("`comments` for `Post`"), "{message}");
        assert!(message.contains("3-row query limit"), "{message}");
    }
}
