//! Billing & Fiscal Monetization demonstration for Rullst Capital.
//! Includes SaaS Tier quotas, verified HMAC webhook simulation, 11 connected gateways, and real SPED NFS-e DPS generation.

pub mod gateways;
pub mod handlers;
pub mod views;

pub use handlers::{
    Subscriber, checkout_handler, checkout_handler_get, checkout_handler_post, pricing_page,
};
