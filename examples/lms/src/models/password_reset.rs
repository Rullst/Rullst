//! Single-use password reset tokens.
//!
//! The emailed token is never stored: `password_reset_tokens` keeps only its
//! SHA-256 digest with Unix-second times. The table's primary key is the
//! account, so each account has at most one live token and a new request
//! replaces the previous one. Redeeming a token deletes it, stores the new
//! password hash and bumps `session_epoch` in one transaction, which signs
//! out every device.
use rullst::auth::PasswordResetToken;
use rullst::db::{Orm, sqlx};

/// How long an emailed reset link stays valid, in minutes: from 10 to 1440
/// (24 hours), checked when the application compiles. The stored token, the
/// reset email and the forgot-password answer all follow it. Shorter links
/// can expire before delayed mail arrives; longer ones leave a working
/// account-recovery link in the mailbox for longer.
pub const RESET_TOKEN_LIFETIME_MINUTES: u32 = 30;
const _: () = assert!(
    matches!(RESET_TOKEN_LIFETIME_MINUTES, 10..=1440),
    "RESET_TOKEN_LIFETIME_MINUTES must be from 10 minutes to 24 hours (1440)"
);
/// A new request for the same account within this many seconds of the last
/// one sends nothing, so nobody can flood an inbox with reset mail.
pub const RESET_REQUEST_COOLDOWN_SECONDS: i64 = 60;

/// Why a reset token was not issued or redeemed.
#[derive(Debug)]
pub enum ResetError {
    /// The token is unknown, expired, already used or replaced by a newer
    /// request, or its account no longer exists.
    InvalidToken,
    /// The operating system's random number generator failed.
    Random(rullst::auth::AuthError),
    /// The database failed; nothing changed.
    Storage(rullst::orm::Error),
}

impl std::fmt::Display for ResetError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidToken => formatter.write_str("invalid or expired password reset token"),
            Self::Random(error) => write!(formatter, "reset token generation failed: {error}"),
            Self::Storage(error) => write!(formatter, "reset token storage failed: {error}"),
        }
    }
}

impl From<rullst::orm::Error> for ResetError {
    fn from(error: rullst::orm::Error) -> Self {
        Self::Storage(error)
    }
}

impl From<sqlx::Error> for ResetError {
    fn from(error: sqlx::Error) -> Self {
        Self::Storage(error.into())
    }
}

/// The current Unix time in seconds.
pub fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| {
            i64::try_from(elapsed.as_secs()).unwrap_or(i64::MAX)
        })
}

/// PostgreSQL numbers its bind markers; the other drivers take `?`.
fn dialect(sql: &str) -> Result<String, rullst::orm::Error> {
    Ok(if Orm::driver()? == "postgres" {
        rullst::orm::replace_placeholders(sql)
    } else {
        sql.to_owned()
    })
}

/// Issues a new token for the account and replaces its previous one, or
/// returns `None` when a token was issued less than
/// [`RESET_REQUEST_COOLDOWN_SECONDS`] ago.
pub async fn issue(user_id: i32, now: i64) -> Result<Option<PasswordResetToken>, ResetError> {
    let token = PasswordResetToken::generate().map_err(ResetError::Random)?;
    let latest = dialect("SELECT created_at FROM password_reset_tokens WHERE user_id = ?")?;
    let delete = dialect("DELETE FROM password_reset_tokens WHERE user_id = ?")?;
    let insert = dialect(
        "INSERT INTO password_reset_tokens (user_id, token_hash, expires_at, created_at) VALUES (?, ?, ?, ?)",
    )?;
    let expires_at = now.saturating_add(i64::from(RESET_TOKEN_LIFETIME_MINUTES) * 60);
    let mut transaction = Orm::begin_transaction().await?;
    let stored = async {
        let issued_at: Option<i64> = sqlx::query_scalar(sqlx::AssertSqlSafe(latest.as_str()))
            .bind(user_id)
            .fetch_optional(&mut *transaction)
            .await?;
        if issued_at
            .is_some_and(|issued_at| now.saturating_sub(issued_at) < RESET_REQUEST_COOLDOWN_SECONDS)
        {
            return Ok(false);
        }
        sqlx::query(sqlx::AssertSqlSafe(delete.as_str()))
            .bind(user_id)
            .execute(&mut *transaction)
            .await?;
        sqlx::query(sqlx::AssertSqlSafe(insert.as_str()))
            .bind(user_id)
            .bind(token.digest())
            .bind(expires_at)
            .bind(now)
            .execute(&mut *transaction)
            .await?;
        Ok::<bool, sqlx::Error>(true)
    }
    .await;
    match stored {
        Ok(true) => {
            transaction.commit().await?;
            Ok(Some(token))
        }
        Ok(false) => {
            transaction.rollback().await?;
            Ok(None)
        }
        Err(error) => {
            if let Err(rollback_error) = transaction.rollback().await {
                eprintln!("Reset token rollback failed: {rollback_error}");
            }
            Err(error.into())
        }
    }
}

/// Deletes the account's reset token, so a link requested before a password
/// change stops working.
pub async fn discard(user_id: i32) -> Result<(), ResetError> {
    let delete = dialect("DELETE FROM password_reset_tokens WHERE user_id = ?")?;
    sqlx::query(sqlx::AssertSqlSafe(delete.as_str()))
        .bind(user_id)
        .execute(Orm::pool()?)
        .await?;
    Ok(())
}

/// The account whose live token has this digest. The token stays unused.
pub async fn live_user(digest: &str, now: i64) -> Result<Option<i32>, ResetError> {
    let lookup = dialect(
        "SELECT user_id, token_hash, expires_at FROM password_reset_tokens WHERE token_hash = ?",
    )?;
    let row: Option<(i32, String, i64)> = sqlx::query_as(sqlx::AssertSqlSafe(lookup.as_str()))
        .bind(digest)
        .fetch_optional(Orm::pool()?)
        .await?;
    Ok(row
        .filter(|(_, stored, expires_at)| {
            *expires_at > now && rullst::auth::password_reset_digests_match(stored, digest)
        })
        .map(|(user_id, _, _)| user_id))
}

/// Redeems the account's token in one transaction: deletes it while it is
/// still live (so it works once), then stores the new password hash and bumps
/// `session_epoch`. A token that expired, was used or was replaced meanwhile
/// changes nothing and returns [`ResetError::InvalidToken`].
pub async fn redeem(
    user_id: i32,
    digest: &str,
    now: i64,
    password_hash: &str,
    updated_at: rullst::orm::chrono::DateTime<rullst::orm::chrono::Utc>,
) -> Result<(), ResetError> {
    let claim = dialect(
        "DELETE FROM password_reset_tokens WHERE user_id = ? AND token_hash = ? AND expires_at > ?",
    )?;
    let update = dialect(
        "UPDATE users SET password_hash = ?, updated_at = ?, session_epoch = session_epoch + 1 WHERE id = ?",
    )?;
    let mut transaction = Orm::begin_transaction().await?;
    let redeemed = async {
        let claimed = sqlx::query(sqlx::AssertSqlSafe(claim.as_str()))
            .bind(user_id)
            .bind(digest)
            .bind(now)
            .execute(&mut *transaction)
            .await?
            .rows_affected();
        if claimed != 1 {
            return Ok(false);
        }
        let updated = sqlx::query(sqlx::AssertSqlSafe(update.as_str()))
            .bind(password_hash)
            .bind(updated_at)
            .bind(user_id)
            .execute(&mut *transaction)
            .await?
            .rows_affected();
        Ok::<bool, sqlx::Error>(updated == 1)
    }
    .await;
    match redeemed {
        Ok(true) => {
            transaction.commit().await?;
            Ok(())
        }
        Ok(false) => {
            transaction.rollback().await?;
            Err(ResetError::InvalidToken)
        }
        Err(error) => {
            if let Err(rollback_error) = transaction.rollback().await {
                eprintln!("Password reset rollback failed: {rollback_error}");
            }
            Err(error.into())
        }
    }
}
