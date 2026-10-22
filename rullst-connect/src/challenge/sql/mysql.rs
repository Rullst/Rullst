//! MySQL and MariaDB: transactional saves, and consumption by
//! `SELECT … FOR UPDATE` then `DELETE` in one transaction (MySQL has no
//! `DELETE … RETURNING`).
//!
//! Exercised against an owned server by `.github/check-connect-sql.py`.

use super::{
    ChallengeDatabase, ChallengeRow, count_from, decode, finish, sealed, seconds, unavailable,
};
use crate::challenge::{ChallengeError, LoginChallenge};
use sqlx::{MySql, MySqlConnection, MySqlPool};

const SCHEMA: &str = "CREATE TABLE IF NOT EXISTS rullst_connect_login_challenges (id_key CHAR(64) NOT NULL PRIMARY KEY, provider VARCHAR(32) NOT NULL, state VARCHAR(128) NOT NULL, nonce VARCHAR(128) NULL, code_verifier VARCHAR(128) NOT NULL, return_to VARCHAR(1024) NULL, created_at BIGINT NOT NULL, expires_at BIGINT NOT NULL, INDEX rullst_connect_login_challenges_expiry (expires_at)) ENGINE=InnoDB";
const COUNT: &str = "SELECT COUNT(*) FROM rullst_connect_login_challenges";
const DELETE_EXPIRED: &str = "DELETE FROM rullst_connect_login_challenges WHERE expires_at <= ?";
const INSERT: &str = "INSERT INTO rullst_connect_login_challenges (id_key, provider, state, nonce, code_verifier, return_to, created_at, expires_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)";
const LOCK: &str = "SELECT provider, state, nonce, code_verifier, return_to, created_at, expires_at FROM rullst_connect_login_challenges WHERE id_key = ? FOR UPDATE";
const DELETE: &str = "DELETE FROM rullst_connect_login_challenges WHERE id_key = ?";
const PURGE: &str =
    "DELETE FROM rullst_connect_login_challenges WHERE expires_at <= ? ORDER BY expires_at LIMIT ?";

impl sealed::Sealed for MySql {}

impl ChallengeDatabase for MySql {
    async fn prepare_challenges(pool: &MySqlPool) -> Result<(), ChallengeError> {
        // MySQL commits DDL implicitly; IF NOT EXISTS keeps it idempotent.
        sqlx::query(SCHEMA)
            .execute(pool)
            .await
            .map_err(|_| unavailable("prepare challenge schema"))?;
        Ok(())
    }

    async fn insert_challenge(
        pool: &MySqlPool,
        key: &str,
        challenge: &LoginChallenge,
        max_entries: u32,
    ) -> Result<(), ChallengeError> {
        let mut transaction = pool
            .begin()
            .await
            .map_err(|_| unavailable("begin challenge insert"))?;
        let result = async {
            let connection = &mut *transaction;
            if count(connection).await? >= u64::from(max_entries) {
                sqlx::query(DELETE_EXPIRED)
                    .bind(seconds(challenge.created_at()))
                    .execute(&mut *connection)
                    .await
                    .map_err(|_| unavailable("purge expired challenges"))?;
                if count(connection).await? >= u64::from(max_entries) {
                    return Err(ChallengeError::CapacityExceeded);
                }
            }
            sqlx::query(INSERT)
                .bind(key)
                .bind(challenge.provider())
                .bind(challenge.state())
                .bind(challenge.nonce())
                .bind(challenge.code_verifier())
                .bind(challenge.return_to().map(|path| path.as_str()))
                .bind(seconds(challenge.created_at()))
                .bind(seconds(challenge.expires_at()))
                .execute(&mut *connection)
                .await
                .map_err(|_| unavailable("insert challenge"))?;
            Ok(())
        }
        .await;
        finish(transaction, result, "finish challenge insert").await
    }

    async fn remove_challenge(
        pool: &MySqlPool,
        key: &str,
    ) -> Result<Option<LoginChallenge>, ChallengeError> {
        let mut transaction = pool
            .begin()
            .await
            .map_err(|_| unavailable("begin challenge consumption"))?;
        let result = async {
            let connection = &mut *transaction;
            let row: Option<ChallengeRow> = sqlx::query_as(LOCK)
                .bind(key)
                .fetch_optional(&mut *connection)
                .await
                .map_err(|_| unavailable("lock challenge"))?;
            let Some(row) = row else {
                return Ok::<_, ChallengeError>(None);
            };
            let deleted = sqlx::query(DELETE)
                .bind(key)
                .execute(&mut *connection)
                .await
                .map_err(|_| unavailable("consume challenge"))?;
            Ok((deleted.rows_affected() == 1).then_some(row))
        }
        .await;
        // Like `DELETE … RETURNING`, the row stays consumed even if it is corrupt.
        finish(transaction, result, "finish challenge consumption")
            .await?
            .map(decode)
            .transpose()
    }

    async fn purge_challenges(
        pool: &MySqlPool,
        now: u64,
        limit: u32,
    ) -> Result<u64, ChallengeError> {
        let deleted = sqlx::query(PURGE)
            .bind(seconds(now))
            .bind(i64::from(limit))
            .execute(pool)
            .await
            .map_err(|_| unavailable("purge expired challenges"))?;
        Ok(deleted.rows_affected())
    }

    async fn count_challenges(pool: &MySqlPool) -> Result<u64, ChallengeError> {
        let mut connection = pool
            .acquire()
            .await
            .map_err(|_| unavailable("count challenges"))?;
        count(&mut connection).await
    }
}

async fn count(connection: &mut MySqlConnection) -> Result<u64, ChallengeError> {
    let (entries,): (i64,) = sqlx::query_as(COUNT)
        .fetch_one(connection)
        .await
        .map_err(|_| unavailable("count challenges"))?;
    count_from(entries)
}
