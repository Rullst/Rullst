//! Insert, find, update and delete through a `#[derive(Orm)]` model.

use rullst_orm::Orm;
use serde_json::{Value, json};

use super::{create_table, sorted};
use crate::transcript::Scenario;

#[derive(Debug, Clone, Orm)]
#[orm(table = "diff_items")]
struct Item {
    id: i32,
    name: String,
    note: Option<String>,
    quantity: i32,
    views: i64,
    price: f64,
    active: bool,
}

impl Item {
    fn new(name: &str, quantity: i32, active: bool) -> Self {
        Self {
            id: 0,
            name: name.to_string(),
            note: None,
            quantity,
            views: i64::from(quantity) * 1_000,
            price: f64::from(quantity) / 4.0,
            active,
        }
    }

    /// Every column except the generated id.
    fn columns(&self) -> Value {
        json!([
            self.name,
            self.note,
            self.quantity,
            self.views,
            self.price,
            self.active
        ])
    }

    fn name(&self) -> &str {
        &self.name
    }
}

pub async fn run(s: &mut Scenario<'_>) {
    let created = create_table(s, "diff_items", |table| {
        table.id();
        table.string("name").not_null();
        table.string("note");
        table.integer("quantity").not_null();
        table.big_integer("views").not_null();
        table.float("price").not_null();
        table.boolean("active").not_null();
    })
    .await;
    if !created {
        return;
    }

    let mut items = vec![
        Item::new("first", 1, true),
        Item::new("second", 2, false),
        Item::new("third", 3, true),
    ];
    let mut saved = Vec::new();
    for item in &mut items {
        saved.push(item.save().await);
    }
    s.outcome("insert", saved.into_iter().collect::<Result<Vec<()>, _>>());
    let ids: Vec<i32> = items.iter().map(|item| item.id).collect();
    s.record(
        "insert_assigns_increasing_ids",
        ids[0] > 0 && ids.windows(2).all(|pair| pair[0] < pair[1]),
    );

    s.outcome(
        "find",
        Item::find(ids[1])
            .await
            .map(|found| found.map(|item| item.columns())),
    );
    let missing = ids.iter().max().copied().unwrap_or_default() + 1_000;
    s.outcome(
        "find_missing",
        Item::find(missing).await.map(|found| found.is_some()),
    );

    let mut second = items[1].clone();
    second.name = "second, renamed".to_string();
    second.note = Some("edited".to_string());
    second.quantity = 20;
    second.active = true;
    s.outcome("update", second.save().await);
    s.outcome(
        "find_after_update",
        Item::find(second.id)
            .await
            .map(|found| found.map(|item| item.columns())),
    );
    s.record("update_keeps_id", second.id == ids[1]);

    s.outcome("delete", items[0].clone().delete().await);
    s.outcome(
        "find_after_delete",
        Item::find(ids[0]).await.map(|found| found.is_some()),
    );
    s.outcome("count_after_delete", Item::query().count().await);
    s.outcome("all_names", sorted(Item::all().await, Item::name));
    s.outcome(
        "first_by_name",
        Item::query()
            .order_by("name")
            .first()
            .await
            .map(|found| found.map(|item| item.columns())),
    );
}
