/// WebAuthn and Passkey authentication submodule.
pub mod passkey;

mod app_key;
mod password;
mod session;

/// Largest password, in UTF-8 bytes, accepted by the Argon2 helpers and the
/// recovery registry. Longer input is rejected rather than silently truncated.
pub(crate) const MAX_PASSWORD_BYTES: usize = 72;

pub use app_key::{get_app_key, parse_app_key_from_toml, validate_app_key};
pub use password::{
    dummy_verify, hash_password, hash_password_async, needs_rehash, verify_password,
    verify_password_async,
};
pub use session::{
    decrypt_session, encrypt_session, extract_session_cookie, make_login_cookie, make_logout_cookie,
};

#[cfg(feature = "oauth")]
pub mod connect {
    //! Re-export do rullst-connect para fornecer autenticação OAuth2 (Google, GitHub, etc.) nativamente no framework.
    pub use rullst_connect::*;
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests;
