use crate::{Clock, MessagingError, Result};
use sqlx::pool::PoolConnection;
use sqlx::{Executor, Sqlite};

use super::SqliteBroker;

pub(super) fn storage_error(operation: &'static str) -> MessagingError {
    MessagingError::StorageUnavailable { operation }
}

impl<C: Clock> SqliteBroker<C> {
    pub(super) fn now(&self) -> Result<i64> {
        let now = self.clock.now_millis()?;
        if now < 0 {
            return Err(MessagingError::ClockOutOfRange);
        }
        Ok(now)
    }

    pub(super) async fn begin_write(
        &self,
        operation: &'static str,
    ) -> Result<PoolConnection<Sqlite>> {
        let mut connection = self
            .pool
            .acquire()
            .await
            .map_err(|_| storage_error(operation))?;
        connection
            .execute("BEGIN IMMEDIATE")
            .await
            .map_err(|_| storage_error(operation))?;
        Ok(connection)
    }
}

pub(super) async fn commit(
    connection: &mut PoolConnection<Sqlite>,
    operation: &'static str,
) -> Result<()> {
    match connection.execute("COMMIT").await {
        Ok(_) => Ok(()),
        Err(_) => {
            connection.close_on_drop();
            Err(storage_error(operation))
        }
    }
}

pub(super) async fn rollback(
    connection: &mut PoolConnection<Sqlite>,
    operation: &'static str,
) -> Result<()> {
    match connection.execute("ROLLBACK").await {
        Ok(_) => Ok(()),
        Err(_) => {
            connection.close_on_drop();
            Err(storage_error(operation))
        }
    }
}

pub(super) async fn finish<T>(
    connection: &mut PoolConnection<Sqlite>,
    result: Result<T>,
    operation: &'static str,
) -> Result<T> {
    match result {
        Ok(value) => {
            commit(connection, operation).await?;
            Ok(value)
        }
        Err(error) => {
            rollback(connection, operation).await?;
            Err(error)
        }
    }
}
