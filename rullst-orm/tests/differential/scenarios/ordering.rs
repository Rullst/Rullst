//! Ordering, paging, counts and bulk deletes on the seeded `diff_scores`.

use serde_json::json;

use super::in_order;
use super::scores::Score;
use crate::transcript::Scenario;

pub async fn run(s: &mut Scenario<'_>) {
    let query = Score::query;
    let cases = [
        ("asc", query().order_by("score")),
        ("desc", query().order_by_desc("score")),
        ("limit_offset", query().order_by("score").limit(2).offset(1)),
        ("offset_without_limit", query().order_by("score").offset(4)),
        (
            "limit_beyond_rows",
            query().order_by_desc("name").limit(100),
        ),
        (
            "filtered_desc",
            query().where_not_null("rank").order_by_desc("rank"),
        ),
        ("nulls_ascending", query().order_by("rank")),
        ("nulls_descending", query().order_by_desc("rank")),
    ];
    for (step, builder) in cases {
        s.outcome(step, in_order(builder.get().await, Score::name));
    }

    for (step, page) in [("paginate_page_2", 2), ("paginate_past_end", 9)] {
        let result = query().order_by("score").paginate(page, 2).await;
        s.outcome(
            step,
            result.map(|page| {
                let names: Vec<&str> = page.data.iter().map(Score::name).collect();
                json!({"total": page.total, "last_page": page.last_page, "names": names})
            }),
        );
    }

    s.outcome("count", query().count().await);
    s.outcome(
        "count_distinct",
        query().distinct().select(&["tag"]).count().await,
    );
    s.outcome("count_grouped", query().group_by("tag").count().await);
    s.outcome(
        "pluck_string",
        query().order_by_desc("score").pluck_string("name").await,
    );
    s.outcome(
        "pluck_i32",
        query().order_by("name").pluck_i32("score").await,
    );

    s.outcome(
        "delete_all_filtered",
        query().where_lt("score", 30).delete_all().await,
    );
    s.outcome(
        "names_after_delete_all",
        in_order(query().order_by("score").get().await, Score::name),
    );
}
