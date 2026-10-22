//! The callback half: consume the challenge, then validate the response.

use super::{LoginError, unix_now};
use crate::challenge::{ChallengeError, ChallengeId, ChallengeStore, ReturnPath};
use crate::extractors::AuthCallback;
use crate::provider::{ExchangeParams, Provider};
use crate::user::ConnectUser;

const MAX_ERROR_CODE_CHARS: usize = 64;

/// A provider identity whose callback matched its stored challenge.
///
/// The identity is the provider's: map [`ConnectUser::id`] (the provider
/// subject) to an application account, trust its email only when
/// `email_verified` is `Some(true)`, and then issue the application's own
/// session.
#[derive(Debug)]
pub struct CompletedLogin {
    user: ConnectUser,
    return_to: Option<ReturnPath>,
}

impl CompletedLogin {
    /// The provider profile from the validated exchange.
    #[must_use]
    pub fn user(&self) -> &ConnectUser {
        &self.user
    }

    /// The same-site page the login started from, if one was named.
    #[must_use]
    pub fn return_to(&self) -> Option<&ReturnPath> {
        self.return_to.as_ref()
    }

    /// The profile and the return page, by value.
    #[must_use]
    pub fn into_parts(self) -> (ConnectUser, Option<ReturnPath>) {
        (self.user, self.return_to)
    }
}

/// Consumes the challenge named by the cookie value `challenge_id`, then
/// validates `callback` against it and exchanges the code.
///
/// In order: no cookie value is [`LoginError::MissingChallenge`]; a
/// malformed, unknown or already used id is
/// `LoginError::Challenge(ChallengeError::NotFound)` and an expired one
/// `Challenge(ChallengeError::Expired)`. From here the challenge is
/// consumed, whatever follows. A challenge issued for another provider is
/// [`LoginError::ProviderMismatch`]; the `state` is compared in constant time
/// ([`AuthCallback::verify_state`]) before anything else in the callback is
/// trusted ([`LoginError::StateMismatch`]); then a provider `error` is
/// [`LoginError::Denied`], a missing code [`LoginError::MissingCode`], and
/// the exchange runs with the stored PKCE verifier and, for OIDC, the stored
/// nonce ([`LoginError::Exchange`]). `provider_name` must be the name given
/// to [`super::LoginOptions`].
pub async fn finish_login<S, P>(
    store: &S,
    provider: &P,
    provider_name: &str,
    challenge_id: Option<&str>,
    callback: &AuthCallback,
) -> Result<CompletedLogin, LoginError>
where
    S: ChallengeStore,
    P: Provider + ?Sized,
{
    finish_login_at(
        store,
        provider,
        provider_name,
        challenge_id,
        callback,
        unix_now()?,
    )
    .await
}

pub(in crate::challenge) async fn finish_login_at<S, P>(
    store: &S,
    provider: &P,
    provider_name: &str,
    challenge_id: Option<&str>,
    callback: &AuthCallback,
    now: u64,
) -> Result<CompletedLogin, LoginError>
where
    S: ChallengeStore,
    P: Provider + ?Sized,
{
    let challenge_id = challenge_id.ok_or(LoginError::MissingChallenge)?;
    // An altered id is indistinguishable from an unknown one.
    let challenge_id = ChallengeId::parse(challenge_id).map_err(|_| ChallengeError::NotFound)?;
    let challenge = store.take(&challenge_id, now).await?;
    if challenge.provider() != provider_name {
        return Err(LoginError::ProviderMismatch);
    }
    callback
        .verify_state(challenge.state())
        .map_err(|_| LoginError::StateMismatch)?;
    if let Some(error) = &callback.error {
        return Err(LoginError::Denied(bounded_error_code(error)));
    }
    let code = callback
        .code
        .as_deref()
        .filter(|code| !code.is_empty())
        .ok_or(LoginError::MissingCode)?;
    let user = provider
        .get_user(ExchangeParams {
            auth_code: code,
            code_verifier: Some(challenge.code_verifier()),
            expected_nonce: challenge.nonce(),
        })
        .await
        .map_err(LoginError::Exchange)?;
    Ok(CompletedLogin {
        user,
        return_to: challenge.return_to().cloned(),
    })
}

fn bounded_error_code(error: &str) -> String {
    error
        .chars()
        .filter(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.')
        })
        .take(MAX_ERROR_CODE_CHARS)
        .collect()
}
