//! Offset reads without a row cap must stay valid SQL on every driver
//! (SQLite and MySQL/MariaDB accept `OFFSET` only after `LIMIT`), and offset
//! chunks follow a deterministic order.

use rullst_orm::schema::{Blueprint, Schema};
use rullst_orm::{FromRow, Orm};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "contract_offset_rows")]
struct ContractOffsetRow {
    id: i32,
    name: String,
}

pub(super) async fn exercise() {
    let driver = Orm::driver().expect("initialized driver");
    Schema::create("contract_offset_rows", |table: &mut Blueprint| {
        table.id();
        table.string("name").not_null();
    })
    .await
    .expect("create offset contract table");
    for name in ["e", "d", "c", "b", "a"] {
        let mut row = ContractOffsetRow {
            id: 0,
            name: name.to_string(),
        };
        row.save().await.expect("insert offset contract row");
    }

    let uncapped = ContractOffsetRow::query()
        .unsafe_unlimited()
        .order_by("id")
        .offset(2);
    let rows = uncapped
        .get()
        .await
        .unwrap_or_else(|error| panic!("{driver} uncapped offset get: {error}"));
    assert_eq!(
        rows.iter().map(|row| row.name.as_str()).collect::<Vec<_>>(),
        ["c", "b", "a"],
        "{driver}"
    );
    let names = uncapped
        .pluck_string("name")
        .await
        .unwrap_or_else(|error| panic!("{driver} uncapped offset pluck: {error}"));
    assert_eq!(names, ["c", "b", "a"], "{driver}");

    // Offset pages need a deterministic order. With an index on `name`, the
    // filtered scan may follow the index (reverse ID order here) unless
    // `chunk()` orders by the primary key.
    // MySQL stores `string` columns as TEXT, which needs an index prefix.
    let index = if driver == "mysql" {
        "CREATE INDEX contract_offset_rows_name ON contract_offset_rows (name(64))"
    } else {
        "CREATE INDEX contract_offset_rows_name ON contract_offset_rows (name)"
    };
    rullst_orm::_sqlx::query(index)
        .execute(Orm::pool().expect("pool"))
        .await
        .expect("create name index");
    let mut chunked = Vec::new();
    ContractOffsetRow::query()
        .where_gt("name", "")
        .chunk(2, |rows| {
            chunked.extend(rows.iter().map(|row| row.id));
            async {}
        })
        .await
        .unwrap_or_else(|error| panic!("{driver} chunk: {error}"));
    let mut ascending = chunked.clone();
    ascending.sort_unstable();
    assert_eq!(chunked, ascending, "{driver}");
    assert_eq!(chunked.len(), 5, "{driver}");

    bounded_traversals(driver).await;
}

/// Pages handed to the handler of an offset or keyset traversal.
type Pages = Vec<Vec<i32>>;

async fn offset_pages(query: ContractOffsetRowQueryBuilder, size: usize) -> Pages {
    let mut pages = Vec::new();
    query
        .chunk(size, |rows| {
            pages.push(rows.iter().map(|row| row.id).collect());
            async {}
        })
        .await
        .expect("bounded chunk");
    pages
}

async fn keyset_pages(query: ContractOffsetRowQueryBuilder, size: usize) -> Pages {
    let mut pages = Vec::new();
    query
        .chunk_by_id(size, |rows| {
            pages.push(rows.iter().map(|row| row.id).collect());
            async { Ok(()) }
        })
        .await
        .expect("bounded chunk_by_id");
    pages
}

/// An explicit `limit()` caps the rows a traversal hands to its handler and
/// an explicit `offset()` is where it starts, instead of both being dropped.
async fn bounded_traversals(driver: &str) {
    let ids = ContractOffsetRow::query()
        .order_by("id")
        .pluck_i32("id")
        .await
        .expect("contract row ids");
    assert_eq!(ids.len(), 5, "{driver}");
    let pages = |bounds: &[std::ops::Range<usize>]| -> Pages {
        bounds
            .iter()
            .map(|range| ids[range.clone()].to_vec())
            .collect()
    };
    let query = ContractOffsetRow::query;
    assert_eq!(
        offset_pages(query().limit(3), 2).await,
        pages(&[0..2, 2..3]),
        "{driver}"
    );
    assert_eq!(
        offset_pages(query().offset(1), 2).await,
        pages(&[1..3, 3..5]),
        "{driver}"
    );
    assert_eq!(
        offset_pages(query().offset(1).limit(3), 2).await,
        pages(&[1..3, 3..4]),
        "{driver}"
    );
    assert_eq!(
        offset_pages(query().order_by_desc("id").limit(2), 1).await,
        [vec![ids[4]], vec![ids[3]]],
        "{driver}"
    );
    assert!(
        offset_pages(query().limit(0), 2).await.is_empty(),
        "{driver}"
    );
    assert_eq!(
        keyset_pages(query().limit(3), 2).await,
        pages(&[0..2, 2..3]),
        "{driver}"
    );
    assert_eq!(
        keyset_pages(query().offset(2), 2).await,
        pages(&[2..4, 4..5]),
        "{driver}"
    );
    assert_eq!(
        keyset_pages(query().offset(1).limit(2), 5).await,
        [ids[1..3].to_vec()],
        "{driver}"
    );
    // The implicit global row cap is not a caller bound.
    assert_eq!(offset_pages(query(), 2).await.concat(), ids, "{driver}");
    assert_eq!(keyset_pages(query(), 2).await.concat(), ids, "{driver}");
}
