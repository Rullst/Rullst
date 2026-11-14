//! What a browser's private cache does with a response on the next visit,
//! from the response's own headers (RFC 9111): reuse it without a request
//! while it is fresh, revalidate it when it carries a validator, or fetch it
//! again. Only explicit freshness counts (`Cache-Control: max-age`, else
//! `Expires` against `Date`); the heuristic freshness some browsers apply to
//! responses with only `Last-Modified` is not, so repeat-visit figures never
//! assume more caching than the app asked for.

/// The headers the decision reads.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct CacheHeaders {
    pub cache_control: Option<String>,
    pub expires: Option<String>,
    pub date: Option<String>,
    pub age: Option<String>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}

/// The validators of a conditional request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Validators {
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}

/// How the next visit gets the response.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Reuse {
    /// Fresh for this many more seconds: no request.
    Fresh(u64),
    /// A conditional request; a `304 Not Modified` answer has no body.
    Revalidate(Validators),
    /// A full request again: `no-store`, or stale without a validator.
    Refetch,
}

/// The value of directive `name` in a `Cache-Control` list: `Some("")` for
/// a directive without a value.
fn directive<'a>(cache_control: &'a str, name: &str) -> Option<&'a str> {
    cache_control.split(',').find_map(|entry| {
        let (key, value) = entry.split_once('=').unwrap_or((entry, ""));
        key.trim()
            .eq_ignore_ascii_case(name)
            .then(|| value.trim().trim_matches('"'))
    })
}

fn seconds(value: &str) -> Option<u64> {
    value.trim().parse().ok()
}

fn http_date(value: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc2822(value.trim())
        .ok()
        .map(|date| date.timestamp())
}

/// The freshness lifetime the response states, in seconds.
fn lifetime(headers: &CacheHeaders) -> Option<u64> {
    let cache_control = headers.cache_control.as_deref().unwrap_or_default();
    if let Some(max_age) = directive(cache_control, "max-age") {
        // An invalid max-age makes the response stale (RFC 9111 §4.2.1).
        return Some(seconds(max_age).unwrap_or(0));
    }
    let expires = headers.expires.as_deref()?;
    let Some(expires) = http_date(expires) else {
        // An invalid Expires, such as "0", means already expired.
        return Some(0);
    };
    let date = headers.date.as_deref().and_then(http_date)?;
    Some(u64::try_from(expires.saturating_sub(date)).unwrap_or(0))
}

/// What the next visit, `elapsed_seconds` after the response arrived, does.
pub(crate) fn reuse(headers: &CacheHeaders, elapsed_seconds: u64) -> Reuse {
    let cache_control = headers.cache_control.as_deref().unwrap_or_default();
    if directive(cache_control, "no-store").is_some() {
        return Reuse::Refetch;
    }
    let age = headers
        .age
        .as_deref()
        .and_then(seconds)
        .unwrap_or(0)
        .saturating_add(elapsed_seconds);
    let must_revalidate = directive(cache_control, "no-cache").is_some();
    if !must_revalidate
        && let Some(lifetime) = lifetime(headers)
        && lifetime > age
    {
        return Reuse::Fresh(lifetime - age);
    }
    if headers.etag.is_some() || headers.last_modified.is_some() {
        return Reuse::Revalidate(Validators {
            etag: headers.etag.clone(),
            last_modified: headers.last_modified.clone(),
        });
    }
    Reuse::Refetch
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(cache_control: &str) -> CacheHeaders {
        CacheHeaders {
            cache_control: Some(cache_control.to_string()),
            ..CacheHeaders::default()
        }
    }

    fn validated(cache_control: Option<&str>) -> CacheHeaders {
        CacheHeaders {
            cache_control: cache_control.map(str::to_string),
            etag: Some("\"v1\"".to_string()),
            ..CacheHeaders::default()
        }
    }

    #[test]
    fn max_age_keeps_a_response_fresh_until_it_runs_out() {
        let immutable = headers("public, max-age=31536000, immutable");
        assert_eq!(reuse(&immutable, 2), Reuse::Fresh(31_535_998));
        assert_eq!(reuse(&headers("MAX-AGE = 60"), 0), Reuse::Fresh(60));
        assert_eq!(reuse(&headers("max-age=\"60\""), 59), Reuse::Fresh(1));
        assert_eq!(reuse(&headers("max-age=60"), 60), Reuse::Refetch);
        assert_eq!(reuse(&headers("max-age=60"), 61), Reuse::Refetch);
        let aged = CacheHeaders {
            age: Some("50".to_string()),
            ..headers("max-age=60")
        };
        assert_eq!(reuse(&aged, 5), Reuse::Fresh(5));
        assert_eq!(reuse(&aged, 10), Reuse::Refetch);
        let bad_age = CacheHeaders {
            age: Some("soon".to_string()),
            ..headers("max-age=60")
        };
        assert_eq!(reuse(&bad_age, 0), Reuse::Fresh(60));
        // An invalid max-age is stale; max-age wins over Expires.
        assert_eq!(reuse(&headers("max-age=forever"), 0), Reuse::Refetch);
        let both = CacheHeaders {
            expires: Some("Thu, 01 Jan 2099 00:00:00 GMT".to_string()),
            date: Some("Thu, 01 Jan 2026 00:00:00 GMT".to_string()),
            ..headers("max-age=0")
        };
        assert_eq!(reuse(&both, 0), Reuse::Refetch);
    }

    #[test]
    fn no_store_no_cache_and_validators_decide_stale_responses() {
        assert_eq!(reuse(&headers("no-store, max-age=600"), 0), Reuse::Refetch);
        let revalidate = Reuse::Revalidate(Validators {
            etag: Some("\"v1\"".to_string()),
            last_modified: None,
        });
        assert_eq!(
            reuse(&validated(Some("no-cache, max-age=600")), 0),
            revalidate
        );
        assert_eq!(reuse(&validated(Some("max-age=0")), 0), revalidate);
        assert_eq!(reuse(&validated(None), 0), revalidate);
        assert_eq!(reuse(&validated(Some("max-age=600")), 0), Reuse::Fresh(600));
        let modified = CacheHeaders {
            last_modified: Some("Thu, 01 Jan 2026 00:00:00 GMT".to_string()),
            ..CacheHeaders::default()
        };
        assert_eq!(
            reuse(&modified, 0),
            Reuse::Revalidate(Validators {
                etag: None,
                last_modified: Some("Thu, 01 Jan 2026 00:00:00 GMT".to_string()),
            })
        );
        assert_eq!(reuse(&CacheHeaders::default(), 0), Reuse::Refetch);
        assert_eq!(reuse(&headers("private"), 0), Reuse::Refetch);
    }

    #[test]
    fn expires_counts_from_the_response_date() {
        let expires = |expires: &str, date: Option<&str>| CacheHeaders {
            expires: Some(expires.to_string()),
            date: date.map(str::to_string),
            ..CacheHeaders::default()
        };
        let date = Some("Thu, 01 Jan 2026 00:00:00 GMT");
        assert_eq!(
            reuse(&expires("Thu, 01 Jan 2026 00:01:40 GMT", date), 10),
            Reuse::Fresh(90)
        );
        assert_eq!(
            reuse(&expires("Wed, 31 Dec 2025 00:00:00 GMT", date), 0),
            Reuse::Refetch
        );
        assert_eq!(reuse(&expires("0", date), 0), Reuse::Refetch);
        // Without a Date there is no lifetime to count from.
        assert_eq!(
            reuse(&expires("Thu, 01 Jan 2026 00:01:40 GMT", None), 0),
            Reuse::Refetch
        );
        assert_eq!(directive("a, b=1", "b"), Some("1"));
        assert_eq!(directive("a, b=1", "a"), Some(""));
        assert_eq!(directive("a, b=1", "c"), None);
    }
}
