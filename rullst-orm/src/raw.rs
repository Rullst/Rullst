use crate::{Error, RullstValue};

pub struct RawQueryBuilder {
    pub sql: String,
    pub bindings: Vec<RullstValue>,
}

impl RawQueryBuilder {
    pub fn new(sql: &str) -> Self {
        Self {
            sql: sql.to_string(),
            bindings: Vec::new(),
        }
    }

    pub fn bind<T: Into<RullstValue>>(mut self, value: T) -> Self {
        self.bindings.push(value.into());
        self
    }

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
            match binding {
                RullstValue::String(s) => {
                    query = query.bind(s.clone());
                }
                RullstValue::Int(i) => {
                    query = query.bind(*i);
                }
                RullstValue::Float(f) => {
                    query = query.bind(*f);
                }
                RullstValue::Bool(b) => {
                    query = query.bind(*b);
                }
            }
        }

        let results = crate::execute_query!(query, fetch_all, read_pool)
            .map_err(|e| Error::DatabaseError(e.to_string()))?;
        Ok(results)
    }

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
            match binding {
                RullstValue::String(s) => {
                    query = query.bind(s.clone());
                }
                RullstValue::Int(i) => {
                    query = query.bind(*i);
                }
                RullstValue::Float(f) => {
                    query = query.bind(*f);
                }
                RullstValue::Bool(b) => {
                    query = query.bind(*b);
                }
            }
        }

        let result = crate::execute_query!(query, execute, pool)
            .map_err(|e| Error::DatabaseError(e.to_string()))?;
        Ok(result.rows_affected())
    }
}
