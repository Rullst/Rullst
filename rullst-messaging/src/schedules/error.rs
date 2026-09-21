use crate::{MessagingError, PublishReceipt};

#[derive(Clone, Debug, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum RecurringError {
    #[error("invalid recurring-publication input: {0}")]
    InvalidInput(&'static str),
    #[error("recurring-publication configuration or durable state does not match")]
    Configuration,
    #[error("schedule name was reused with a different definition")]
    Conflict,
    #[error("recurring-publication capacity is exhausted")]
    Capacity,
    #[error("schedule or occurrence does not exist in this namespace")]
    NotFound,
    #[error("occurrence lease is stale, expired, cancelled or belongs to another namespace")]
    InvalidLease,
    #[error("recurring-publication storage is unavailable")]
    Storage,
    #[error("recurring-publication time is invalid or moved backwards")]
    Clock,
    #[error("recurring-publication encrypted content could not be authenticated")]
    Encryption,
    #[error("recurring-publication secure randomness is unavailable")]
    Randomness,
}

#[derive(Clone, Debug, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum RecurringRelayError {
    #[error("recurring publication failed before broker dispatch")]
    BeforePublication(#[source] RecurringError),
    #[error("broker publication failed; the occurrence remains subject to bounded retry")]
    Publication(#[source] MessagingError),
    #[error("broker accepted publication but the occurrence acknowledgement is uncertain")]
    Acknowledgement {
        publication: PublishReceipt,
        #[source]
        source: RecurringError,
    },
}
