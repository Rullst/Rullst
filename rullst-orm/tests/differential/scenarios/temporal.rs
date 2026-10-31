//! `timestamp_tz()`, `date()` and `timestamps()` with `chrono` fields:
//! round trips, sub-microsecond precision, ordering and filters.

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

/// Saves an event and reads back its instant at nanosecond resolution.
async fn round_trip(
    title: &str,
    starts_at: DateTime<Utc>,
    day: NaiveDate,
) -> Result<String, Error> {
    let mut event = Event {
        id: 0,
        title: title.to_string(),
        starts_at,
        day,
        created_at: None,
        updated_at: None,
    };
    event.save().await?;
    let stored = Event::find(event.id).await?.ok_or(Error::RecordNotFound)?;
    Ok(stored.starts_at.to_rfc3339_opts(SecondsFormat::Nanos, true))
}

pub async fn run(s: &mut Scenario<'_>) {
    let created = create_table(s, "diff_events", |table| {
        table.id();
        table.string("title").not_null();
        table.timestamp_tz("starts_at").not_null();
        table.date("day").not_null();
        table.timestamps();
    })
    .await;
    if !created {
        return;
    }
    let fixtures = [
        ("micros", "2026-10-15T09:30:00.123456Z", date(2026, 10, 15)),
        (
            "nanos",
            "2026-10-15T09:30:00.987654321Z",
            date(2026, 10, 15),
        ),
        ("whole_second", "2026-10-15T09:29:59Z", date(2026, 10, 14)),
        ("before_epoch", "1969-07-20T20:17:40Z", date(1969, 7, 20)),
        ("after_2038", "2040-02-29T23:59:59.5Z", date(2040, 2, 29)),
    ];
    for (title, text, day) in fixtures {
        s.outcome(
            &format!("round_trip.{title}"),
            round_trip(title, instant(text), day).await,
        );
    }

    let query = Event::query;
    s.outcome(
        "order_by_instant",
        in_order(query().order_by("starts_at").get().await, Event::title),
    );
    let noon = Utc
        .with_ymd_and_hms(2026, 10, 15, 9, 30, 0)
        .single()
        .expect("valid instant");
    s.outcome(
        "instant_gt",
        sorted(
            query().where_gt("starts_at", noon).get().await,
            Event::title,
        ),
    );
    s.outcome(
        "instant_between",
        sorted(
            query()
                .where_between("starts_at", instant("1970-01-01T00:00:00Z"), noon)
                .get()
                .await,
            Event::title,
        ),
    );
    s.outcome(
        "date_eq",
        sorted(
            query().where_eq("day", date(2026, 10, 15)).get().await,
            Event::title,
        ),
    );
    s.outcome(
        "date_lt",
        sorted(
            query().where_lt("day", date(2000, 1, 1)).get().await,
            Event::title,
        ),
    );
    s.outcome(
        "date_round_trip",
        query()
            .where_eq("title", "after_2038")
            .first()
            .await
            .map(|event| event.map(|event| event.day.to_string())),
    );
    s.outcome(
        "timestamps_filled_by_default",
        query().order_by("id").get().await.map(|events| {
            events
                .iter()
                .all(|event| event.created_at.is_some() && event.updated_at.is_some())
        }),
    );
}
