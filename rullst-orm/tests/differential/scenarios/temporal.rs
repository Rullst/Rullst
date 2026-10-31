//! `timestamp_tz()`, `date()` and `timestamps()` with `chrono` fields:
//! round trips, sub-microsecond precision, instants outside the 1970–2038
//! `TIMESTAMP` range, ordering and filters.

use rullst_orm::chrono::{DateTime, NaiveDate, SecondsFormat, TimeZone, Utc};
use rullst_orm::{Error, Orm};

use super::{create_table, in_order, sorted};
use crate::transcript::Scenario;

#[derive(Debug, Clone, Orm)]
#[orm(table = "diff_events")]
struct Event {
    id: i32,
    title: String,
    starts_at: DateTime<Utc>,
    ends_at: Option<DateTime<Utc>>,
    day: NaiveDate,
    #[orm(db_default)]
    created_at: Option<DateTime<Utc>>,
    #[orm(db_default)]
    updated_at: Option<DateTime<Utc>>,
}

impl Event {
    fn title(&self) -> &str {
        &self.title
    }
}

fn instant(text: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(text)
        .expect("valid RFC 3339 fixture")
        .with_timezone(&Utc)
}

fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).expect("valid date fixture")
}

fn nanos(value: &DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Nanos, true)
}

/// Saves an event and reads it back.
async fn save(
    title: &str,
    starts_at: DateTime<Utc>,
    ends_at: Option<DateTime<Utc>>,
    day: NaiveDate,
) -> Result<Event, Error> {
    let mut event = Event {
        id: 0,
        title: title.to_string(),
        starts_at,
        ends_at,
        day,
        created_at: None,
        updated_at: None,
    };
    event.save().await?;
    Event::find(event.id).await?.ok_or(Error::RecordNotFound)
}

/// Kept for the ordering and filter steps.
const KEPT: [(&str, &str, (i32, u32, u32)); 4] = [
    ("micros", "2026-10-15T09:30:00.123456Z", (2026, 10, 15)),
    ("nanos", "2026-10-15T09:30:00.987654321Z", (2026, 10, 15)),
    ("whole_second", "2026-10-15T09:29:59Z", (2026, 10, 14)),
    ("y2000", "2000-01-01T00:00:00Z", (2000, 1, 1)),
];

/// Outside the 1970–2038 range of a `TIMESTAMP` column; deleted after their
/// round trip, so the ordering and filter steps do not depend on them.
const BOUNDARY: [(&str, &str); 2] = [
    ("before_epoch", "1969-07-20T20:17:40Z"),
    ("after_2038", "2040-02-29T23:59:59.5Z"),
];

pub async fn run(s: &mut Scenario<'_>) {
    let created = create_table(s, "diff_events", |table| {
        table.id();
        table.string("title").not_null();
        table.timestamp_tz("starts_at").not_null();
        table.timestamp_tz("ends_at");
        table.date("day").not_null();
        table.timestamps();
    })
    .await;
    if !created {
        return;
    }
    for (title, text, (year, month, day)) in KEPT {
        let saved = save(title, instant(text), None, date(year, month, day)).await;
        s.outcome(
            &format!("round_trip.{title}"),
            saved.map(|event| nanos(&event.starts_at)),
        );
    }
    for (title, text) in BOUNDARY {
        let at = instant(text);
        let saved = save(title, at, Some(at), at.date_naive()).await;
        s.outcome(
            &format!("round_trip.{title}"),
            saved.map(|event| nanos(&event.starts_at)),
        );
        let _ = Event::query().where_eq("title", title).delete_all().await;
        // The same instant in a nullable column, next to an in-range one.
        let saved = save(title, instant(KEPT[0].1), Some(at), at.date_naive()).await;
        s.outcome(
            &format!("nullable_round_trip.{title}"),
            saved.map(|event| event.ends_at.as_ref().map(nanos)),
        );
        let _ = Event::query().where_eq("title", title).delete_all().await;
    }
    run_queries(s).await;
}

async fn run_queries(s: &mut Scenario<'_>) {
    let query = Event::query;
    s.outcome(
        "order_by_instant",
        in_order(query().order_by("starts_at").get().await, Event::title),
    );
    let noon = Utc
        .with_ymd_and_hms(2026, 10, 15, 9, 30, 0)
        .single()
        .expect("valid instant");
    let since_epoch = instant("1970-01-01T00:00:00Z");
    let cases = [
        ("instant_gt", query().where_gt("starts_at", noon)),
        (
            "instant_between",
            query().where_between("starts_at", since_epoch, noon),
        ),
        ("date_eq", query().where_eq("day", date(2026, 10, 15))),
        ("date_lt", query().where_lt("day", date(2001, 1, 1))),
        ("ends_at_null", query().where_null("ends_at")),
    ];
    for (step, builder) in cases {
        s.outcome(step, sorted(builder.get().await, Event::title));
    }
    s.outcome(
        "date_round_trip",
        query()
            .where_eq("title", "y2000")
            .first()
            .await
            .map(|event| event.map(|event| event.day.to_string())),
    );
    s.outcome(
        "timestamps_filled_by_default",
        query().get().await.map(|events| {
            events
                .iter()
                .all(|event| event.created_at.is_some() && event.updated_at.is_some())
        }),
    );
}
