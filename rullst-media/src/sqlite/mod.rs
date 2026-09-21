//! Durable shared-local video state and authorized lifecycle orchestration.
mod access;
mod notifications;
mod record;
mod retention;
mod service;
mod store;
mod transaction;
mod workflow;

pub use record::{Asset, Lifecycle};
pub use service::MediaService;
pub use store::{SqliteMedia, StoreConfig};
