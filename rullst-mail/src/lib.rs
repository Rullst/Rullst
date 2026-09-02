//! # `rullst-mail` — High-Performance Transactional Email & Mailables Engine
//!
//! Provides typed, bounded primitives for transactional emails with built-in:
//! - **RFC 8058 One-Click List-Unsubscribe** headers
//! - **Automatic Plain-Text Fallback** derivation
//! - **In-Memory MailTrap & Fluent Assertions**
//! - **Outbound DLP Secret Scanner** (AWS keys, passwords, bearer tokens)
//! - Multiple delivery drivers (**SMTP**, **Resend**, **SendGrid**, **Postmark**, **AWS SES**, **Log**, **Memory**, **Failover**)
//! - **Dynamic Multi-Tenancy Resolver** (`TenantMailResolver`)
//! - **Resilient Circuit Breaker & Automatic Failover** (`FailoverDriver`)
//! - Opt-in attachment inspection, recipient suppression, and minimized observations

pub mod attachment;
pub mod drivers;
pub mod error;
pub mod facade;
pub mod factory;
pub mod inspection;
pub mod message;
pub mod observability;
#[cfg(feature = "capital-invoice")]
pub mod paid_invoice;
pub mod pipeline;
pub mod resolver;
pub mod security;
pub mod suppression;
pub mod tracking;
pub mod validator;
pub mod worker;

/// Official AWS SES v2 SDK re-export used by native driver configuration.
#[cfg(feature = "aws-ses")]
pub use aws_sdk_sesv2 as aws_ses_sdk;

pub use attachment::*;
pub use drivers::*;
pub use error::*;
pub use facade::*;
pub use factory::*;
pub use inspection::*;
pub use message::*;
pub use observability::*;
#[cfg(feature = "capital-invoice")]
pub use paid_invoice::*;
pub use pipeline::*;
pub use resolver::*;
pub use security::*;
pub use suppression::*;
pub use tracking::*;
pub use validator::*;
pub use worker::*;

/// Backwards compatibility alias for `rullst_mail::mail::*`
pub mod mail {
    pub use crate::*;
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;

#[cfg(kani)]
#[cfg_attr(mutants, mutants::skip)]
mod kani_proofs {
    use super::*;

    #[kani::proof]
    fn proof_message_builder_recipient() {
        let msg = Message::new().to("user@rullst.dev").subject("Hello");
        assert!(!msg.to.is_empty());
        assert!(!msg.subject.is_empty());
    }
}
