use crate::database::RullstDatabase;

/// Re-exporting Transaction so users use `rullst_orm::db::Transaction`
pub type Transaction<'a> = sqlx::Transaction<'a, RullstDatabase>;

/// Re-exporting Pool for connection pool usage
#[cfg(not(any(
    feature = "strict-postgres",
    feature = "strict-mysql",
    feature = "strict-sqlite"
)))]
pub type Pool = sqlx::AnyPool;

#[cfg(feature = "strict-postgres")]
pub type Pool = sqlx::PgPool;

#[cfg(all(feature = "strict-mysql", not(feature = "strict-postgres")))]
pub type Pool = sqlx::MySqlPool;

#[cfg(all(
    feature = "strict-sqlite",
    not(feature = "strict-postgres"),
    not(feature = "strict-mysql")
))]
pub type Pool = sqlx::SqlitePool;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_types_exported() {
        let _ = std::any::type_name::<Transaction>();
        let _ = std::any::type_name::<Pool>();
    }
}
