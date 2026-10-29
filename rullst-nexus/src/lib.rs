//! Rullst Nexus: an authenticated, server-rendered administrative panel for
//! registered `rullst-orm` models.
//!
//! It provides CRUD, search and pagination for models that implement
//! [`NexusModel`] (manually or through `#[derive(Nexus)]` from `rullst-orm`),
//! server-side field validation, an optional tenant scope and transaction-coupled
//! mutation audit, telemetry and security views and an optional AI query page.
//! [`Nexus::try_build`] fails closed until an explicit [`NexusAuthPolicy`] is
//! selected, and returns an `axum::Router` to mount under `/nexus`.
//!
//! **Maturity: Extension**
//! ([tiers](https://rullst.github.io/Rullst/book/maturity.html)), optional and
//! supported for its documented scope. The default `drivers-all` feature
//! enables every ORM driver; with `default-features = false`, `strict-sqlite`,
//! `strict-postgres` or `strict-mysql` selects one backend instead. `tz` adds
//! IANA time zones for date-time fields. Applications usually enable it through
//! the `rullst` facade's `nexus` feature. See the [crate
//! guide](https://rullst.github.io/Rullst/book/crates/nexus.html) and the
//! [README](https://github.com/Rullst/Rullst/blob/main/rullst-nexus/README.md).

pub mod nexus;
pub use nexus::*;
