//! Apple OAuth2 and JWT token claims data structures.

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct AppleClaims<'a> {
    pub iss: &'a str,
    pub iat: u64,
    pub exp: u64,
    pub aud: &'a str,
    pub sub: &'a str,
}
