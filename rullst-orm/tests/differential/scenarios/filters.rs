//! Query-builder filters. No filter here specifies an order, so the matched
//! names are recorded sorted.

use super::scores::{self, Score};
use super::sorted;
use crate::transcript::Scenario;

pub async fn run(s: &mut Scenario<'_>) {
    if !scores::seed(s).await {
        return;
    }
    let query = Score::query;
    let cases = [
        ("eq", query().where_eq("score", 30)),
        ("not_eq_skips_null", query().where_not_eq("tag", "red")),
        ("gt", query().where_gt("score", 40)),
        ("lt", query().where_lt("score", 20)),
        ("between", query().where_between("score", 20, 40)),
        ("not_between", query().where_not_between("score", 20, 40)),
        ("in", query().where_in("score", vec![10, 50, 70])),
        ("in_text", query().where_in("tag", vec!["red", "green"])),
        ("not_in", query().where_not_in("score", vec![10, 50])),
        (
            "not_in_skips_null",
            query().where_not_in("tag", vec!["red"]),
        ),
        ("in_empty", query().where_in("score", Vec::<i32>::new())),
        (
            "not_in_empty",
            query().where_not_in("score", Vec::<i32>::new()),
        ),
        ("null", query().where_null("tier")),
        ("not_null", query().where_not_null("tag")),
        (
            "or_where",
            query().where_eq("name", "ann").or_where("name", "eve"),
        ),
        (
            "or_where_null",
            query().where_gt("score", 50).or_where_null("tag"),
        ),
        ("like_contains", query().where_like("name", "%a%")),
        ("like_single_char", query().where_like("name", "_e_")),
        ("not_like", query().where_not_like("name", "%n")),
        (
            "eq_none_matches_nothing",
            query().where_eq("tier", None::<i64>),
        ),
        // `None` in any comparison is SQL NULL: it matches no row, and a NOT
        // IN list containing it matches none either.
        (
            "not_eq_none_matches_nothing",
            query().where_not_eq("tier", None::<i64>),
        ),
        (
            "gt_none_matches_nothing",
            query().where_gt("tier", None::<i64>),
        ),
        (
            "in_with_none",
            query().where_in("tier", vec![Some(1_i64), None]),
        ),
        (
            "not_in_with_none",
            query().where_not_in("tier", vec![Some(1_i64), None]),
        ),
        (
            "or_where_none",
            query()
                .where_eq("name", "ann")
                .or_where("tier", None::<i64>),
        ),
        (
            "combined",
            query()
                .where_gt("score", 10)
                .where_null("tier")
                .where_eq("tag", "blue"),
        ),
        ("i64_filter", query().where_in("tier", vec![1_i64, 4])),
    ];
    for (step, builder) in cases {
        s.outcome(step, sorted(builder.get().await, Score::name));
    }
    s.outcome(
        "count_filtered",
        query().where_gt("score", 20).count().await,
    );
    s.outcome(
        "first_filtered_ordered",
        query()
            .where_not_null("tag")
            .order_by_desc("score")
            .first()
            .await
            .map(|row| row.map(|row| row.name)),
    );
}
