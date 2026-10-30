//! Date/time columns: with a strict driver feature `chrono` fields round-trip,
//! filter and order on every driver; the SQLx `Any` pool keeps SQLite's ISO
//! text and refuses native PostgreSQL and MySQL/MariaDB date/time columns.

use rullst_orm::Orm;
use rullst_orm::schema::Blueprint;

fn calendar(table: &mut Blueprint) {
    table.id();
    table.string("title").not_null();
    table.timestamp_tz("starts_at").not_null();
    table.timestamp("local_start").not_null();
    table.date("day").not_null();
    table.time("opens").not_null();
    table.timestamp_tz("ends_at");
    table.timestamps();
}

pub(super) async fn exercise() {
    let driver = Orm::driver().expect("initialized driver");
    #[cfg(any(
        feature = "strict-postgres",
        feature = "strict-mysql",
        feature = "strict-sqlite"
    ))]
    native::exercise(driver).await;
    #[cfg(not(any(
        feature = "strict-postgres",
        feature = "strict-mysql",
        feature = "strict-sqlite"
    )))]
    any_pool::exercise(driver).await;
}

#[cfg(any(
    feature = "strict-postgres",
    feature = "strict-mysql",
    feature = "strict-sqlite"
))]
mod native {
    use rullst_orm::Orm;
    use rullst_orm::chrono::{
        DateTime, Duration, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc,
    };
    use rullst_orm::schema::Schema;

    #[derive(Debug, Clone, PartialEq, Orm)]
    #[orm(table = "contract_calendar")]
    struct CalendarEntry {
        id: i32,
        title: String,
        starts_at: DateTime<Utc>,
        local_start: NaiveDateTime,
        day: NaiveDate,
        opens: NaiveTime,
        ends_at: Option<DateTime<Utc>>,
        #[orm(db_default)]
        created_at: Option<DateTime<Utc>>,
        #[orm(db_default)]
        updated_at: Option<DateTime<Utc>>,
    }

    fn titles(entries: &[CalendarEntry]) -> Vec<&str> {
        entries.iter().map(|entry| entry.title.as_str()).collect()
    }

    pub(super) async fn exercise(driver: &str) {
        Schema::create("contract_calendar", super::calendar)
            .await
            .unwrap_or_else(|error| panic!("{driver} date/time columns: {error}"));

        let starts_at = Utc
            .with_ymd_and_hms(2026, 10, 15, 9, 30, 0)
            .single()
            .expect("instant")
            + Duration::microseconds(123_456);
        let day = NaiveDate::from_ymd_opt(2026, 2, 28).expect("date");
        let local_start = day
            .and_hms_micro_opt(6, 30, 0, 250_000)
            .expect("local time");
        let opens = NaiveTime::from_hms_micro_opt(8, 15, 30, 500_000).expect("time");
        let before = Utc::now();
        let mut entry = CalendarEntry {
            id: 0,
            title: "launch".to_string(),
            starts_at,
            local_start,
            day,
            opens,
            ends_at: None,
            created_at: None,
            updated_at: None,
        };
        entry.save().await.expect("insert date/time values");
        let after = Utc::now();

        let stored = CalendarEntry::find(entry.id)
            .await
            .expect("read date/time values")
            .expect("entry exists");
        assert_eq!(stored.starts_at, starts_at, "{driver}");
        assert_eq!(stored.local_start, local_start, "{driver}");
        assert_eq!(stored.day, day, "{driver}");
        assert_eq!(stored.opens, opens, "{driver}");
        assert_eq!(stored.ends_at, None, "{driver}");
        // The database default filled the column the insert left out. Its
        // clock may drift from the test's a little, but never by a time
        // zone's offset.
        let created = stored.created_at.expect("database default");
        assert!(
            created > before - Duration::minutes(10) && created < after + Duration::minutes(10),
            "{driver}: created_at {created} outside {before}..{after}"
        );
        assert!(stored.updated_at.is_some(), "{driver}");
        assert_eq!(
            entry.created_at, None,
            "the insert does not read defaults back"
        );
        assert!(
            stored
                .to_json()
                .contains("\"starts_at\":\"2026-10-15T09:30:00.123456Z\""),
            "{driver}: {}",
            stored.to_json()
        );

        let mut later = CalendarEntry {
            id: 0,
            title: "later".to_string(),
            starts_at: starts_at + Duration::days(1),
            local_start: local_start + Duration::hours(1),
            day: day.succ_opt().expect("next day"),
            opens,
            ends_at: Some(starts_at + Duration::days(2)),
            created_at: None,
            updated_at: None,
        };
        later.save().await.expect("insert a later entry");
        let ordered = CalendarEntry::query()
            .order_by_desc("starts_at")
            .get()
            .await
            .expect("order by an instant");
        assert_eq!(titles(&ordered), ["later", "launch"], "{driver}");
        let upcoming = CalendarEntry::query()
            .where_gt("starts_at", starts_at)
            .get()
            .await
            .expect("filter by an instant");
        assert_eq!(titles(&upcoming), ["later"], "{driver}");
        let on_day = CalendarEntry::query()
            .where_eq("day", day)
            .get()
            .await
            .expect("filter by a date");
        assert_eq!(titles(&on_day), ["launch"], "{driver}");
        let early = CalendarEntry::query()
            .where_lt("local_start", local_start + Duration::minutes(1))
            .get()
            .await
            .expect("filter by a local date and time");
        assert_eq!(titles(&early), ["launch"], "{driver}");
        let ended = CalendarEntry::query()
            .where_not_null("ends_at")
            .get()
            .await
            .expect("filter by a nullable instant");
        assert_eq!(titles(&ended), ["later"], "{driver}");

        // Writes never send `#[orm(db_default)]` columns.
        let ends_at = starts_at + Duration::hours(2);
        entry
            .update_partial()
            .ends_at(Some(ends_at))
            .save()
            .await
            .expect("partial update of an instant");
        let mut reloaded = CalendarEntry::find(entry.id)
            .await
            .expect("reload")
            .expect("entry exists");
        assert_eq!(reloaded.ends_at, Some(ends_at), "{driver}");
        reloaded.created_at = None;
        reloaded.save().await.expect("full-row save");
        let saved = CalendarEntry::find(entry.id)
            .await
            .expect("reload")
            .expect("entry exists");
        assert_eq!(saved.created_at, Some(created), "{driver} keeps created_at");

        if driver == "sqlite" {
            // RFC 3339 UTC text, which sorts by time; the database default
            // has the same shape.
            let (starts_text, created_text): (String, String) = rullst_orm::sqlx::query_as(
                "SELECT starts_at, created_at FROM contract_calendar WHERE id = ?",
            )
            .bind(entry.id)
            .fetch_one(Orm::pool().expect("pool"))
            .await
            .expect("read stored text");
            assert_eq!(starts_text, "2026-10-15T09:30:00.123456+00:00");
            assert_eq!(created_text.len(), "2026-10-15T09:30:00.123+00:00".len());
            assert_eq!(&created_text[10..11], "T", "{created_text}");
            assert!(created_text.ends_with("+00:00"), "{created_text}");
        }

        Schema::drop_if_exists("contract_calendar")
            .await
            .expect("drop calendar");
    }
}

#[cfg(not(any(
    feature = "strict-postgres",
    feature = "strict-mysql",
    feature = "strict-sqlite"
)))]
mod any_pool {
    use rullst_orm::chrono::{TimeZone, Utc};
    use rullst_orm::schema::Schema;
    use rullst_orm::{Error, Orm};

    /// SQLx `Any` decodes no `chrono` type, so the fields are text.
    #[derive(Debug, Clone, Orm)]
    #[orm(table = "contract_calendar")]
    struct CalendarText {
        id: i32,
        title: String,
        starts_at: String,
        local_start: String,
        day: String,
        opens: String,
        ends_at: Option<String>,
        #[orm(db_default)]
        created_at: Option<String>,
        #[orm(db_default)]
        updated_at: Option<String>,
    }

    pub(super) async fn exercise(driver: &str) {
        let created = Schema::create("contract_calendar", super::calendar).await;
        if driver != "sqlite" {
            assert!(
                matches!(&created, Err(Error::Validation(message)) if message.contains("strict-")),
                "{driver}: {created:?}"
            );
            let table = Orm::raw("DELETE FROM contract_calendar").execute().await;
            assert!(table.is_err(), "{driver}: no table may be created");
            return;
        }
        created.expect("SQLite date/time columns are text");
        let mut entry = CalendarText {
            id: 0,
            title: "launch".to_string(),
            starts_at: "2026-10-15T09:30:00.123456+00:00".to_string(),
            local_start: "2026-02-28 06:30:00.250".to_string(),
            day: "2026-02-28".to_string(),
            opens: "08:15:30.500".to_string(),
            ends_at: None,
            created_at: None,
            updated_at: None,
        };
        entry.save().await.expect("insert date/time text");
        let stored = CalendarText::find(entry.id)
            .await
            .expect("read date/time text")
            .expect("entry exists");
        assert_eq!(stored.starts_at, entry.starts_at);
        assert_eq!(stored.day, entry.day);
        assert!(stored.created_at.is_some_and(|value| !value.is_empty()));
        // A `chrono` filter binds the same RFC 3339 text.
        let earlier = Utc
            .with_ymd_and_hms(2026, 10, 15, 9, 30, 0)
            .single()
            .expect("instant");
        let upcoming = CalendarText::query()
            .where_gt("starts_at", earlier)
            .get()
            .await
            .expect("filter text by an instant");
        assert_eq!(upcoming.len(), 1);
        Schema::drop_if_exists("contract_calendar")
            .await
            .expect("drop calendar");
    }
}
