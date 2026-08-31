//! Bounded, versioned JSON envelopes for Rullst web and platform clients.
//!
//! These types carry data and correlation metadata only. Authentication,
//! authorization, ownership, tenant scope, trusted time and domain decisions
//! must still be derived and enforced by the server.

mod error;
mod policy;
mod types;

pub use error::ClientContractError;
pub use policy::ClientContractPolicy;
pub use types::{
    ClientRequest, ClientVersionOffer, ContractVersion, FailureCode, FailureDetail, IdempotencyKey,
    RequestId, ServerFailure, ServerResponse,
};

/// Stable protocol marker serialized into every envelope.
pub const CLIENT_CONTRACT_NAME: &str = "rullst.client";
/// Current wire version emitted by the framework.
pub const CURRENT_CLIENT_CONTRACT_VERSION: ContractVersion = ContractVersion(1);
/// Default maximum encoded request or response size: 256 KiB.
pub const DEFAULT_CLIENT_CONTRACT_BODY_BYTES: usize = 256 * 1024;
/// Hard configuration ceiling for the generic codec: 2 MiB.
pub const MAX_CLIENT_CONTRACT_BODY_BYTES: usize = 2 * 1024 * 1024;

#[cfg(test)]
mod tests;
