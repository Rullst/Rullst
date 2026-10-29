//! The Rullst framework: one dependency for the Core runtime and, through Cargo
//! features, the optional Rullst crates.
//!
//! The crate root re-exports `rullst-core`: the HTTP [`Server`], the
//! [`routes!`] and [`html!`] macros, configuration, the security baseline,
//! queues, realtime and the other Core modules. Each optional crate appears
//! under its own module when you enable its feature:
//!
//! | Feature | Module | Crate | Tier |
//! | :--- | :--- | :--- | :--- |
//! | `orm` with `drivers-all` or one `strict-*` profile | `rullst::orm` (also in `rullst::db`) | `rullst-orm` | Core |
//! | `security` | `rullst::security::runtime` | `rullst-security` | Core |
//! | `auth` | `rullst::auth` | `rullst-auth` | Extension |
//! | `oauth` | `rullst::connect` | `rullst-connect` | Extension |
//! | `mail` | `rullst::mail` | `rullst-mail` | Extension |
//! | `capital` | `rullst::capital` | `rullst-capital` | Extension |
//! | `ai` | `rullst::ai` | `rullst-ai` | Extension |
//! | `messaging` | `rullst::messaging` | `rullst-messaging` | Extension |
//! | `nexus` | `rullst::nexus` | `rullst-nexus` | Extension |
//! | `studio` | `rullst::studio` | `rullst-studio` | Extension |
//! | `iot` | `rullst::iot` | `rullst-iot` | Experimental |
//! | `privacy` | `rullst::privacy` | `rullst-privacy` | Experimental |
//!
//! There is no default feature, so a database-free application compiles no
//! database code. Pin the exact version with `=` and name the features you use,
//! for example `features = ["strict-postgres", "auth"]`; `cargo rullst new`
//! writes this for you. docs.rs builds this page with the default (empty)
//! feature set, so the optional modules are not listed here: each crate has
//! its own documentation.
//!
//! Guides, tutorials and the maturity tiers are in the
//! [Rullst book](https://rullst.github.io/Rullst/book/).

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
