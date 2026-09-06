use crate::{Clock, MessagingError, Result};
use sqlx::{Sqlite, Transaction};

use super::SqliteBroker;

#[cfg(test)]
mod tests {
    use super::super::storage::StorageProfile;
    use super::*;
    use crate::{BrokerConfig, SystemClock};
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn cancelled_write_rolls_back_before_returning_connection_to_pool() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query("CREATE TABLE cancellation_probe (value INTEGER NOT NULL)")
            .execute(&pool)
            .await
            .unwrap();
        let broker = SqliteBroker {
            config: BrokerConfig::try_new("audit-cancellation").unwrap(),
            clock: SystemClock,
            pool: pool.clone(),
            storage: StorageProfile::plaintext(),
        };
        let (signal, ready) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            let mut connection = broker.begin_write("cancellation regression").await.unwrap();
            sqlx::query("INSERT INTO cancellation_probe VALUES (1)")
                .execute(&mut *connection)
                .await
                .unwrap();
            signal.send(()).unwrap();
            std::future::pending::<()>().await;
        });
        ready.await.unwrap();
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM cancellation_probe")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(
            count.0, 0,
            "a canceled broker write must not leak uncommitted state to the next pool borrower"
        );
        let mut next = pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .expect("subsequent transaction starts cleanly");
        sqlx::query("INSERT INTO cancellation_probe VALUES (2)")
            .execute(&mut *next)
            .await
            .unwrap();
        next.commit().await.unwrap();
        pool.close().await;
    }
}

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
    ) -> Result<Transaction<'static, Sqlite>> {
        self.pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|_| storage_error(operation))
    }
}

pub(super) async fn finish<T>(
    connection: Transaction<'static, Sqlite>,
    result: Result<T>,
    operation: &'static str,
) -> Result<T> {
    match result {
        Ok(value) => {
            connection
                .commit()
                .await
                .map_err(|_| storage_error(operation))?;
            Ok(value)
        }
        Err(error) => {
            connection
                .rollback()
                .await
                .map_err(|_| storage_error(operation))?;
            Err(error)
        }
    }
}
