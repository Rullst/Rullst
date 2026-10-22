//! The short-lived cookie that carries only a challenge id.

use super::{ChallengeError, ChallengeId, MAX_CHALLENGE_LIFETIME_SECONDS};

/// The challenge cookie's name. The `rullst_` prefix makes Rullst Core add
/// `Secure` whenever the request arrived over HTTPS, in every environment.
pub const CHALLENGE_COOKIE_NAME: &str = "rullst_oauth_challenge";

const MAX_PATH_BYTES: usize = 256;
/// Longer than any [`ChallengeId`]: such a value is never one of ours.
const MAX_VALUE_BYTES: usize = 128;

/// Builds and reads the `Set-Cookie`/`Cookie` values of the challenge id.
///
/// The cookie is `HttpOnly`, lives at most ten minutes and uses
/// `SameSite=Lax`, which a provider's top-level cross-site `GET` callback
/// still receives. Its `Path` is the login route prefix (for example
/// `/auth/google`), so no other route sees it. It never contains the state,
/// verifier or nonce.
///
/// Pass `secure` the way Rullst Core decides it for its own cookies:
/// `Environment::requires_secure_defaults()` (staging and production).
/// Core also adds `Secure` to this cookie on HTTPS requests.
///
/// A provider that returns the callback as a cross-site form POST
/// (`response_mode=form_post`, such as Sign in with Apple) needs
/// [`Self::for_cross_site_post`]: browsers send no `SameSite=Lax` cookie
/// with that POST.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChallengeCookie {
    path: String,
    secure: bool,
    cross_site_post: bool,
}

impl ChallengeCookie {
    /// A cookie scoped to `path`: `/` followed by at most 255 bytes of
    /// `A-Z`, `a-z`, `0-9`, `-`, `.`, `_`, `~` or `/`.
    pub fn new(path: impl Into<String>, secure: bool) -> Result<Self, ChallengeError> {
        let path = path.into();
        let valid = path.starts_with('/')
            && path.len() <= MAX_PATH_BYTES
            && path.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~' | b'/')
            });
        if !valid {
            return Err(ChallengeError::Invalid("cookie path"));
        }
        Ok(Self {
            path,
            secure,
            cross_site_post: false,
        })
    }

    /// The same cookie for a callback the provider POSTs from its own site
    /// (`response_mode=form_post`): `SameSite=None` and always `Secure`,
    /// because browsers attach only `SameSite=None` cookies to a cross-site
    /// POST and accept that attribute only on `Secure` cookies (which they
    /// keep over HTTPS and on `localhost`). The id still lives at most ten
    /// minutes, is consumed once and is useless without the matching
    /// `state`. Added in v13.
    #[must_use]
    pub fn for_cross_site_post(mut self) -> Self {
        self.cross_site_post = true;
        self.secure = true;
        self
    }

    /// The `Set-Cookie` value storing `id` for `max_age_seconds`, clamped to
    /// 1 through 600 seconds.
    #[must_use]
    pub fn set(&self, id: &ChallengeId, max_age_seconds: u64) -> String {
        let max_age = max_age_seconds.clamp(1, MAX_CHALLENGE_LIFETIME_SECONDS);
        format!(
            "{CHALLENGE_COOKIE_NAME}={}; Path={}; Max-Age={max_age}; HttpOnly; SameSite={}{}",
            id.as_str(),
            self.path,
            self.same_site(),
            self.secure_attribute()
        )
    }

    /// The `Set-Cookie` value that deletes the cookie.
    #[must_use]
    pub fn clear(&self) -> String {
        format!(
            "{CHALLENGE_COOKIE_NAME}=; Path={}; Max-Age=0; Expires=Thu, 01 Jan 1970 00:00:00 GMT; HttpOnly; SameSite={}{}",
            self.path,
            self.same_site(),
            self.secure_attribute()
        )
    }

    /// The challenge id from the request's `Cookie` header values.
    ///
    /// Returns `None` when no cookie has this name, or when several carry
    /// different values: an ambiguous pair (for example one planted by a
    /// sibling subdomain) is refused rather than guessed. The value is not
    /// validated here; `finish_login` treats a malformed one as unknown.
    pub fn read<'a>(cookie_headers: impl IntoIterator<Item = &'a str>) -> Option<String> {
        let mut found: Option<&str> = None;
        for header in cookie_headers {
            for pair in header.split(';') {
                let Some((name, value)) = pair.trim().split_once('=') else {
                    continue;
                };
                if name != CHALLENGE_COOKIE_NAME {
                    continue;
                }
                if value.is_empty() || value.len() > MAX_VALUE_BYTES {
                    return None;
                }
                match found {
                    Some(previous) if previous != value => return None,
                    _ => found = Some(value),
                }
            }
        }
        found.map(str::to_string)
    }

    fn same_site(&self) -> &'static str {
        if self.cross_site_post { "None" } else { "Lax" }
    }

    fn secure_attribute(&self) -> &'static str {
        if self.secure { "; Secure" } else { "" }
    }
}

#[cfg(test)]
#[path = "cookie_tests.rs"]
mod tests;
