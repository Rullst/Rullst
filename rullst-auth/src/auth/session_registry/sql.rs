//! The registry's SQL, shared by every database: one generic implementation
//! over the per-database statements of `sqlite.rs`, `postgres.rs` and
//! `mysql.rs`.

use super::SessionRegistryError;
use sqlx::{ColumnIndex, Database, Decode, Encode, Executor, IntoArguments, Pool, Type};
use std::marker::PhantomData;

/// The statements of one database. Placeholders follow its dialect; every
/// statement binds its values in the order its comment gives.
pub(crate) struct Statements {
    /// Starts the write transaction of a sign-in.
    pub(crate) begin: &'static str,
    /// Creates the table and its indexes when they are missing.
    pub(crate) schema: &'static [&'static str],
    /// Reads nothing; fails when the table is missing.
    pub(crate) probe: &'static str,
    /// `user_id`, `now`: deletes the account's expired sessions.
    pub(crate) prune_user: &'static str,
    /// `user_id`: the account's session count.
    pub(crate) count_user: &'static str,
    /// `user_id`, `limit`: the account's least recently used sessions.
    pub(crate) oldest_user: &'static str,
    /// `token_hash`, `public_id`, `user_id`, `created_at`, `last_seen_at`,
    /// `expires_at`, `browser`, `platform`.
    pub(crate) insert: &'static str,
    /// `token_hash`: the session row.
    pub(crate) find: &'static str,
    /// `last_seen_at`, `token_hash`, `last_seen_at`: moves the last-seen
    /// hour forward.
    pub(crate) touch: &'static str,
    /// `token_hash`: deletes one session.
    pub(crate) delete_hash: &'static str,
    /// `user_id`, `token_hash`: deletes one session of the account.
    pub(crate) delete_token: &'static str,
    /// `user_id`, `public_id`: deletes one session by its reference.
    pub(crate) delete_reference: &'static str,
    /// `user_id`, `token_hash`: deletes every other session of the account.
    pub(crate) delete_others: &'static str,
    /// `user_id`: deletes every session of the account.
    pub(crate) delete_user: &'static str,
    /// `user_id`, `now`, `limit`: the account's live sessions, most recently
    /// used first.
    pub(crate) list_user: &'static str,
    /// `now`, `limit`: expired sessions, oldest expiry first.
    pub(crate) expired: &'static str,
}

/// A row to insert.
pub struct NewRow {
    pub(crate) token_hash: String,
    pub(crate) public_id: String,
    pub(crate) user_id: i32,
    pub(crate) created_at: i64,
    pub(crate) last_seen_at: i64,
    pub(crate) expires_at: i64,
    pub(crate) browser: &'static str,
    pub(crate) platform: &'static str,
}

/// A stored row: `token_hash`, `public_id`, `user_id`, `created_at`,
/// `last_seen_at`, `expires_at`, `browser`, `platform`.
pub(crate) type StoredRow = (String, String, i32, i64, i64, i64, String, String);

pub(crate) mod sealed {
    use super::{NewRow, SessionRegistryError, StoredRow};
    use sqlx::Pool;
    use std::future::Future;

    /// The per-database operations behind
    /// [`SessionRegistry`](super::super::SessionRegistry).
    pub trait Backend: sqlx::Database {
        fn prepare(
            pool: &Pool<Self>,
        ) -> impl Future<Output = Result<(), SessionRegistryError>> + Send;

        fn probe(
            pool: &Pool<Self>,
        ) -> impl Future<Output = Result<(), SessionRegistryError>> + Send;

        fn delete_user_on(
            connection: &mut <Self as sqlx::Database>::Connection,
            user_id: i32,
        ) -> impl Future<Output = Result<u64, SessionRegistryError>> + Send;

        fn insert(
            pool: &Pool<Self>,
            row: NewRow,
            per_user: u32,
        ) -> impl Future<Output = Result<(), SessionRegistryError>> + Send;

        fn find(
            pool: &Pool<Self>,
            token_hash: &str,
        ) -> impl Future<Output = Result<Option<StoredRow>, SessionRegistryError>> + Send;

        fn touch(
            pool: &Pool<Self>,
            token_hash: &str,
            last_seen_at: i64,
        ) -> impl Future<Output = Result<(), SessionRegistryError>> + Send;

        fn delete_token(
            pool: &Pool<Self>,
            user_id: i32,
            token_hash: &str,
        ) -> impl Future<Output = Result<u64, SessionRegistryError>> + Send;

        fn delete_reference(
            pool: &Pool<Self>,
            user_id: i32,
            public_id: &str,
        ) -> impl Future<Output = Result<u64, SessionRegistryError>> + Send;

        fn delete_others(
            pool: &Pool<Self>,
            user_id: i32,
            keep_hash: &str,
        ) -> impl Future<Output = Result<u64, SessionRegistryError>> + Send;

        fn delete_user(
            pool: &Pool<Self>,
            user_id: i32,
        ) -> impl Future<Output = Result<u64, SessionRegistryError>> + Send;

        fn list(
            pool: &Pool<Self>,
            user_id: i32,
            now: i64,
            limit: u32,
        ) -> impl Future<Output = Result<Vec<StoredRow>, SessionRegistryError>> + Send;

        fn purge(
            pool: &Pool<Self>,
            now: i64,
            limit: u32,
        ) -> impl Future<Output = Result<u64, SessionRegistryError>> + Send;
    }
}

/// A database the registry stores sessions in: SQLite
/// (`session-registry-sqlite`), PostgreSQL (`session-registry-postgres`) or
/// MySQL/MariaDB (`session-registry-mysql`).
///
/// Sealed: the per-database SQL is crate-private.
pub trait SessionDatabase: sealed::Backend {}

/// How one database reports affected rows; `sqlx` has no common accessor.
pub(crate) trait Dialect: Database {
    const STATEMENTS: Statements;

    fn rows_affected(result: &Self::QueryResult) -> u64;
}

fn unavailable(_: sqlx::Error) -> SessionRegistryError {
    SessionRegistryError::Unavailable
}

/// The generic implementation, instantiated by each database's `Backend`.
pub(crate) struct Ops<DB>(PhantomData<DB>);

impl<DB> Ops<DB>
where
    DB: Dialect,
    for<'c> &'c mut <DB as Database>::Connection: Executor<'c, Database = DB>,
    <DB as Database>::Arguments: IntoArguments<DB>,
    i32: Type<DB> + for<'q> Encode<'q, DB> + for<'r> Decode<'r, DB>,
    i64: Type<DB> + for<'q> Encode<'q, DB> + for<'r> Decode<'r, DB>,
    String: Type<DB> + for<'q> Encode<'q, DB> + for<'r> Decode<'r, DB>,
    usize: ColumnIndex<<DB as Database>::Row>,
{
    const SQL: Statements = DB::STATEMENTS;

    pub(crate) async fn prepare(pool: &Pool<DB>) -> Result<(), SessionRegistryError> {
        let mut transaction = pool.begin().await.map_err(unavailable)?;
        for statement in Self::SQL.schema {
            sqlx::query::<DB>(*statement)
                .execute(&mut *transaction)
                .await
                .map_err(unavailable)?;
        }
        transaction.commit().await.map_err(unavailable)
    }

    pub(crate) async fn probe(pool: &Pool<DB>) -> Result<(), SessionRegistryError> {
        sqlx::query_as::<DB, (i64,)>(Self::SQL.probe)
            .fetch_one(pool)
            .await
            .map(|_| ())
            .map_err(unavailable)
    }

    pub(crate) async fn delete_user_on(
        connection: &mut <DB as Database>::Connection,
        user_id: i32,
    ) -> Result<u64, SessionRegistryError> {
        let result = sqlx::query::<DB>(Self::SQL.delete_user)
            .bind(user_id)
            .execute(&mut *connection)
            .await
            .map_err(unavailable)?;
        Ok(DB::rows_affected(&result))
    }

    /// Prunes the account's expired sessions, evicts its least recently
    /// used ones down to `per_user - 1` and inserts `row`, in one
    /// transaction.
    pub(crate) async fn insert(
        pool: &Pool<DB>,
        row: NewRow,
        per_user: u32,
    ) -> Result<(), SessionRegistryError> {
        let mut transaction = pool
            .begin_with(Self::SQL.begin)
            .await
            .map_err(unavailable)?;
        sqlx::query::<DB>(Self::SQL.prune_user)
            .bind(row.user_id)
            .bind(row.created_at)
            .execute(&mut *transaction)
            .await
            .map_err(unavailable)?;
        let (count,): (i64,) = sqlx::query_as::<DB, (i64,)>(Self::SQL.count_user)
            .bind(row.user_id)
            .fetch_one(&mut *transaction)
            .await
            .map_err(unavailable)?;
        // The sessions beyond `per_user - 1`, least recently used first; none
        // below the limit (`LIMIT 0`; a negative SQLite limit means none).
        let excess = (count - i64::from(per_user) + 1).max(0);
        let oldest: Vec<(String,)> = sqlx::query_as::<DB, (String,)>(Self::SQL.oldest_user)
            .bind(row.user_id)
            .bind(excess)
            .fetch_all(&mut *transaction)
            .await
            .map_err(unavailable)?;
        for (token_hash,) in oldest {
            sqlx::query::<DB>(Self::SQL.delete_hash)
                .bind(token_hash)
                .execute(&mut *transaction)
                .await
                .map_err(unavailable)?;
        }
        sqlx::query::<DB>(Self::SQL.insert)
            .bind(row.token_hash)
            .bind(row.public_id)
            .bind(row.user_id)
            .bind(row.created_at)
            .bind(row.last_seen_at)
            .bind(row.expires_at)
            .bind(row.browser.to_owned())
            .bind(row.platform.to_owned())
            .execute(&mut *transaction)
            .await
            .map_err(unavailable)?;
        transaction.commit().await.map_err(unavailable)
    }

    pub(crate) async fn find(
        pool: &Pool<DB>,
        token_hash: &str,
    ) -> Result<Option<StoredRow>, SessionRegistryError> {
        sqlx::query_as::<DB, StoredRow>(Self::SQL.find)
            .bind(token_hash.to_owned())
            .fetch_optional(pool)
            .await
            .map_err(unavailable)
    }

    pub(crate) async fn touch(
        pool: &Pool<DB>,
        token_hash: &str,
        last_seen_at: i64,
    ) -> Result<(), SessionRegistryError> {
        sqlx::query::<DB>(Self::SQL.touch)
            .bind(last_seen_at)
            .bind(token_hash.to_owned())
            .bind(last_seen_at)
            .execute(pool)
            .await
            .map(|_| ())
            .map_err(unavailable)
    }

    async fn delete(
        pool: &Pool<DB>,
        statement: &'static str,
        user_id: i32,
        text: Option<&str>,
    ) -> Result<u64, SessionRegistryError> {
        let query = sqlx::query::<DB>(statement).bind(user_id);
        let query = match text {
            Some(text) => query.bind(text.to_owned()),
            None => query,
        };
        let result = query.execute(pool).await.map_err(unavailable)?;
        Ok(DB::rows_affected(&result))
    }

    pub(crate) async fn delete_token(
        pool: &Pool<DB>,
        user_id: i32,
        token_hash: &str,
    ) -> Result<u64, SessionRegistryError> {
        Self::delete(pool, Self::SQL.delete_token, user_id, Some(token_hash)).await
    }

    pub(crate) async fn delete_reference(
        pool: &Pool<DB>,
        user_id: i32,
        public_id: &str,
    ) -> Result<u64, SessionRegistryError> {
        Self::delete(pool, Self::SQL.delete_reference, user_id, Some(public_id)).await
    }

    pub(crate) async fn delete_others(
        pool: &Pool<DB>,
        user_id: i32,
        keep_hash: &str,
    ) -> Result<u64, SessionRegistryError> {
        Self::delete(pool, Self::SQL.delete_others, user_id, Some(keep_hash)).await
    }

    pub(crate) async fn delete_user(
        pool: &Pool<DB>,
        user_id: i32,
    ) -> Result<u64, SessionRegistryError> {
        Self::delete(pool, Self::SQL.delete_user, user_id, None).await
    }

    pub(crate) async fn list(
        pool: &Pool<DB>,
        user_id: i32,
        now: i64,
        limit: u32,
    ) -> Result<Vec<StoredRow>, SessionRegistryError> {
        sqlx::query_as::<DB, StoredRow>(Self::SQL.list_user)
            .bind(user_id)
            .bind(now)
            .bind(i64::from(limit))
            .fetch_all(pool)
            .await
            .map_err(unavailable)
    }

    /// Deletes at most `limit` expired sessions in one transaction.
    pub(crate) async fn purge(
        pool: &Pool<DB>,
        now: i64,
        limit: u32,
    ) -> Result<u64, SessionRegistryError> {
        let mut transaction = pool
            .begin_with(Self::SQL.begin)
            .await
            .map_err(unavailable)?;
        let expired: Vec<(String,)> = sqlx::query_as::<DB, (String,)>(Self::SQL.expired)
            .bind(now)
            .bind(i64::from(limit))
            .fetch_all(&mut *transaction)
            .await
            .map_err(unavailable)?;
        let mut deleted = 0;
        for (token_hash,) in expired {
            let result = sqlx::query::<DB>(Self::SQL.delete_hash)
                .bind(token_hash)
                .execute(&mut *transaction)
                .await
                .map_err(unavailable)?;
            deleted += DB::rows_affected(&result);
        }
        transaction.commit().await.map_err(unavailable)?;
        Ok(deleted)
    }
}

/// Implements [`sealed::Backend`] for a database with [`Ops`].
macro_rules! backend {
    ($database:ty) => {
        impl $crate::auth::session_registry::sql::SessionDatabase for $database {}

        impl $crate::auth::session_registry::sql::sealed::Backend for $database {
            async fn prepare(
                pool: &sqlx::Pool<Self>,
            ) -> Result<(), $crate::auth::session_registry::SessionRegistryError> {
                $crate::auth::session_registry::sql::Ops::<Self>::prepare(pool).await
            }

            async fn probe(
                pool: &sqlx::Pool<Self>,
            ) -> Result<(), $crate::auth::session_registry::SessionRegistryError> {
                $crate::auth::session_registry::sql::Ops::<Self>::probe(pool).await
            }

            async fn delete_user_on(
                connection: &mut <Self as sqlx::Database>::Connection,
                user_id: i32,
            ) -> Result<u64, $crate::auth::session_registry::SessionRegistryError> {
                $crate::auth::session_registry::sql::Ops::<Self>::delete_user_on(
                    connection, user_id,
                )
                .await
            }

            async fn insert(
                pool: &sqlx::Pool<Self>,
                row: $crate::auth::session_registry::sql::NewRow,
                per_user: u32,
            ) -> Result<(), $crate::auth::session_registry::SessionRegistryError> {
                $crate::auth::session_registry::sql::Ops::<Self>::insert(pool, row, per_user).await
            }

            async fn find(
                pool: &sqlx::Pool<Self>,
                token_hash: &str,
            ) -> Result<
                Option<$crate::auth::session_registry::sql::StoredRow>,
                $crate::auth::session_registry::SessionRegistryError,
            > {
                $crate::auth::session_registry::sql::Ops::<Self>::find(pool, token_hash).await
            }

            async fn touch(
                pool: &sqlx::Pool<Self>,
                token_hash: &str,
                last_seen_at: i64,
            ) -> Result<(), $crate::auth::session_registry::SessionRegistryError> {
                $crate::auth::session_registry::sql::Ops::<Self>::touch(
                    pool,
                    token_hash,
                    last_seen_at,
                )
                .await
            }

            async fn delete_token(
                pool: &sqlx::Pool<Self>,
                user_id: i32,
                token_hash: &str,
            ) -> Result<u64, $crate::auth::session_registry::SessionRegistryError> {
                $crate::auth::session_registry::sql::Ops::<Self>::delete_token(
                    pool, user_id, token_hash,
                )
                .await
            }

            async fn delete_reference(
                pool: &sqlx::Pool<Self>,
                user_id: i32,
                public_id: &str,
            ) -> Result<u64, $crate::auth::session_registry::SessionRegistryError> {
                $crate::auth::session_registry::sql::Ops::<Self>::delete_reference(
                    pool, user_id, public_id,
                )
                .await
            }

            async fn delete_others(
                pool: &sqlx::Pool<Self>,
                user_id: i32,
                keep_hash: &str,
            ) -> Result<u64, $crate::auth::session_registry::SessionRegistryError> {
                $crate::auth::session_registry::sql::Ops::<Self>::delete_others(
                    pool, user_id, keep_hash,
                )
                .await
            }

            async fn delete_user(
                pool: &sqlx::Pool<Self>,
                user_id: i32,
            ) -> Result<u64, $crate::auth::session_registry::SessionRegistryError> {
                $crate::auth::session_registry::sql::Ops::<Self>::delete_user(pool, user_id).await
            }

            async fn list(
                pool: &sqlx::Pool<Self>,
                user_id: i32,
                now: i64,
                limit: u32,
            ) -> Result<
                Vec<$crate::auth::session_registry::sql::StoredRow>,
                $crate::auth::session_registry::SessionRegistryError,
            > {
                $crate::auth::session_registry::sql::Ops::<Self>::list(pool, user_id, now, limit)
                    .await
            }

            async fn purge(
                pool: &sqlx::Pool<Self>,
                now: i64,
                limit: u32,
            ) -> Result<u64, $crate::auth::session_registry::SessionRegistryError> {
                $crate::auth::session_registry::sql::Ops::<Self>::purge(pool, now, limit).await
            }
        }
    };
}
pub(crate) use backend;
