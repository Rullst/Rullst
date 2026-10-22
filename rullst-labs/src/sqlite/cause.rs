//! The failure behind the last `LabError::Storage` of the current thread.
//!
//! `Storage` deliberately carries nothing, so a test fixture reads this
//! diagnostic to report why a store failed. It holds a SQLite result code, an
//! I/O error kind or a pool condition: never a message, SQL text or path.

use std::cell::Cell;

/// A secret-free class of the SQLx error that became `LabError::Storage`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StorageCause {
    /// Acquiring or establishing a pooled connection outlived its wait.
    PoolTimedOut,
    /// The pool was closed.
    PoolClosed,
    /// SQLite's extended result code, such as 5 for `SQLITE_BUSY`.
    Sqlite(i32),
    /// An operating-system I/O error.
    Io(std::io::ErrorKind),
    /// Any other SQLx error.
    Other,
}

impl StorageCause {
    pub(crate) fn of(error: &sqlx::Error) -> Self {
        match error {
            sqlx::Error::PoolTimedOut => Self::PoolTimedOut,
            sqlx::Error::PoolClosed => Self::PoolClosed,
            sqlx::Error::Database(error) => error
                .code()
                .and_then(|code| code.parse().ok())
                .map_or(Self::Other, Self::Sqlite),
            sqlx::Error::Io(error) => Self::Io(error.kind()),
            _ => Self::Other,
        }
    }
}

thread_local! {
    static LAST: Cell<Option<StorageCause>> = const { Cell::new(None) };
}

/// Remembers `error` as the current thread's last storage failure.
pub(super) fn record(error: &sqlx::Error) {
    LAST.with(|last| last.set(Some(StorageCause::of(error))));
}

/// The current thread's last storage failure. A `#[tokio::test]` polls the
/// store on its own thread, so its fixtures can report it.
#[cfg(test)]
pub(crate) fn last() -> Option<StorageCause> {
    LAST.with(Cell::get)
}

#[cfg(test)]
mod tests {
    use super::{StorageCause, last, record};
    use sqlx::{Connection, SqliteConnection};

    #[tokio::test]
    async fn causes_keep_codes_and_kinds_but_no_text() {
        assert_eq!(
            StorageCause::of(&sqlx::Error::PoolTimedOut),
            StorageCause::PoolTimedOut
        );
        assert_eq!(
            StorageCause::of(&sqlx::Error::PoolClosed),
            StorageCause::PoolClosed
        );
        let io = std::io::Error::from(std::io::ErrorKind::PermissionDenied);
        assert_eq!(
            StorageCause::of(&sqlx::Error::Io(io)),
            StorageCause::Io(std::io::ErrorKind::PermissionDenied)
        );
        assert_eq!(
            StorageCause::of(&sqlx::Error::RowNotFound),
            StorageCause::Other
        );
        let mut connection = SqliteConnection::connect("sqlite::memory:").await.unwrap();
        sqlx::query("CREATE TABLE t (v INTEGER UNIQUE)")
            .execute(&mut connection)
            .await
            .unwrap();
        let duplicate = sqlx::query("INSERT INTO t VALUES (1),(1)")
            .execute(&mut connection)
            .await
            .unwrap_err();
        // SQLITE_CONSTRAINT_UNIQUE, an extended result code.
        assert_eq!(StorageCause::of(&duplicate), StorageCause::Sqlite(2067));
    }

    #[test]
    fn the_last_cause_belongs_to_the_recording_thread() {
        record(&sqlx::Error::PoolClosed);
        record(&sqlx::Error::PoolTimedOut);
        assert_eq!(last(), Some(StorageCause::PoolTimedOut));
        let elsewhere = std::thread::spawn(last).join().unwrap();
        assert_eq!(elsewhere, None);
    }
}
