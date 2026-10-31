//! Value round trips and filters: integer bounds, floats, booleans, NULL
//! against the empty string, Unicode and long text.

use rullst_orm::{Error, Orm};
use serde_json::{Value, json};

use super::{create_table, sorted};
use crate::transcript::Scenario;

#[derive(Debug, Clone, Orm)]
#[orm(table = "diff_values")]
struct Sample {
    id: i32,
    label: String,
    small: i32,
    wide: i64,
    ratio: f64,
    flag: bool,
    note: Option<String>,
}

impl Sample {
    fn label(&self) -> &str {
        &self.label
    }

    fn columns(&self) -> Value {
        json!([self.small, self.wide, self.ratio, self.flag, self.note])
    }
}

/// Label, `small`, `wide`, `ratio`, `flag` and `note` of one row.
type Row = (&'static str, i32, i64, f64, bool, Option<&'static str>);

const ROWS: [Row; 5] = [
    ("max", i32::MAX, i64::MAX, f64::MAX, true, Some("")),
    ("min", i32::MIN, i64::MIN, -1e-300, false, None),
    ("zero", 0, 0, 0.0, false, Some("zero")),
    ("decimal", -1, -1, 1_234_567.89, true, None),
    ("sum", 7, 5_000_000_000, 0.1 + 0.2, true, Some("sum")),
];

async fn insert(
    label: &str,
    small: i32,
    wide: i64,
    ratio: f64,
    flag: bool,
    note: Option<&str>,
) -> Result<i32, Error> {
    let mut sample = Sample {
        id: 0,
        label: label.to_string(),
        small,
        wide,
        ratio,
        flag,
        note: note.map(str::to_string),
    };
    sample.save().await?;
    Ok(sample.id)
}

/// Saves `text` as a label and reports whether it reads back unchanged.
async fn text_round_trip(text: &str) -> Result<bool, Error> {
    let id = insert(text, 1, 1, 1.0, true, None).await?;
    let stored = Sample::find(id).await?.ok_or(Error::RecordNotFound)?;
    Ok(stored.label == text)
}

pub async fn run(s: &mut Scenario<'_>) {
    let created = create_table(s, "diff_values", |table| {
        table.id();
        table.string("label").not_null();
        table.integer("small").not_null();
        table.big_integer("wide").not_null();
        table.float("ratio").not_null();
        table.boolean("flag").not_null();
        table.string("note");
    })
    .await;
    if !created {
        return;
    }

    for (label, small, wide, ratio, flag, note) in ROWS {
        let stored = match insert(label, small, wide, ratio, flag, note).await {
            Ok(id) => Sample::find(id)
                .await
                .map(|found| found.map(|row| row.columns())),
            Err(error) => Err(error),
        };
        s.outcome(&format!("round_trip.{label}"), stored);
    }

    let query = Sample::query;
    s.outcome(
        "wide_eq_max",
        sorted(
            query().where_eq("wide", i64::MAX).get().await,
            Sample::label,
        ),
    );
    s.outcome(
        "wide_lt_zero",
        sorted(query().where_lt("wide", 0_i64).get().await, Sample::label),
    );
    s.outcome(
        "wide_gt_i32_max",
        sorted(
            query().where_gt("wide", i64::from(i32::MAX)).get().await,
            Sample::label,
        ),
    );
    s.outcome(
        "small_eq_min",
        sorted(
            query().where_eq("small", i32::MIN).get().await,
            Sample::label,
        ),
    );
    s.outcome(
        "ratio_gt_one",
        sorted(query().where_gt("ratio", 1.0).get().await, Sample::label),
    );
    s.outcome(
        "flag_true",
        sorted(query().where_eq("flag", true).get().await, Sample::label),
    );
    s.outcome(
        "flag_false",
        sorted(query().where_eq("flag", false).get().await, Sample::label),
    );
    s.outcome(
        "note_null",
        sorted(query().where_null("note").get().await, Sample::label),
    );
    s.outcome(
        "note_not_null",
        sorted(query().where_not_null("note").get().await, Sample::label),
    );
    s.outcome(
        "note_eq_empty",
        sorted(query().where_eq("note", "").get().await, Sample::label),
    );
    s.outcome(
        "note_eq_none_matches_nothing",
        sorted(
            query().where_eq("note", None::<String>).get().await,
            Sample::label,
        ),
    );

    let unicode = "Grüße 👋 你好 مرحبا ∑ e\u{301}";
    s.outcome("unicode_round_trip", text_round_trip(unicode).await);
    s.outcome(
        "unicode_eq",
        Sample::query().where_eq("label", unicode).count().await,
    );
    s.outcome(
        "long_text_round_trip",
        text_round_trip(&"é".repeat(5_000)).await,
    );
    s.outcome("text_with_nul", text_round_trip("before\0after").await);
}
