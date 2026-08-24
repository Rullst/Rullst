//! Billing & Fiscal Monetization demonstration for Rullst Capital.
//! Includes SaaS tier quotas, offline provider fixtures, and an unsigned DPS XML preview.

pub mod gateways;
pub mod handlers;
pub mod views;

pub use handlers::{
    Subscriber, checkout_handler, checkout_handler_get, checkout_handler_post, pricing_page,
};
