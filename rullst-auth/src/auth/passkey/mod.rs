pub mod cbor;
mod ceremony;
pub mod config;
pub mod service;
pub mod types;

#[cfg(test)]
mod invariant_tests;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;

pub use config::PasskeyConfig;
pub use service::PasskeyAuth;
pub use types::{
    AllowCredential, AuthenticatorAssertionResponse, AuthenticatorAttestationResponse,
    AuthenticatorSelection, CreationChallengeResponse, Passkey, PubKeyCredParam,
    PublicKeyCredential, PublicKeyCredentialCreationOptions, PublicKeyCredentialRequestOptions,
    RegisterPublicKeyCredential, RelyingPartyInfo, RequestChallengeResponse, UserInfo,
};
