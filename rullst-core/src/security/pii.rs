//! Personally Identifiable Information (PII) masking algorithms & middleware.

use axum::{
    extract::Request,
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};

/// Automatic PII (Personally Identifiable Information) masking middleware for response payloads.
#[cfg_attr(mutants, mutants::skip)]
pub async fn pii_masking_middleware(req: Request, next: Next) -> Response {
    let response = next.run(req).await;

    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if content_type.contains("text")
        || content_type.contains("json")
        || content_type.contains("javascript")
    {
        let (parts, body) = response.into_parts();
        if let Ok(bytes) = axum::body::to_bytes(body, 2 * 1024 * 1024).await {
            let body_str = String::from_utf8_lossy(&bytes);
            let masked_body = mask_pii(&body_str);

            let mut parts = parts;
            if parts.headers.contains_key(header::CONTENT_LENGTH) {
                if let Ok(val) = axum::http::HeaderValue::from_str(&masked_body.len().to_string()) {
                    parts.headers.insert(header::CONTENT_LENGTH, val);
                }
            }

            let new_body = axum::body::Body::from(masked_body);
            return Response::from_parts(parts, new_body);
        } else {
            match Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(axum::body::Body::empty())
            {
                Ok(res) => return res,
                Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            }
        }
    }

    response
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
            if (13..=19).contains(&count) {
                let mask_count = count - 4;
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
