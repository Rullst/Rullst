//! Optional privacy contracts for the Rullst v13 line.
//!
//! **Maturity: Experimental**
//! ([tiers](https://rullst.github.io/Rullst/book/maturity.html)). The API may
//! change between 13.x releases.
//!
//! No feature is enabled by default, and without one this crate exposes no
//! items. Enable `age-assurance` for risk policies, signed threshold
//! attestations and replay protection (`challenge-tokens`, `sqlite` and
//! `postgres` add to it), or `consent` for versioned optional-processing
//! choices (`consent-sqlite` and `consent-postgres` add durable stores). This
//! crate does not infer age from images, certify legal compliance, or replace
//! application authentication and authorization. The
//! [README](https://github.com/Rullst/Rullst/blob/main/rullst-privacy/README.md)
//! describes both features.
#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(unsafe_code)]
#![cfg_attr(feature = "age-assurance", doc = include_str!("../README.md"))]

#[cfg(any(feature = "postgres", feature = "consent-postgres"))]
mod postgres_connection;
#[cfg(any(feature = "sqlite", feature = "consent-sqlite"))]
mod sqlite_establish;

#[cfg(feature = "age-assurance")]
pub mod age_assurance;

#[cfg(feature = "consent")]
pub mod consent;
