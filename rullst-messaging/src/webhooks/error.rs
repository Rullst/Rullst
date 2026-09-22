#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum WebhookError {
    #[error("invalid webhook configuration or bounded input: {0}")]
    InvalidInput(&'static str),
    #[error("webhook configuration does not match its immutable outbox")]
    Configuration,
    #[error("webhook destination is not an approved public HTTPS address")]
    DestinationDenied,
    #[error("webhook DNS resolution failed")]
    Resolution,
    #[error("webhook TLS or HTTP transport failed")]
    Transport,
    #[error("webhook operation exceeded its deadline")]
    Timeout,
    #[error("webhook storage is unavailable")]
    Storage,
    #[error("webhook capacity is exhausted")]
    Capacity,
    #[error("webhook event identity conflicts with retained content")]
    Conflict,
    #[error("webhook lease is expired, cancelled or stale")]
    InvalidLease,
    #[error("webhook timestamp or trusted clock is invalid")]
    Clock,
    #[error("webhook signature or receipt metadata is invalid")]
    InvalidSignature,
    #[error("receiver accepted webhook but local acknowledgement is uncertain")]
    Acknowledgement { delivery_id: String, status: u16 },
}
