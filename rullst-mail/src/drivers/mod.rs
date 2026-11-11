// src/drivers/mod.rs — Driver trait and error definitions.

pub mod aws_ses;
pub mod failover;
mod http;
pub mod log;
pub mod memory;
pub mod resend;
mod rest;
pub mod sendpulse;
pub mod smtp;
pub mod traits;

pub use self::aws_ses::AwsSesDriver;
pub use self::failover::FailoverDriver;
pub use self::log::LogDriver;
pub use self::memory::{MailAssertion, MailTrap, MemoryDriver};
pub use self::resend::ResendDriver;
pub use self::sendpulse::SendPulseDriver;
pub use self::smtp::SmtpDriver;
pub use self::traits::MailDriver;
pub use crate::mock::{DeliveryMode, OfflineMailMock, OfflineMockDelivery, credential_mode};

pub use crate::error::MailError;
