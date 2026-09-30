//! Durable shared-local video state and authorized lifecycle orchestration.
mod access;
mod failure;
mod notifications;
mod playback;
mod record;
mod retention;
mod service;
mod store;
mod transaction;
mod workflow;

pub use record::{Asset, Lifecycle, OperationFailure};
pub use service::MediaService;
pub use store::{SqliteMedia, StoreConfig};
