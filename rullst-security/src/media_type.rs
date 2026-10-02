//! `Content-Type` classification for the RASP, schema-guard, DLP and AI
//! firewall layers.
//!
//! Request classification accepts at least every spelling that axum's `Json`
//! and `Form` extractors accept, so a body a handler parses cannot skip
//! inspection through letter case, parameters or a structured-syntax suffix.

/// Media type of a `Content-Type` value: the text before the first `;`,
/// trimmed. The classifiers below ignore ASCII case, as the `mime` parser does.
pub(crate) fn essence(content_type: &str) -> &str {
    content_type.split(';').next().unwrap_or_default().trim()
}

fn application_subtype(media_type: &str) -> Option<&str> {
    const APPLICATION: &[u8] = b"application/";
    let prefix = media_type.as_bytes().get(..APPLICATION.len())?;
    if !prefix.eq_ignore_ascii_case(APPLICATION) {
        return None;
    }
    media_type.get(APPLICATION.len()..)
}

/// Whether an `application/` subtype is `syntax` itself or uses it on either
/// side of its last `+`. axum's `Json` accepts subtype `json` and suffix
/// `json` after parsing with `mime`, whose subtype ends at the last `+`.
fn has_structured_syntax(media_type: &str, syntax: &str) -> bool {
    let Some(subtype) = application_subtype(media_type) else {
        return false;
    };
    match subtype.rfind('+') {
        Some(plus) => {
            subtype[..plus].eq_ignore_ascii_case(syntax)
                || subtype[plus + 1..].eq_ignore_ascii_case(syntax)
        }
        None => subtype.eq_ignore_ascii_case(syntax),
    }
}

/// `application/json`, `application/json+…` or `application/…+json`.
pub(crate) fn is_json(media_type: &str) -> bool {
    has_structured_syntax(media_type, "json")
}

/// `application/xml`, `application/xml+…` or `application/…+xml`.
pub(crate) fn is_xml(media_type: &str) -> bool {
    has_structured_syntax(media_type, "xml")
}

/// Any media type beginning with `application/x-www-form-urlencoded`: axum's
/// `Form` accepts every `Content-Type` with that prefix.
pub(crate) fn is_form(media_type: &str) -> bool {
    const FORM: &[u8] = b"application/x-www-form-urlencoded";
    media_type
        .as_bytes()
        .get(..FORM.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(FORM))
}

/// Any `text/*` media type.
pub(crate) fn is_text(media_type: &str) -> bool {
    media_type
        .as_bytes()
        .get(..5)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(b"text/"))
}

/// Whether a request body with this media type is inspected: text, JSON,
/// XML and URL-encoded forms, in every spelling the extractors accept.
pub(crate) fn is_inspected_request_body(media_type: &str) -> bool {
    is_text(media_type) || is_json(media_type) || is_xml(media_type) || is_form(media_type)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classification_covers_every_form_the_extractors_accept() {
        for json in [
            "application/json",
            "APPLICATION/JSON",
            "application/vnd.api+JSON",
            "Application/problem+json",
            "application/json+patch",
            "application/a+b+json",
        ] {
            assert!(is_json(json), "{json}");
            assert!(is_inspected_request_body(json), "{json}");
        }
        for form in [
            "application/x-www-form-urlencoded",
            "APPLICATION/X-WWW-FORM-URLENCODED",
            "application/x-www-form-urlencodedX",
        ] {
            assert!(is_form(form), "{form}");
            assert!(is_inspected_request_body(form), "{form}");
        }
        for other in ["Text/Plain", "application/XML", "Application/atom+Xml"] {
            assert!(is_inspected_request_body(other), "{other}");
        }
        for skipped in [
            "application/octet-stream",
            "application/jsonp",
            "application/json-seq",
            "text-json",
            "image/svg",
            "multipart/form-data",
            "",
            "application",
            "application/",
        ] {
            assert!(!is_inspected_request_body(skipped), "{skipped}");
        }
        assert_eq!(
            essence(" application/json ; charset=utf-8"),
            "application/json"
        );
        assert!(!is_json("application/json-seq"));
        assert!(!is_json("text/json"));
    }
}
