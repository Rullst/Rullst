//! An application-owned Postmark transport for `rullst-mail`.
//!
//! Rullst 13 ships Resend, AWS SES, SendPulse and SMTP; every other provider
//! is a `MailDriver` the application owns. This crate is one to copy: it
//! sends through Postmark's Email API (`POST https://api.postmarkapp.com/email`)
//! and keeps the built-in drivers' contract.
//!
//! - [`PostmarkDriver`] runs the shared `DeliveryPipeline` first, maps
//!   `from`, `reply_to`, the bodies, attachments (inline images as
//!   `ContentID: "cid:…"`) and the `List-Unsubscribe` headers to Postmark's
//!   fields, refuses what Postmark cannot express (a future `send_at`, a
//!   request over 10 MB), reads a bounded receipt and classifies failures
//!   with `MailError::from_provider_response`.
//! - An empty or `mock_*` server token selects the deterministic offline
//!   mode: messages are validated as for a real send, then captured in
//!   [`PostmarkDriver::offline_outbox`] without network I/O.
//! - Suppression works as for the built-in drivers: wrap the driver in a
//!   `SuppressionGuard`, and record Postmark's own suppressions (its
//!   `SubscriptionChange` webhook) in the store with [`PostmarkWebhook`].
//!
//! The tests run the `rullst-mail` conformance kit against the offline mode
//! and the live path against a loopback HTTP fixture. Validate the transport
//! against a Postmark sandbox server and a confirmed sender signature before
//! production. The walkthrough is `docs/src/mail-custom-transport.md`.

mod driver;
mod webhook;

pub use driver::{OfflineOutbox, PROVIDER, PostmarkDriver, accepted, request_body};
pub use webhook::{PostmarkWebhook, WebhookError};
