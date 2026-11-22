//! [`SessionRegistry`]: the operations over one database.

use super::super::app_key::{detect_environment, get_app_key};
use super::super::session::{derive_cipher, extract_session_cookie};
use super::super::session_claims::{SessionClaims, max_age_seconds};
use super::revalidation::{Revoked, announce};
use super::sql::{NewRow, StoredRow};
use super::token::{self, SessionSecret};
use super::types::{
    AuthenticatedSession, CurrentSession, IssuedSession, NewSession, SessionInfo, SessionRef,
    valid_reference,
};
use super::{
    DeviceSummary, LAST_SEEN_GRANULARITY_SECONDS, MAX_PURGE_BATCH, MAX_SESSIONS_PER_USER,
    SessionDatabase, SessionRegistryError,
};
use axum::http::HeaderMap;
use sqlx::Pool;
use std::fmt;
use std::future::Future;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use subtle::ConstantTimeEq;

/// Every database call ends within this deadline.
const DEADLINE: Duration = Duration::from_secs(5);

/// The registered sessions in the table `rullst_auth_sessions` of the
/// application's database, reached through its own pool (for a Rullst
/// application, `Orm::pool()`).
///
/// See the [module documentation](super) for the model. Methods that take
/// `now` expect trusted server time in Unix seconds, never a value from the
/// request; [`sign_in`](Self::sign_in),
/// [`authenticate_request`](Self::authenticate_request) and
/// [`sign_out`](Self::sign_out) read `APP_KEY` and the system clock
/// themselves.
pub struct SessionRegistry<DB: SessionDatabase> {
    pool: Pool<DB>,
}

/// The current Unix time in seconds.
///
/// # Errors
/// [`SessionRegistryError::Crypto`] when the system clock is before 1970.
pub fn unix_now() -> Result<u64, SessionRegistryError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .map_err(|_| SessionRegistryError::Crypto)
}

async fn bounded<T>(
    operation: impl Future<Output = Result<T, SessionRegistryError>>,
) -> Result<T, SessionRegistryError> {
    tokio::time::timeout(DEADLINE, operation)
        .await
        .map_err(|_| SessionRegistryError::Unavailable)?
}

/// A Unix time as its signed column value.
fn seconds(value: u64) -> Result<i64, SessionRegistryError> {
    i64::try_from(value).map_err(|_| SessionRegistryError::InvalidInput("time"))
}

/// The start of the hour containing `now`.
fn last_seen_hour(now: u64) -> u64 {
    now - now % LAST_SEEN_GRANULARITY_SECONDS
}

/// A stored row after validation.
struct Stored {
    token_hash: String,
    reference: SessionRef,
    user_id: i32,
    created_at: u64,
    last_seen_at: u64,
    expires_at: u64,
    device: DeviceSummary,
}

impl Stored {
    fn decode(row: StoredRow) -> Result<Self, SessionRegistryError> {
        let (
            token_hash,
            public_id,
            user_id,
            created_at,
            last_seen_at,
            expires_at,
            browser,
            platform,
        ) = row;
        let time = |value: i64| u64::try_from(value).map_err(|_| SessionRegistryError::Corrupt);
        let (created_at, last_seen_at, expires_at) =
            (time(created_at)?, time(last_seen_at)?, time(expires_at)?);
        if !valid_reference(&public_id) || expires_at < created_at {
            return Err(SessionRegistryError::Corrupt);
        }
        Ok(Self {
            token_hash,
            reference: SessionRef(public_id),
            user_id,
            created_at,
            last_seen_at,
            expires_at,
            device: DeviceSummary::from_codes(&browser, &platform),
        })
    }
}

impl<DB: SessionDatabase> SessionRegistry<DB> {
    pub(super) fn pool(&self) -> &Pool<DB> {
        &self.pool
    }

    /// Creates the table `rullst_auth_sessions` and its indexes on `pool`
    /// when they are missing, then [`connect`](Self::connect)s: the
    /// create-on-first-use path for an application whose migrations do not
    /// own the table (the generated applications use a migration that calls
    /// [`create_table`](Self::create_table), and `connect`).
    ///
    /// The table it creates has no foreign key on `user_id`, so the rows of
    /// an account deleted with SQL stay until they expire; see
    /// [`create_table`](Self::create_table).
    pub async fn open(pool: Pool<DB>) -> Result<Self, SessionRegistryError> {
        Self::create_table(&pool).await?;
        Self::connect(pool).await
    }

    /// Creates the table `rullst_auth_sessions` and its indexes when they
    /// are missing; existing ones are kept. Call it from a migration whose
    /// rollback drops the table (`DROP TABLE rullst_auth_sessions` drops its
    /// indexes too). On PostgreSQL, concurrent calls serialize on an
    /// advisory lock.
    ///
    /// The registry does not know the application's accounts table, so
    /// `user_id` (`INTEGER NOT NULL`) has no foreign key. Code that deletes
    /// an account through the application calls
    /// [`revoke_all_sessions`](Self::revoke_all_sessions) (or
    /// [`revoke_all_sessions_on`](Self::revoke_all_sessions_on) in the
    /// deleting transaction), but an account deleted with SQL, such as by a
    /// retention job or for a deletion request, leaves its rows, with the
    /// account id, browser and platform family and times, until they expire
    /// (the session lifetime, 30 days by default) and
    /// [`purge_expired`](Self::purge_expired) removes them. Link the column
    /// to the accounts table in the migration that owns this table, as the
    /// one `cargo rullst auth` generates does: after this call, PostgreSQL
    /// and MySQL/MariaDB take `ALTER TABLE rullst_auth_sessions ADD CONSTRAINT
    /// rullst_auth_sessions_user_id_foreign FOREIGN KEY (user_id) REFERENCES
    /// users (id) ON DELETE CASCADE`; SQLite cannot add a key to an existing
    /// table, so create the same table with `user_id INTEGER NOT NULL
    /// REFERENCES users (id) ON DELETE CASCADE` before this call, which then
    /// keeps it and adds the indexes (SQLx enables SQLite's `foreign_keys`
    /// on every connection it opens). The type of the referenced column must
    /// match (`INTEGER`, or `INT` on MySQL).
    pub async fn create_table(pool: &Pool<DB>) -> Result<(), SessionRegistryError> {
        bounded(DB::prepare(pool)).await
    }

    /// The registry over `pool`, whose table must exist: nothing is
    /// created. Fails with [`Unavailable`](SessionRegistryError::Unavailable)
    /// when the table is missing or the database does not answer.
    pub async fn connect(pool: Pool<DB>) -> Result<Self, SessionRegistryError> {
        bounded(DB::probe(&pool)).await?;
        Ok(Self { pool })
    }

    /// Registers a login and returns its token, after the application has
    /// authenticated the account. The account's expired sessions are
    /// deleted; beyond [`MAX_SESSIONS_PER_USER`] the least recently used
    /// ones are signed out.
    ///
    /// # Errors
    /// [`InvalidInput`](SessionRegistryError::InvalidInput) for a lifetime
    /// outside 1 second to 400 days, [`Configuration`](SessionRegistryError::Configuration)
    /// for an invalid key, [`Crypto`](SessionRegistryError::Crypto) and
    /// [`Unavailable`](SessionRegistryError::Unavailable).
    pub async fn create_session(
        &self,
        session: &NewSession,
        app_key: &[u8],
        now: u64,
    ) -> Result<IssuedSession, SessionRegistryError> {
        let max_age_seconds = max_age_seconds(session.max_age)
            .map_err(|_| SessionRegistryError::InvalidInput("max_age"))?;
        let expires_at = now
            .checked_add(max_age_seconds)
            .ok_or(SessionRegistryError::InvalidInput("time"))?;
        let (created_at, stored_expiry) = (seconds(now)?, seconds(expires_at)?);
        let cipher = derive_cipher(app_key).map_err(|_| SessionRegistryError::Configuration)?;
        let secret = SessionSecret::generate()?;
        let claims = SessionClaims {
            user_id: session.user_id,
            epoch: session.epoch,
            expires_at,
        };
        let token = token::seal(&cipher, &claims, &secret)?;
        let reference = secret.reference();
        let (browser, platform) = session.device.codes();
        let row = NewRow {
            token_hash: secret.lookup_hash(),
            public_id: reference.0.clone(),
            user_id: session.user_id,
            created_at,
            last_seen_at: seconds(last_seen_hour(now))?,
            expires_at: stored_expiry,
            browser,
            platform,
        };
        bounded(DB::insert(&self.pool, row, MAX_SESSIONS_PER_USER)).await?;
        Ok(IssuedSession {
            token,
            reference,
            expires_at,
            max_age_seconds,
        })
    }

    /// [`create_session`](Self::create_session) with `APP_KEY` and the
    /// system clock; returns the `Set-Cookie` value, `Secure` outside
    /// development as [`make_login_cookie_with_epoch`](crate::make_login_cookie_with_epoch)
    /// sets it.
    pub async fn sign_in(&self, session: &NewSession) -> Result<String, SessionRegistryError> {
        let app_key = get_app_key().map_err(|_| SessionRegistryError::Configuration)?;
        let secure = detect_environment()
            .map_err(|_| SessionRegistryError::Configuration)?
            .requires_secure_defaults();
        let issued = self.create_session(session, &app_key, unix_now()?).await?;
        Ok(issued.set_cookie(secure))
    }

    /// The live session a `rullst_session` cookie value names.
    ///
    /// Opens the token, looks its record up by primary key and moves the
    /// record's last-seen hour forward when a new hour started. Check the
    /// session epoch of [`AuthenticatedSession::claims`] afterwards when the
    /// application keeps one.
    ///
    /// # Errors
    /// [`InvalidToken`](SessionRegistryError::InvalidToken),
    /// [`Expired`](SessionRegistryError::Expired) and
    /// [`Revoked`](SessionRegistryError::Revoked) mean signed out (see
    /// [`SessionRegistryError::is_signed_out`]); `Unavailable` and
    /// `Corrupt` never authenticate.
    pub async fn authenticate(
        &self,
        token: &str,
        app_key: &[u8],
        now: u64,
    ) -> Result<AuthenticatedSession, SessionRegistryError> {
        let cipher = derive_cipher(app_key).map_err(|_| SessionRegistryError::Configuration)?;
        let (claims, secret) = token::open(&cipher, token, Some(now))?;
        let token_hash = secret.lookup_hash();
        let row = bounded(DB::find(&self.pool, &token_hash))
            .await?
            .ok_or(SessionRegistryError::Revoked)?;
        let stored = Stored::decode(row)?;
        if !bool::from(stored.token_hash.as_bytes().ct_eq(token_hash.as_bytes()))
            || stored.user_id != claims.user_id
        {
            return Err(SessionRegistryError::Revoked);
        }
        if now > stored.expires_at {
            return Err(SessionRegistryError::Expired);
        }
        let hour = last_seen_hour(now);
        if hour > stored.last_seen_at {
            bounded(DB::touch(&self.pool, &token_hash, seconds(hour)?)).await?;
        }
        Ok(AuthenticatedSession {
            claims,
            current: CurrentSession {
                user_id: claims.user_id,
                token_hash,
                reference: stored.reference,
                created_at: stored.created_at,
                expires_at: stored.expires_at,
            },
        })
    }

    /// [`authenticate`](Self::authenticate) of the request's
    /// `rullst_session` cookie with `APP_KEY` and the system clock. A
    /// request without the cookie fails with
    /// [`InvalidToken`](SessionRegistryError::InvalidToken).
    pub async fn authenticate_request(
        &self,
        headers: &HeaderMap,
    ) -> Result<AuthenticatedSession, SessionRegistryError> {
        let token = extract_session_cookie(headers).ok_or(SessionRegistryError::InvalidToken)?;
        let app_key = get_app_key().map_err(|_| SessionRegistryError::Configuration)?;
        self.authenticate(&token, &app_key, unix_now()?).await
    }

    /// Signs out this session everywhere: every copy of its cookie stops
    /// working. Returns whether a record was deleted.
    pub async fn revoke(&self, current: &CurrentSession) -> Result<bool, SessionRegistryError> {
        let deleted = bounded(DB::delete_token(
            &self.pool,
            current.user_id,
            &current.token_hash,
        ))
        .await?;
        announce(Revoked::One {
            user_id: current.user_id,
            reference: current.reference.clone(),
        });
        Ok(deleted > 0)
    }

    /// Deletes the record of a registered-session token, expired or not;
    /// returns whether one was deleted.
    ///
    /// # Errors
    /// [`InvalidToken`](SessionRegistryError::InvalidToken) for a value that
    /// is not a registered-session token of `app_key`.
    pub async fn revoke_token(
        &self,
        token: &str,
        app_key: &[u8],
    ) -> Result<bool, SessionRegistryError> {
        let cipher = derive_cipher(app_key).map_err(|_| SessionRegistryError::Configuration)?;
        let (claims, secret) = token::open(&cipher, token, None)?;
        let deleted = bounded(DB::delete_token(
            &self.pool,
            claims.user_id,
            &secret.lookup_hash(),
        ))
        .await?;
        announce(Revoked::One {
            user_id: claims.user_id,
            reference: secret.reference(),
        });
        Ok(deleted > 0)
    }

    /// The sign-out of a request: [`revoke_token`](Self::revoke_token) of
    /// its `rullst_session` cookie with `APP_KEY`. `Ok(false)` without the
    /// cookie. Clear the browser's cookie too
    /// ([`make_logout_cookie`](crate::make_logout_cookie)).
    pub async fn sign_out(&self, headers: &HeaderMap) -> Result<bool, SessionRegistryError> {
        let Some(token) = extract_session_cookie(headers) else {
            return Ok(false);
        };
        let app_key = get_app_key().map_err(|_| SessionRegistryError::Configuration)?;
        self.revoke_token(&token, &app_key).await
    }

    /// The live sessions of `user_id`, most recently used first: at most
    /// [`MAX_SESSIONS_PER_USER`]. Pass the signed-in account's id, never an
    /// id from the request, and serve the page with `Cache-Control:
    /// no-store`.
    pub async fn list_sessions(
        &self,
        user_id: i32,
        now: u64,
    ) -> Result<Vec<SessionInfo>, SessionRegistryError> {
        let rows = bounded(DB::list(
            &self.pool,
            user_id,
            seconds(now)?,
            MAX_SESSIONS_PER_USER,
        ))
        .await?;
        rows.into_iter()
            .map(|row| {
                Stored::decode(row).map(|stored| SessionInfo {
                    reference: stored.reference,
                    created_at: stored.created_at,
                    last_seen_at: stored.last_seen_at,
                    expires_at: stored.expires_at,
                    device: stored.device,
                })
            })
            .collect()
    }

    /// Signs out one session of `user_id`, the signed-in account; a
    /// reference of another account's session revokes nothing. Returns
    /// whether a session was revoked.
    pub async fn revoke_session(
        &self,
        user_id: i32,
        session: &SessionRef,
    ) -> Result<bool, SessionRegistryError> {
        let deleted = bounded(DB::delete_reference(&self.pool, user_id, session.as_str())).await?;
        announce(Revoked::One {
            user_id,
            reference: session.clone(),
        });
        Ok(deleted > 0)
    }

    /// Signs out every session of the account except `current`; returns how
    /// many were revoked.
    pub async fn revoke_other_sessions(
        &self,
        current: &CurrentSession,
    ) -> Result<u64, SessionRegistryError> {
        let deleted = bounded(DB::delete_others(
            &self.pool,
            current.user_id,
            &current.token_hash,
        ))
        .await?;
        announce(Revoked::Others {
            user_id: current.user_id,
            kept: current.reference.clone(),
        });
        Ok(deleted)
    }

    /// Signs out every session of `user_id`: call it on a password reset,
    /// on "sign out everywhere" and when the account is deactivated or
    /// deleted (an account deleted with SQL needs the foreign key described
    /// on [`create_table`](Self::create_table)). Returns how many were
    /// revoked.
    pub async fn revoke_all_sessions(&self, user_id: i32) -> Result<u64, SessionRegistryError> {
        let deleted = bounded(DB::delete_user(&self.pool, user_id)).await?;
        announce(Revoked::All { user_id });
        Ok(deleted)
    }

    /// [`revoke_all_sessions`](Self::revoke_all_sessions) inside the
    /// caller's transaction (`&mut *transaction` of an SQLx transaction on
    /// the registry's database), so the revocation commits or rolls back
    /// with the rest of it, for example with a password removal. Returns how
    /// many were deleted.
    pub async fn revoke_all_sessions_on(
        connection: &mut DB::Connection,
        user_id: i32,
    ) -> Result<u64, SessionRegistryError> {
        let deleted = bounded(DB::delete_user_on(connection, user_id)).await?;
        // A notice before the commit only makes open checks look again.
        announce(Revoked::All { user_id });
        Ok(deleted)
    }

    /// Deletes at most `limit` (1 to [`MAX_PURGE_BATCH`]) sessions that
    /// expired before `now`, in one transaction; returns how many. Schedule
    /// it with trusted time; sign-ins also delete their account's expired
    /// sessions.
    pub async fn purge_expired(&self, now: u64, limit: u32) -> Result<u64, SessionRegistryError> {
        if !(1..=MAX_PURGE_BATCH).contains(&limit) {
            return Err(SessionRegistryError::InvalidInput("limit"));
        }
        bounded(DB::purge(&self.pool, seconds(now)?, limit)).await
    }
}

impl<DB: SessionDatabase> Clone for SessionRegistry<DB> {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
        }
    }
}

impl<DB: SessionDatabase> fmt::Debug for SessionRegistry<DB> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SessionRegistry")
            .field("database", &"[REDACTED]")
            .finish()
    }
}
