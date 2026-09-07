//! Personally Identifiable Information (PII) masking algorithms & middleware.

use axum::{
    body::HttpBody,
    extract::Request,
    http::{
        HeaderMap, HeaderValue, Method, StatusCode,
        header::{self, HeaderName},
    },
    middleware::Next,
    response::Response,
};

const MAX_BUFFERED_RESPONSE_BYTES: u64 = 2 * 1024 * 1024;

pub(super) const fn card_mask_count(digit_count: usize) -> Option<usize> {
    if digit_count >= 13 && digit_count <= 19 {
        Some(digit_count - 4)
    } else {
        None
    }
}

fn is_textual_response(headers: &HeaderMap) -> bool {
    let Some(media_type) = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .map(str::trim)
    else {
        return false;
    };

    if media_type.eq_ignore_ascii_case("text/event-stream") {
        return false;
    }

    media_type
        .get(..5)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("text/"))
        || media_type.eq_ignore_ascii_case("application/json")
        || media_type.eq_ignore_ascii_case("application/javascript")
        || media_type.eq_ignore_ascii_case("application/xml")
        || media_type
            .strip_suffix("+json")
            .is_some_and(|prefix| prefix.starts_with("application/"))
        || media_type
            .strip_suffix("+xml")
            .is_some_and(|prefix| prefix.starts_with("application/"))
}

fn has_identity_encoding(headers: &HeaderMap) -> bool {
    headers
        .get(header::CONTENT_ENCODING)
        .is_none_or(|value| value.as_bytes().eq_ignore_ascii_case(b"identity"))
}

fn is_safely_bufferable(headers: &HeaderMap, body: &axum::body::Body) -> bool {
    let hint = body.size_hint();
    let Some(upper) = hint.upper() else {
        return false;
    };

    if hint.lower() != upper || upper > MAX_BUFFERED_RESPONSE_BYTES {
        return false;
    }

    match headers.get(header::CONTENT_LENGTH) {
        Some(value) => value
            .to_str()
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .is_some_and(|declared| declared == upper),
        None => true,
    }
}

fn update_representation_headers(headers: &mut HeaderMap, body_len: usize) {
    headers.remove(header::ETAG);
    headers.remove(header::CONTENT_RANGE);
    headers.remove(header::ACCEPT_RANGES);
    headers.remove(HeaderName::from_static("content-md5"));
    headers.remove(HeaderName::from_static("digest"));
    headers.remove(HeaderName::from_static("content-digest"));
    headers.remove(header::CONTENT_LENGTH);

    if let Ok(value) = HeaderValue::from_str(&body_len.to_string()) {
        headers.insert(header::CONTENT_LENGTH, value);
    }
}

fn body_collection_failure() -> Response {
    let mut response = Response::new(axum::body::Body::from("response masking failed"));
    *response.status_mut() = StatusCode::BAD_GATEWAY;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/plain; charset=utf-8"),
    );
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

/// Automatic PII (Personally Identifiable Information) masking middleware for response payloads.
#[cfg_attr(mutants, mutants::skip)]
pub async fn pii_masking_middleware(req: Request, next: Next) -> Response {
    let request_method = req.method().clone();
    let response = next.run(req).await;

    if request_method == Method::HEAD
        || response.status().is_informational()
        || matches!(
            response.status(),
            StatusCode::NO_CONTENT | StatusCode::NOT_MODIFIED
        )
        || !is_textual_response(response.headers())
        || !has_identity_encoding(response.headers())
        || !is_safely_bufferable(response.headers(), response.body())
    {
        return response;
    }

    let (mut parts, body) = response.into_parts();
    let bytes = match axum::body::to_bytes(body, MAX_BUFFERED_RESPONSE_BYTES as usize).await {
        Ok(bytes) => bytes,
        Err(_) => return body_collection_failure(),
    };
    let Ok(body_text) = std::str::from_utf8(&bytes) else {
        return Response::from_parts(parts, axum::body::Body::from(bytes));
    };

    let masked_body = mask_pii(body_text);
    if masked_body.as_bytes() != bytes.as_ref() {
        update_representation_headers(&mut parts.headers, masked_body.len());
    }

    Response::from_parts(parts, axum::body::Body::from(masked_body))
}

/// Helper function to perform lightweight regex-free PII masking for emails and credit card numbers.
#[cfg_attr(mutants, mutants::skip)]
pub fn mask_pii(text: &str) -> String {
    let mut chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_ascii_digit() {
            let mut digit_indices = vec![i];
            let mut j = i + 1;
            let mut non_digits = 0;
            while j < chars.len() && non_digits < 3 {
                let c = chars[j];
                if c.is_ascii_digit() {
                    digit_indices.push(j);
                    non_digits = 0;
                } else if c == ' ' || c == '-' {
                    non_digits += 1;
                } else {
                    break;
                }
                j += 1;
            }

            let count = digit_indices.len();
            if let Some(mask_count) = card_mask_count(count) {
                for idx in 0..mask_count {
                    chars[digit_indices[idx]] = '*';
                }
                i = j;
                continue;
            }
        }
        i += 1;
    }

    let mut idx = 0;
    while idx < chars.len() {
        if chars[idx] == '@' {
            let mut start = idx;
            while start > 0 {
                let c = chars[start - 1];
                if c.is_alphanumeric() || c == '.' || c == '_' || c == '%' || c == '+' || c == '-' {
                    start -= 1;
                } else {
                    break;
                }
            }

            let mut end = idx + 1;
            let mut dot_seen = false;
            while end < chars.len() {
                let c = chars[end];
                if c.is_alphanumeric() || c == '-' {
                    end += 1;
                } else if c == '.' {
                    dot_seen = true;
                    end += 1;
                } else {
                    break;
                }
            }

            let username_len = idx - start;
            let domain_len = end - (idx + 1);
            if username_len > 1 && domain_len > 3 && dot_seen {
                for item in chars.iter_mut().take(idx).skip(start + 1) {
                    *item = '*';
                }
                idx = end;
                continue;
            }
        }
        idx += 1;
    }

    chars.into_iter().collect()
}
