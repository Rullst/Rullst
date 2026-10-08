//! Billing and monetization demonstration for Rullst Capital.
//! Includes SaaS tier quotas and offline Stripe/InfinitePay provider fixtures.

pub mod gateways;
pub mod handlers;
pub mod views;

pub use handlers::{
    Subscriber, checkout_handler, checkout_handler_get, checkout_handler_post, pricing_page,
};
