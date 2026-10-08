// src/drivers/mod.rs — Driver trait and error definitions.

pub mod aws_ses;
#[deprecated(
    since = "12.3.0",
    note = "removed in Rullst 13.0; see the v13 migration guide row \"Mail providers removed\""
)]
#[allow(deprecated)]
pub mod azure;
pub mod failover;
mod http;
pub mod log;
#[deprecated(
    since = "12.3.0",
    note = "removed in Rullst 13.0; see the v13 migration guide row \"Mail providers removed\""
)]
#[allow(deprecated)]
pub mod mailjet;
#[deprecated(
    since = "12.3.0",
    note = "removed in Rullst 13.0; see the v13 migration guide row \"Mail providers removed\""
)]
#[allow(deprecated)]
pub mod mailtrap;
pub mod memory;
pub mod mock;
#[deprecated(
    since = "12.3.0",
    note = "removed in Rullst 13.0; see the v13 migration guide row \"Mail providers removed\""
)]
#[allow(deprecated)]
pub mod postmark;
pub mod resend;
mod rest;
#[deprecated(
    since = "12.3.0",
    note = "removed in Rullst 13.0; see the v13 migration guide row \"Mail providers removed\""
)]
#[allow(deprecated)]
pub mod sendgrid;
pub mod sendpulse;
pub mod smtp;
pub mod traits;

pub use self::aws_ses::AwsSesDriver;
// Deprecated in 12.3 (v13 migration guide row "Mail providers removed").
#[allow(deprecated)]
pub use self::azure::{
    AzureCommunicationDriver, AzureMailAccessToken, AzureMailCredential, AzureManagedIdentity,
    StaticAzureMailCredential,
};
pub use self::failover::FailoverDriver;
pub use self::log::LogDriver;
#[allow(deprecated)]
pub use self::mailjet::MailjetDriver;
#[allow(deprecated)]
pub use self::mailtrap::MailtrapDriver;
pub use self::memory::{MailAssertion, MailTrap, MemoryDriver};
pub use self::mock::{DeliveryMode, OfflineMailMock, OfflineMockDelivery, credential_mode};
#[allow(deprecated)]
pub use self::postmark::PostmarkDriver;
pub use self::resend::ResendDriver;
#[allow(deprecated)]
pub use self::sendgrid::SendGridDriver;
pub use self::sendpulse::SendPulseDriver;
pub use self::smtp::SmtpDriver;
pub use self::traits::MailDriver;

pub use crate::error::MailError;
