// The test harness registers the unit tests of the modules deprecated in 12.3
// from the crate root, where no item-level allow can reach them.
#![cfg_attr(test, allow(deprecated))]

pub mod billable;
pub mod capital;
pub mod charge;
pub mod checkout;
pub mod customer;
pub mod dashboard;
pub mod error;
#[deprecated(
    since = "12.3.0",
    note = "removed in Rullst 13.0; see the v13 migration guide row \"Capital providers and NFS-e removed\""
)]
pub mod fiscal;
pub mod invoice;
pub mod one_time_checkout;
#[deprecated(
    since = "12.3.0",
    note = "removed in Rullst 13.0; see the v13 migration guide row \"Capital providers and NFS-e removed\""
)]
pub mod paddle_checkout;
#[deprecated(
    since = "12.3.0",
    note = "removed in Rullst 13.0; see the v13 migration guide row \"Capital providers and NFS-e removed\""
)]
pub mod polar_checkout;
pub mod providers;
pub mod quota;
pub mod stripe_checkout_event;
pub use one_time_checkout::*;
pub mod one_time_event;
pub use one_time_event::*;
pub mod stripe_event;
pub mod stripe_snapshot;
pub mod subscription;
pub mod usage;

#[cfg(any(feature = "axum", feature = "actix", feature = "webhook-sql"))]
pub mod webhook;

pub use billable::*;
pub use capital::*;
pub use charge::*;
pub use checkout::*;
pub use customer::*;
pub use dashboard::*;
pub use error::*;
// Deprecated in 12.3 (v13 migration guide row "Capital providers and NFS-e removed").
#[allow(deprecated)]
pub use fiscal::*;
pub use invoice::*;
// Deprecated in 12.3 (v13 migration guide row "Capital providers and NFS-e removed").
#[allow(deprecated)]
pub use paddle_checkout::*;
// Deprecated in 12.3 (v13 migration guide row "Capital providers and NFS-e removed").
#[allow(deprecated)]
pub use polar_checkout::*;
pub use quota::*;
pub use stripe_checkout_event::*;
pub use stripe_event::*;
pub use stripe_snapshot::*;
pub use subscription::*;
pub use usage::*;

#[cfg(any(feature = "axum", feature = "actix", feature = "webhook-sql"))]
pub use webhook::*;
