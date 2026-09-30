//! `count()` and `paginate()` totals count the rows `get()` returns, including
//! DISTINCT and GROUP BY queries, through a wrapped derived table.

use rullst_orm::schema::{Blueprint, Schema};
use rullst_orm::{FromRow, Orm};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "contract_count_orders")]
struct ContractCountOrder {
    id: i32,
    status: String,
    email: String,
}

pub(super) async fn exercise() {
    let driver = Orm::driver().expect("initialized driver");
    Schema::create("contract_count_orders", |table: &mut Blueprint| {
        table.id();
        table.string("status").not_null();
        table.string("email").not_null();
    })
    .await
    .expect("create count contract table");
    for (status, email) in [
        ("paid", "a@example.test"),
        ("paid", "a@example.test"),
        ("open", "b@example.test"),
        ("open", "c@example.test"),
        ("void", "c@example.test"),
    ] {
        let mut order = ContractCountOrder {
            id: 0,
            status: status.to_string(),
            email: email.to_string(),
        };
        order.save().await.expect("insert count contract row");
    }

    let rows = ContractCountOrder::query().count().await;
    assert_eq!(rows.expect("plain count"), 5, "{driver}");
    let emails = ContractCountOrder::query()
        .distinct()
        .select(&["email"])
        .count()
        .await;
    assert_eq!(emails.expect("distinct count"), 3, "{driver}");
    let statuses = ContractCountOrder::query().group_by("status").count().await;
    assert_eq!(statuses.expect("grouped count"), 3, "{driver}");
    let filtered = ContractCountOrder::query()
        .where_eq("status", "open")
        .distinct()
        .select(&["email"])
        .count()
        .await;
    assert_eq!(filtered.expect("filtered distinct count"), 2, "{driver}");

    // Grouping by the primary key keeps every column selectable on each
    // backend; the total must count the groups, not one group's rows.
    let page = ContractCountOrder::query()
        .group_by("id")
        .order_by("id")
        .paginate(1, 2)
        .await
        .unwrap_or_else(|error| panic!("{driver} grouped paginate: {error}"));
    assert_eq!(
        (page.total, page.last_page, page.data.len()),
        (5, 3, 2),
        "{driver}"
    );
}
