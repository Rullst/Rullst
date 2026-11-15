// The WebAuthn payload parser behind `PasskeyAuth`.
pub(crate) mod cbor;
mod ceremony;
pub mod config;
mod es256;
pub mod service;
pub mod shared;
#[cfg(feature = "sqlite")]
mod sqlite;
pub mod types;

#[cfg(test)]
mod es256_differential_tests;
#[cfg(test)]
mod es256_wycheproof_tests;
#[cfg(test)]
mod invariant_tests;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;

pub use config::PasskeyConfig;
pub use service::PasskeyAuth;
#[cfg(feature = "sqlite")]
pub use sqlite::{PasskeyDeviceSummary, PasskeyStoreError, SqlitePasskeyStore};
pub use types::{
    AllowCredential, AuthenticatorAssertionResponse, AuthenticatorAttestationResponse,
    AuthenticatorSelection, CreationChallengeResponse, Passkey, PubKeyCredParam,
    PublicKeyCredential, PublicKeyCredentialCreationOptions, PublicKeyCredentialRequestOptions,
    RegisterPublicKeyCredential, RelyingPartyInfo, RequestChallengeResponse, UserInfo,
};
