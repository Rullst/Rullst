//! `paginate()` obeys the global `max_query_limit` exactly like `limit()`.
//!
//! This binary owns its process-global ORM and query limit.
#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use rullst_orm::{FromRow, Orm};

#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(table = "paginated_rows")]
struct PaginatedRow {
    id: i32,
    label: String,
}

#[tokio::test]
async fn paginate_clamps_per_page_to_the_global_query_limit() {
    let database_path = std::env::temp_dir().join(format!(
        "rullst-paginate-limit-{}-{}.db",
        std::process::id(),
        rand::random::<u64>()
    ));
    Orm::init(&format!("sqlite:{}?mode=rwc", database_path.display()))
        .await
        .expect("initialize SQLite pagination database");
    let pool = Orm::pool().expect("SQLite pool");
    sqlx::query("CREATE TABLE paginated_rows (id INTEGER PRIMARY KEY, label TEXT NOT NULL)")
        .execute(pool)
        .await
        .expect("create pagination fixture");
    for id in 1..=12_i32 {
        sqlx::query("INSERT INTO paginated_rows (id, label) VALUES (?, ?)")
            .bind(id)
            .bind(format!("row-{id}"))
            .execute(pool)
            .await
            .expect("seed pagination fixture");
    }

    Orm::set_max_query_limit(5);

    let first = PaginatedRow::query()
        .order_by("id")
        .paginate(1, 5_000_000)
        .await
        .expect("oversized per_page is clamped, not rejected");
    assert_eq!(first.data.len(), 5, "per_page must not exceed the cap");
    assert_eq!(first.per_page, 5, "the result reports the effective size");
    assert_eq!(first.total, 12);
    assert_eq!(first.current_page, 1);
    assert_eq!(first.last_page, 3, "last_page follows the clamped size");
    assert_eq!(first.data[0].id, 1);
    assert_eq!(first.data[4].label, "row-5");

    let last = PaginatedRow::query()
        .order_by("id")
        .paginate(3, usize::MAX)
        .await
        .expect("page offsets use the clamped size");
    assert_eq!(
        last.data.iter().map(|row| row.id).collect::<Vec<_>>(),
        vec![11, 12]
    );
    assert_eq!(last.per_page, 5);

    let small = PaginatedRow::query()
        .order_by("id")
        .paginate(2, 3)
        .await
        .expect("a per_page below the cap is unchanged");
    assert_eq!(
        small.data.iter().map(|row| row.id).collect::<Vec<_>>(),
        vec![4, 5, 6]
    );
    assert_eq!(small.per_page, 3);
    assert_eq!(small.last_page, 4);

    Orm::set_max_query_limit(0);
    let unlimited = PaginatedRow::query()
        .order_by("id")
        .paginate(1, 50)
        .await
        .expect("a disabled cap leaves per_page unbounded");
    assert_eq!(unlimited.data.len(), 12);
    assert_eq!(unlimited.per_page, 50);
    assert_eq!(unlimited.last_page, 1);

    pool.close().await;
    let _ = std::fs::remove_file(database_path);
}
