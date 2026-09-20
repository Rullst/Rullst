extern crate self as rullst;

pub use rullst_core::*;
pub use rullst_macros::{Billable, require_role};

#[cfg(target_arch = "wasm32")]
mod server_function_wasm_contract;

/// Security facade for the lightweight Core middleware and, when the
/// `security` feature is enabled, the extended `rullst-security` suite.
///
/// The nested `runtime` module keeps the two existing `CspNonce` types
/// unambiguous while the security crates are consolidated in a future
/// SemVer-planned architecture cycle.
pub mod security {
    #[cfg(not(target_arch = "wasm32"))]
    pub use rullst_core::security::*;

    #[cfg(all(feature = "security", not(target_arch = "wasm32")))]
    pub use rullst_security as runtime;
}

pub mod db {
    pub use rullst_core::db::*;
    #[cfg(feature = "orm")]
    pub use rullst_orm::*;
}

#[cfg(feature = "orm")]
pub use rullst_orm as orm;
#[cfg(feature = "orm")]
pub use rullst_orm;

#[cfg(feature = "auth")]
pub use rullst_auth as auth;

/// Integration between durable Auth recovery and deterministic Mail delivery.
#[cfg(any(feature = "account-mail-sqlite", feature = "account-mail-postgres"))]
pub mod account_mail;

#[cfg(feature = "oauth")]
pub use rullst_connect as connect;

#[cfg(feature = "mail")]
pub use rullst_mail as mail;

#[cfg(all(feature = "messaging", not(target_arch = "wasm32")))]
pub use rullst_messaging as messaging;

/// Optional age-assurance and purpose-bound consent contracts.
#[cfg(all(feature = "privacy", not(target_arch = "wasm32")))]
pub use rullst_privacy as privacy;

#[cfg(feature = "ai")]
pub use rullst_ai as ai;

#[cfg(feature = "nexus")]
pub use rullst_nexus as nexus;

#[cfg(feature = "capital")]
pub use rullst_capital as capital;

#[cfg(feature = "studio")]
pub use rullst_studio as studio;

#[cfg(all(feature = "security", not(target_arch = "wasm32")))]
pub use rullst_security as security_runtime;

#[cfg(feature = "iot")]
pub use rullst_iot as iot;

// Compile the public guides and tutorials as doctests without exposing their aggregation
// module in normal builds or generated API documentation.
#[cfg(doctest)]
#[doc(hidden)]
pub mod book_doctests;
