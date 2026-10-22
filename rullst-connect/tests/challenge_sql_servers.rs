//! The SQL login-challenge contract on owned PostgreSQL and MySQL servers.
//!
//! `.github/check-connect-sql.py` starts both servers, sets the URLs and runs
//! these ignored tests. Each test first drops the challenge table, so a rerun
//! or a mutation run starts from an empty database.
#![cfg(any(feature = "postgres", feature = "mysql"))]

mod challenge_sql_support;

fn url(variable: &str) -> String {
    std::env::var(variable).unwrap_or_else(|_| panic!("{variable} must name a disposable database"))
}

#[cfg(feature = "postgres")]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "needs RULLST_CONNECT_TEST_POSTGRES_URL; run .github/check-connect-sql.py"]
async fn postgres_challenge_contract() {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(8)
        .connect(&url("RULLST_CONNECT_TEST_POSTGRES_URL"))
        .await
        .unwrap();
    sqlx::query("DROP TABLE IF EXISTS rullst_connect_login_challenges")
        .execute(&pool)
        .await
        .unwrap();
    challenge_sql_support::exercise(pool.clone()).await;
    pool.close().await;
}

#[cfg(feature = "mysql")]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "needs RULLST_CONNECT_TEST_MYSQL_URL; run .github/check-connect-sql.py"]
async fn mysql_challenge_contract() {
    let pool = sqlx::mysql::MySqlPoolOptions::new()
        .max_connections(8)
        .connect(&url("RULLST_CONNECT_TEST_MYSQL_URL"))
        .await
        .unwrap();
    sqlx::query("DROP TABLE IF EXISTS rullst_connect_login_challenges")
        .execute(&pool)
        .await
        .unwrap();
    challenge_sql_support::exercise(pool.clone()).await;
    pool.close().await;
}
