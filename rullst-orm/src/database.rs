#[cfg(not(any(
    feature = "strict-postgres",
    feature = "strict-mysql",
    feature = "strict-sqlite"
)))]
pub type RullstDatabase = sqlx::Any;

#[cfg(feature = "strict-postgres")]
pub type RullstDatabase = sqlx::Postgres;

#[cfg(all(feature = "strict-mysql", not(feature = "strict-postgres")))]
pub type RullstDatabase = sqlx::MySql;

#[cfg(all(
    feature = "strict-sqlite",
    not(feature = "strict-postgres"),
    not(feature = "strict-mysql")
))]
pub type RullstDatabase = sqlx::Sqlite;

pub trait QueryResultExt {
    fn get_last_insert_id(&self) -> i64;
}

#[cfg(not(any(
    feature = "strict-postgres",
    feature = "strict-mysql",
    feature = "strict-sqlite"
)))]
impl QueryResultExt for sqlx::any::AnyQueryResult {
    #[cfg_attr(test, mutants::skip)]
    fn get_last_insert_id(&self) -> i64 {
        self.last_insert_id().unwrap_or(0)
    }
}

#[cfg(feature = "strict-postgres")]
impl QueryResultExt for sqlx::postgres::PgQueryResult {
    #[cfg_attr(test, mutants::skip)]
    fn get_last_insert_id(&self) -> i64 {
        0
    }
}

#[cfg(all(feature = "strict-mysql", not(feature = "strict-postgres")))]
impl QueryResultExt for sqlx::mysql::MySqlQueryResult {
    #[cfg_attr(test, mutants::skip)]
    fn get_last_insert_id(&self) -> i64 {
        self.last_insert_id() as i64
    }
}

#[cfg(all(
    feature = "strict-sqlite",
    not(feature = "strict-postgres"),
    not(feature = "strict-mysql")
))]
impl QueryResultExt for sqlx::sqlite::SqliteQueryResult {
    #[cfg_attr(test, mutants::skip)]
    fn get_last_insert_id(&self) -> i64 {
        self.last_insert_rowid()
    }
}
