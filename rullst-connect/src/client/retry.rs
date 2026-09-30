//! Method-aware retry policy for the optional `retry` feature.
//!
//! Token, device-poll and revocation requests are single-use POSTs: a code,
//! refresh token or device code may already have been consumed when a timeout
//! or connection reset hides the provider's answer. Replaying such a request
//! can trigger refresh-token reuse detection or invalidate a code, so those
//! requests are retried only when the provider explicitly rejected them with
//! HTTP 429 before processing them.

use reqwest_middleware::ClientWithMiddleware;
use reqwest_retry::policies::ExponentialBackoff;
use reqwest_retry::{
    DefaultRetryableStrategy, RetryTransientMiddleware, Retryable, RetryableStrategy,
};

/// Upper bound for a caller-selected retry count.
const MAX_RETRIES: u32 = 10;

/// Retries a non-idempotent request only after an explicit HTTP 429 rejection.
struct RateLimitedOnly;

impl RetryableStrategy for RateLimitedOnly {
    fn handle(
        &self,
        result: &Result<reqwest::Response, reqwest_middleware::Error>,
    ) -> Option<Retryable> {
        match result {
            Ok(response) if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS => {
                Some(Retryable::Transient)
            }
            Ok(_) => None,
            Err(_) => Some(Retryable::Fatal),
        }
    }
}

/// One shared connection pool behind two retry policies.
pub(super) struct RetryingClients {
    /// GET, HEAD and OPTIONS: transient transport errors, 5xx, 408 and 429.
    idempotent: ClientWithMiddleware,
    /// Every other method: only HTTP 429.
    single_use: ClientWithMiddleware,
}

impl RetryingClients {
    pub(super) fn new(client: reqwest::Client, max_retries: u32) -> Self {
        let policy =
            || ExponentialBackoff::builder().build_with_max_retries(max_retries.min(MAX_RETRIES));
        let idempotent = reqwest_middleware::ClientBuilder::new(client.clone())
            .with(RetryTransientMiddleware::new_with_policy_and_strategy(
                policy(),
                DefaultRetryableStrategy,
            ))
            .build();
        let single_use = reqwest_middleware::ClientBuilder::new(client)
            .with(RetryTransientMiddleware::new_with_policy_and_strategy(
                policy(),
                RateLimitedOnly,
            ))
            .build();
        Self {
            idempotent,
            single_use,
        }
    }

    pub(super) fn for_method(&self, method: &reqwest::Method) -> &ClientWithMiddleware {
        if matches!(
            *method,
            reqwest::Method::GET | reqwest::Method::HEAD | reqwest::Method::OPTIONS
        ) {
            &self.idempotent
        } else {
            &self.single_use
        }
    }
}
