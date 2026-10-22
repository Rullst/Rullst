//! SQLite: `BEGIN IMMEDIATE` saves and `DELETE … RETURNING` consumption.

use super::{
    ChallengeDatabase, ChallengeRow, count_from, decode, finish, sealed, seconds, unavailable,
};
use crate::challenge::{ChallengeError, LoginChallenge};
use sqlx::{Sqlite, SqliteConnection, SqlitePool};

const SCHEMA: [&str; 2] = [
    "CREATE TABLE IF NOT EXISTS rullst_connect_login_challenges (id_key TEXT PRIMARY KEY NOT NULL, provider TEXT NOT NULL, state TEXT NOT NULL, nonce TEXT, code_verifier TEXT NOT NULL, return_to TEXT, created_at INTEGER NOT NULL, expires_at INTEGER NOT NULL)",
    "CREATE INDEX IF NOT EXISTS rullst_connect_login_challenges_expiry ON rullst_connect_login_challenges (expires_at)",
];
const COUNT: &str = "SELECT COUNT(*) FROM rullst_connect_login_challenges";
const DELETE_EXPIRED: &str = "DELETE FROM rullst_connect_login_challenges WHERE expires_at <= ?";
const INSERT: &str = "INSERT INTO rullst_connect_login_challenges (id_key, provider, state, nonce, code_verifier, return_to, created_at, expires_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)";
const TAKE: &str = "DELETE FROM rullst_connect_login_challenges WHERE id_key = ? RETURNING provider, state, nonce, code_verifier, return_to, created_at, expires_at";
const PURGE: &str = "DELETE FROM rullst_connect_login_challenges WHERE id_key IN (SELECT id_key FROM rullst_connect_login_challenges WHERE expires_at <= ? ORDER BY expires_at LIMIT ?)";

impl sealed::Sealed for Sqlite {}

impl ChallengeDatabase for Sqlite {
    async fn prepare_challenges(pool: &SqlitePool) -> Result<(), ChallengeError> {
        let mut transaction = pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|_| unavailable("begin challenge schema"))?;
        let result = async {
            for statement in SCHEMA {
                sqlx::query(statement)
                    .execute(&mut *transaction)
                    .await
                    .map_err(|_| unavailable("prepare challenge schema"))?;
            }
            Ok(())
        }
        .await;
        finish(transaction, result, "finish challenge schema").await
    }

    async fn insert_challenge(
        pool: &SqlitePool,
        key: &str,
        challenge: &LoginChallenge,
        max_entries: u32,
    ) -> Result<(), ChallengeError> {
        let mut transaction = pool
            .begin_with("BEGIN IMMEDIATE")
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
        pool: &SqlitePool,
        key: &str,
    ) -> Result<Option<LoginChallenge>, ChallengeError> {
        let row: Option<ChallengeRow> = sqlx::query_as(TAKE)
            .bind(key)
            .fetch_optional(pool)
            .await
            .map_err(|_| unavailable("consume challenge"))?;
        row.map(decode).transpose()
    }

    async fn purge_challenges(
        pool: &SqlitePool,
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

    async fn count_challenges(pool: &SqlitePool) -> Result<u64, ChallengeError> {
        let mut connection = pool
            .acquire()
            .await
            .map_err(|_| unavailable("count challenges"))?;
        count(&mut connection).await
    }
}

async fn count(connection: &mut SqliteConnection) -> Result<u64, ChallengeError> {
    let (entries,): (i64,) = sqlx::query_as(COUNT)
        .fetch_one(connection)
        .await
        .map_err(|_| unavailable("count challenges"))?;
    count_from(entries)
}
