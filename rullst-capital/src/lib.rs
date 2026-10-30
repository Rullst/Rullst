//! Provider-neutral billing for Rullst: checkout, subscriptions, signed
//! webhooks, quotas and entitlements.
//!
//! [`BillingProvider`] and [`MeteredBillingProvider`] are the provider
//! contracts. [`StripeProvider`] is the supported adapter. [`InfinitePayProvider`]
//! is experimental: it runs offline fixtures only, and its live checkout and
//! callback verification return [`CapitalError::UnsupportedOperation`]. Other
//! gateways implement the traits in application code. Empty or `mock_*`
//! credentials select deterministic offline fixtures.
//!
//! The crate also provides webhook signature, freshness and replay verification
//! (the default `axum` feature adds the `verify_webhook` middleware), the
//! [`Billable`] facade, shared quotas ([`QuotaGate`]), plan
//! [`entitlements`], immediate charges, coupons and trials, metered usage and
//! payment-bound invoices ([`Invoice`], [`PaidInvoice`]).
//!
//! **Maturity: Extension**
//! ([tiers](https://rullst.github.io/Rullst/book/maturity.html)), with
//! InfinitePay marked experimental inside it. Cargo features: `axum` (default),
//! `actix`, `invoice-pdf`, `quota-sql` and `webhook-sql`. v13 removed the other
//! payment providers, payouts and NFS-e.
//!
//! See the [book page](https://rullst.github.io/Rullst/book/crates/capital.html)
//! and the [README](https://github.com/Rullst/Rullst/blob/main/rullst-capital/README.md).
#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod billable;
pub mod capital;
pub mod charge;
pub mod checkout;
mod currency;
pub mod customer;
pub mod dashboard;
pub mod entitlements;
pub mod error;
pub mod invoice;
pub mod one_time_checkout;
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
pub use invoice::*;
pub use quota::*;
pub use stripe_checkout_event::*;
pub use stripe_event::*;
pub use stripe_snapshot::*;
pub use subscription::*;
pub use usage::*;

#[cfg(any(feature = "axum", feature = "actix", feature = "webhook-sql"))]
pub use webhook::*;
