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

// `rullst-orm`, `rullst-auth`, `rullst-mail`, `rullst-ai`, `rullst-nexus`,
// `rullst-capital` and `rullst-studio` are native-only dependencies (see the
// `cfg(not(target_arch = "wasm32"))` table in Cargo.toml). Their features stay
// valid on wasm32 but enable no crate there, so every re-export carries the
// same target guard as Core's own ORM re-exports.
pub mod db {
    pub use rullst_core::db::*;
    #[cfg(all(feature = "orm", not(target_arch = "wasm32")))]
    pub use rullst_orm::*;
}

#[cfg(all(feature = "orm", not(target_arch = "wasm32")))]
pub use rullst_orm as orm;
#[cfg(all(feature = "orm", not(target_arch = "wasm32")))]
pub use rullst_orm;

#[cfg(all(feature = "auth", not(target_arch = "wasm32")))]
pub use rullst_auth as auth;

/// Integration between durable Auth recovery and deterministic Mail delivery.
#[cfg(all(
    any(feature = "account-mail-sqlite", feature = "account-mail-postgres"),
    not(target_arch = "wasm32")
))]
pub mod account_mail;

#[cfg(feature = "oauth")]
pub use rullst_connect as connect;

#[cfg(all(feature = "mail", not(target_arch = "wasm32")))]
pub use rullst_mail as mail;

#[cfg(all(feature = "messaging", not(target_arch = "wasm32")))]
pub use rullst_messaging as messaging;

/// Optional age-assurance and purpose-bound consent contracts.
#[cfg(all(feature = "privacy", not(target_arch = "wasm32")))]
pub use rullst_privacy as privacy;

#[cfg(all(feature = "ai", not(target_arch = "wasm32")))]
pub use rullst_ai as ai;

#[cfg(all(feature = "nexus", not(target_arch = "wasm32")))]
pub use rullst_nexus as nexus;

#[cfg(all(feature = "capital", not(target_arch = "wasm32")))]
pub use rullst_capital as capital;

#[cfg(all(feature = "studio", not(target_arch = "wasm32")))]
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
