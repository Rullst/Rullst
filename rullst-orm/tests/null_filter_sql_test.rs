//! `None` in a value-taking filter is written as the SQL `NULL` literal, not
//! bound: PostgreSQL cannot compare a text-typed NULL parameter with a
//! non-text column (`bigint = text`), while the literal takes the column's
//! type, and a comparison with NULL matches no row on every backend. The
//! differential database test checks the rows on all four backends; this
//! test pins the SQL without a database.

use rullst_orm::Orm;

#[derive(Debug, Clone, Orm)]
#[orm(table = "null_filter_rows")]
struct NullFilterRow {
    id: i32,
    tier: Option<i64>,
    name: String,
}

#[test]
fn none_filter_values_are_null_literals_and_other_values_stay_bound() {
    let query = NullFilterRow::query;
    let none = None::<i64>;
    let cases = [
        (query().where_eq("tier", none).to_sql(), "tier = NULL"),
        (query().where_not_eq("tier", none).to_sql(), "tier != NULL"),
        (query().where_gt("tier", none).to_sql(), "tier > NULL"),
        (query().where_lt("tier", none).to_sql(), "tier < NULL"),
        (
            query().where_like("name", None::<String>).to_sql(),
            "name LIKE NULL",
        ),
        (
            query().where_not_like("name", None::<String>).to_sql(),
            "name NOT LIKE NULL",
        ),
        (
            query().where_in("tier", vec![Some(1_i64), None]).to_sql(),
            "tier IN (?, NULL)",
        ),
        (
            query()
                .where_not_in("tier", vec![None, Some(2_i64)])
                .to_sql(),
            "tier NOT IN (NULL, ?)",
        ),
        (
            query().where_between("tier", None, Some(9_i64)).to_sql(),
            "tier BETWEEN NULL AND ?",
        ),
        (
            query()
                .where_not_between("tier", Some(1_i64), None)
                .to_sql(),
            "tier NOT BETWEEN ? AND NULL",
        ),
        (
            query()
                .where_eq("name", "ann")
                .or_where("tier", none)
                .to_sql(),
            "(name = ?) OR (tier = NULL)",
        ),
        (
            query().where_col(NullFilterRowColumn::Tier, none).to_sql(),
            "tier = NULL",
        ),
        (query().where_eq("tier", Some(3_i64)).to_sql(), "tier = ?"),
    ];
    for (sql, expected) in cases {
        assert!(sql.contains(expected), "expected `{expected}` in {sql}");
    }
}
