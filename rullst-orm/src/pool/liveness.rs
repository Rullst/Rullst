//! The check an `Orm::init*` pool runs on an idle connection before handing
//! it out.

use std::time::Duration;

use futures::future::BoxFuture;
use sqlx::pool::PoolConnectionMetadata;

use super::dsn;
use crate::RullstPoolOptions;
use crate::database::RullstDatabase;

type Connection = <RullstDatabase as sqlx::Database>::Connection;

/// How long a PostgreSQL or MySQL connection may sit idle in the pool and
/// still be handed out without a ping: the bypass window of HikariCP's
/// `aliveBypassWindowMs`.
const IDLE_PING_AFTER: Duration = Duration::from_millis(500);

/// Sets the check a pool of `database_url` runs before handing out an idle
/// connection.
///
/// SQLx pings every connection when it returns to the pool, and by default
/// again before handing it out (`test_before_acquire`), so a one-statement
/// request cost three round trips. That second ping is off for every pool:
///
/// - A PostgreSQL or MySQL connection is pinged only when it has been idle
///   for at least 500 ms. When the ping fails, the pool closes it and takes
///   another idle connection or opens a new one. A connection idle for less
///   passed the release ping less than 500 ms earlier. A database restart
///   takes longer than that, so once the server is back every connection it
///   ended has been idle for longer and is replaced; a longer window, such as
///   30 s, could hand out up to a pool's worth of dead connections after a
///   quick restart. A connection that dies less than 500 ms after its release
///   fails its next query instead, and SQLx closes it then.
/// - A SQLite connection is a file the process opened itself, not a socket
///   that a server or a network can close, so it is never pinged: the round
///   trip to its worker thread cost about as much as the query. A connection
///   that failed still fails its next query, and SQLx closes it then.
pub(super) fn check_before_acquire(
    options: RullstPoolOptions,
    database_url: &str,
) -> RullstPoolOptions {
    let options = options.test_before_acquire(false);
    if pings_idle_connections(database_url) {
        options.before_acquire(ping_when_idle)
    } else {
        options
    }
}

/// Whether a pool of `database_url` pings its idle connections: a PostgreSQL
/// or MySQL pool does, a SQLite pool never does.
pub(super) fn pings_idle_connections(database_url: &str) -> bool {
    dsn::driver_for_url(database_url) != "sqlite"
}

/// Whether a connection that has been idle for `idle_for` is pinged before
/// the pool hands it out.
pub(super) fn needs_idle_ping(idle_for: Duration) -> bool {
    idle_for >= IDLE_PING_AFTER
}

/// SQLx's `before_acquire` hook. `Ok(false)` makes the pool close the
/// connection and acquire another one.
fn ping_when_idle(
    connection: &mut Connection,
    metadata: PoolConnectionMetadata,
) -> BoxFuture<'_, Result<bool, sqlx::Error>> {
    Box::pin(async move {
        if needs_idle_ping(metadata.idle_for)
            && let Err(error) = sqlx::Connection::ping(connection).await
        {
            tracing::info!(%error, "closing an idle pooled connection whose ping failed");
            return Ok(false);
        }
        Ok(true)
    })
}
