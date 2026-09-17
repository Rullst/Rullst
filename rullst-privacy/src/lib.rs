//! Optional privacy contracts for the unreleased Rullst v13 line.
//!
//! Enable `age-assurance` for risk policies, signed threshold attestations and
//! replay protection. This crate does not infer age from images, certify legal
//! compliance, or replace application authentication and authorization.
#![forbid(unsafe_code)]
#![cfg_attr(feature = "age-assurance", doc = include_str!("../README.md"))]

#[cfg(feature = "age-assurance")]
pub mod age_assurance;
