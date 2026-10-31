//! The table the `filters` and `ordering` scenarios query. Names, scores and
//! ranks are distinct and only one rank is NULL, so an ordered query has
//! exactly one correct order.

use rullst_orm::Orm;

use super::create_table;
use crate::transcript::Scenario;

#[derive(Debug, Clone, Orm)]
#[orm(table = "diff_scores")]
pub struct Score {
    pub id: i32,
    pub name: String,
    pub score: i32,
    pub rank: Option<i64>,
    pub tag: Option<String>,
}

impl Score {
    pub fn name(&self) -> &str {
        &self.name
    }
}

const ROWS: [(&str, i32, Option<i64>, Option<&str>); 6] = [
    ("ann", 10, Some(3), Some("red")),
    ("ben", 20, None, Some("blue")),
    ("cat", 30, Some(1), None),
    ("dan", 40, Some(2), Some("red")),
    ("eve", 50, Some(5), Some("green")),
    ("fay", 60, Some(4), Some("blue")),
];

/// Creates and fills `diff_scores`; `false` when it could not.
pub async fn seed(s: &mut Scenario<'_>) -> bool {
    let created = create_table(s, "diff_scores", |table| {
        table.id();
        table.string("name").not_null();
        table.integer("score").not_null();
        table.big_integer("rank");
        table.string("tag");
    })
    .await;
    if !created {
        return false;
    }
    let mut inserted = Ok(());
    for (name, score, rank, tag) in ROWS {
        let mut row = Score {
            id: 0,
            name: name.to_string(),
            score,
            rank,
            tag: tag.map(str::to_string),
        };
        inserted = inserted.and(row.save().await);
    }
    let ok = inserted.is_ok();
    s.outcome("seed", inserted);
    ok
}
