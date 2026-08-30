//! Transaction spans and outcomes for the global ORM facade.

use super::Orm;

type SharedTransaction =
    std::sync::Arc<tokio::sync::Mutex<Option<crate::db::Transaction<'static>>>>;

impl Orm {
    /// Opens a transaction under a secret-free tracing span.
    #[tracing::instrument(
        name = "rullst.orm.transaction.begin",
        target = "rullst_orm",
        fields(orm.driver = tracing::field::Empty)
    )]
    pub async fn begin_transaction() -> Result<crate::db::Transaction<'static>, crate::Error> {
        Self::record_driver();
        let pool = Self::pool()?;
        pool.begin().await.map_err(Into::into)
    }

    /// Executes a closure inside an isolated transaction and records its final
    /// commit or rollback outcome without recording SQL, bindings, or errors.
    #[tracing::instrument(
        name = "rullst.orm.transaction",
        target = "rullst_orm",
        skip(f),
        fields(
            orm.driver = tracing::field::Empty,
            orm.outcome = tracing::field::Empty
        )
    )]
    pub async fn transaction<F, R, E>(f: F) -> Result<R, crate::Error>
    where
        F: FnOnce(
                SharedTransaction,
            )
                -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<R, E>> + Send>>
            + Send,
        E: std::fmt::Display,
    {
        Self::record_driver();
        let tx = Self::begin_transaction().await?;
        let tx_arc = std::sync::Arc::new(tokio::sync::Mutex::new(Some(tx)));
        let post_commit = crate::post_commit::PostCommitScope::new();
        let result = post_commit
            .run(crate::CURRENT_TX.scope(tx_arc.clone(), f(tx_arc.clone())))
            .await;

        match result {
            Ok(value) => Self::commit_transaction(tx_arc, post_commit, value).await,
            Err(error) => Self::rollback_transaction(tx_arc, error).await,
        }
    }

    fn record_driver() {
        if let Ok(driver) = Self::try_driver() {
            tracing::Span::current().record("orm.driver", driver);
        }
    }

    async fn commit_transaction<R>(
        transaction: SharedTransaction,
        post_commit: crate::post_commit::PostCommitScope,
        value: R,
    ) -> Result<R, crate::Error> {
        let transaction = transaction.lock().await.take().ok_or_else(|| {
            tracing::Span::current().record("orm.outcome", "commit_ownership_missing");
            crate::Error::Internal(
                "managed transaction ownership was removed before automatic commit".to_string(),
            )
        })?;
        if let Err(error) = transaction.commit().await {
            tracing::Span::current().record("orm.outcome", "commit_failed");
            return Err(error.into());
        }
        if let Err(error) = post_commit.commit().await {
            tracing::Span::current().record("orm.outcome", "committed_post_commit_failed");
            return Err(error);
        }
        tracing::Span::current().record("orm.outcome", "committed");
        Ok(value)
    }

    async fn rollback_transaction<R, E>(
        transaction: SharedTransaction,
        error: E,
    ) -> Result<R, crate::Error>
    where
        E: std::fmt::Display,
    {
        let Some(transaction) = transaction.lock().await.take() else {
            tracing::Span::current().record("orm.outcome", "rollback_ownership_missing");
            return Err(crate::Error::Internal(
                "managed transaction ownership was removed before automatic rollback".to_string(),
            ));
        };
        if let Err(rollback_error) = transaction.rollback().await {
            tracing::Span::current().record("orm.outcome", "rollback_failed");
            return Err(crate::Error::DatabaseError(format!(
                "Transaction failed: {error}; rollback also failed: {rollback_error}",
            )));
        }
        tracing::Span::current().record("orm.outcome", "rolled_back");
        Err(crate::Error::DatabaseError(format!(
            "Transaction failed: {error}",
        )))
    }
}
