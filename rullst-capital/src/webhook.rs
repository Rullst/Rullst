use crate::capital::provider;
use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use std::collections::HashMap;

/// An Axum middleware to intercept billing webhooks and verify their signatures cryptographically.
/// If valid, the `WebhookEvent` is inserted into the request extensions for the handler to consume.
pub async fn verify_webhook(
    headers: HeaderMap,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // We must read the raw body for HMAC signature verification.
    // However, consuming the body inside middleware means the downstream handler
    // cannot read it as `Json<T>` or `Bytes`.
    // We will read the body, verify the signature, inject the event, and then we could
    // reconstruct the body if needed, but since we parsed the event, the handler just needs the event.

    // Extract body bytes (limited to 2MB to prevent DoS/OOM exhaustion)
    const MAX_WEBHOOK_PAYLOAD_BYTES: usize = 2 * 1024 * 1024;
    let body_bytes = match axum::body::to_bytes(req.into_body(), MAX_WEBHOOK_PAYLOAD_BYTES).await {
        Ok(bytes) => bytes,
        Err(_) => return Err(StatusCode::PAYLOAD_TOO_LARGE),
    };

    // Convert HeaderMap to HashMap<String, String> as expected by BillingProvider
    let mut header_map = HashMap::new();
    for (k, v) in headers.iter() {
        if let Ok(v_str) = v.to_str() {
            header_map.insert(k.as_str().to_lowercase(), v_str.to_string());
        }
    }

    // Call the active provider to verify and parse the event
    let p = match provider() {
        Some(p) => p,
        None => {
            eprintln!("rullst-capital: BillingProvider not initialized — webhook rejected");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    match p.handle_webhook(&body_bytes, &header_map) {
        Ok(event) => {
            // Create a new request with an empty body (or original body) and the extension
            let (mut parts, _) = Request::new(axum::body::Body::from(body_bytes)).into_parts();
            parts.extensions.insert(event);
            parts.headers = headers;

            let req = Request::from_parts(parts, axum::body::Body::empty());
            Ok(next.run(req).await)
        }
        Err(e) => {
            // Log the error in a real app, but for now we reject
            eprintln!("Webhook verification failed: {}", e);
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}
