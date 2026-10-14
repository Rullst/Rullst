use crate::value::BindRullstValue;
use crate::{Error, RullstValue};

pub struct RawQueryBuilder {
    pub sql: String,
    pub bindings: Vec<RullstValue>,
}

impl RawQueryBuilder {
    pub fn new(sql: impl Into<String>) -> Self {
        Self {
            sql: sql.into(),
            bindings: Vec::new(),
        }
    }

    pub fn bind<T: Into<RullstValue>>(mut self, value: T) -> Self {
        self.bindings.push(value.into());
        self
    }

    #[tracing::instrument(
        name = "rullst.orm.query",
        target = "rullst_orm",
        skip(self),
        fields(
            orm.operation = "raw.select",
            orm.binding_count = self.bindings.len()
        )
    )]
    pub async fn map_to<T>(&self) -> Result<Vec<T>, Error>
    where
        for<'r> T: crate::_sqlx::FromRow<'r, <crate::RullstDatabase as crate::_sqlx::Database>::Row>
            + Send
            + Unpin,
    {
        if crate::schema::is_query_log_enabled() {
            println!(
                "[SQL Debug] {:?} | Bindings: [{} parameter(s)]",
                self.sql,
                self.bindings.len()
            );
        }

        let mut query =
            crate::_sqlx::query_as::<_, T>(crate::_sqlx::AssertSqlSafe(self.sql.as_str()));
        for binding in &self.bindings {
            query = query.bind_rullst_value(binding);
        }

        // `?` maps constraint violations to their typed `Error` variants.
        let results = crate::execute_query!(query, fetch_all, read_pool)?;
        Ok(results)
    }

    #[tracing::instrument(
        name = "rullst.orm.query",
        target = "rullst_orm",
        skip(self),
        fields(
            orm.operation = "raw.execute",
            orm.binding_count = self.bindings.len()
        )
    )]
    pub async fn execute(&self) -> Result<u64, Error> {
        if crate::schema::is_query_log_enabled() {
            println!(
                "[SQL Debug] {:?} | Bindings: [{} parameter(s)]",
                self.sql,
                self.bindings.len()
            );
        }

        let mut query = crate::_sqlx::query(crate::_sqlx::AssertSqlSafe(self.sql.as_str()));
        for binding in &self.bindings {
            query = query.bind_rullst_value(binding);
        }

        let result = crate::execute_query!(query, execute, pool)?;
        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use crate::{Error, Orm};

    // No test of the library initializes the ORM, so these reach the pool.
    #[tokio::test]
    async fn raw_queries_report_an_uninitialized_orm() {
        let executed = Orm::raw("UPDATE t SET n = n + ?").bind(1).execute().await;
        assert!(
            matches!(executed, Err(Error::NotInitialized)),
            "{executed:?}"
        );
        let mapped = Orm::raw("SELECT 1").map_to::<(i32,)>().await;
        assert!(matches!(mapped, Err(Error::NotInitialized)), "{mapped:?}");
    }
}
