//! The jiff-based date/time types (`rullst_orm::Timestamp`, `DateTime`, `Date`,
//! `Time`) against the `chrono` ones on a live database: each reads what the
//! other wrote, filters and orders over rows written by both, and the
//! database stores the same value either way (on SQLite, the same text).

use rullst_orm::Orm;

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
    use rullst_orm::chrono::{
        DateTime as ChronoDateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc,
    };
    use rullst_orm::jiff::{self, SignedDuration, civil};
    use rullst_orm::schema::{Blueprint, Schema};
    use rullst_orm::{Date, DateTime, Orm, Time, Timestamp};

    const TABLE: &str = "contract_jiff_calendar";

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

    #[derive(Debug, Clone, PartialEq, rullst_orm::Orm)]
    #[orm(table = "contract_jiff_calendar", full_row_save)]
    struct JiffEntry {
        id: i32,
        title: String,
        starts_at: Timestamp,
        local_start: DateTime,
        day: Date,
        opens: Time,
        ends_at: Option<Timestamp>,
        #[orm(db_default)]
        created_at: Option<Timestamp>,
        #[orm(db_default)]
        updated_at: Option<Timestamp>,
    }

    #[derive(Debug, Clone, PartialEq, rullst_orm::Orm)]
    #[orm(table = "contract_jiff_calendar", full_row_save)]
    struct ChronoEntry {
        id: i32,
        title: String,
        starts_at: ChronoDateTime<Utc>,
        local_start: NaiveDateTime,
        day: NaiveDate,
        opens: NaiveTime,
        ends_at: Option<ChronoDateTime<Utc>>,
        #[orm(db_default)]
        created_at: Option<ChronoDateTime<Utc>>,
        #[orm(db_default)]
        updated_at: Option<ChronoDateTime<Utc>>,
    }

    fn naive(value: DateTime) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(&value.to_string(), "%Y-%m-%dT%H:%M:%S%.f").expect("naive")
    }

    fn chrono(value: Timestamp) -> ChronoDateTime<Utc> {
        ChronoDateTime::parse_from_rfc3339(&value.to_string())
            .expect("instant")
            .with_timezone(&Utc)
    }

    /// What the database keeps of a date-time: SQLite's text keeps it all;
    /// PostgreSQL keeps whole microseconds since 2000, truncated toward 2000;
    /// MySQL/MariaDB receive the fraction's whole microseconds.
    fn stored_civil(driver: &str, value: DateTime) -> DateTime {
        let value = *value;
        let kept = match driver {
            "sqlite" => value,
            "postgres" => {
                let epoch = civil::date(2000, 1, 1).at(0, 0, 0, 0);
                let micros = value.duration_since(epoch).as_micros();
                let micros = i64::try_from(micros).expect("in range");
                epoch
                    .checked_add(SignedDuration::from_micros(micros))
                    .expect("in range")
            }
            _ => {
                let nanos = value.subsec_nanosecond();
                value
                    .with()
                    .subsec_nanosecond(nanos - nanos % 1000)
                    .build()
                    .expect("in range")
            }
        };
        DateTime::from(kept)
    }

    fn stored(driver: &str, value: Timestamp) -> Timestamp {
        let wall = DateTime::from(jiff::tz::Offset::UTC.to_datetime(*value));
        let kept = stored_civil(driver, wall);
        Timestamp::from(jiff::tz::Offset::UTC.to_timestamp(*kept).expect("in range"))
    }

    fn jiff_entry(
        title: &str,
        starts_at: &str,
        local_start: &str,
        day: &str,
        opens: &str,
    ) -> JiffEntry {
        JiffEntry {
            id: 0,
            title: title.to_string(),
            starts_at: starts_at.parse().expect("instant"),
            local_start: local_start.parse().expect("date-time"),
            day: day.parse().expect("date"),
            opens: opens.parse().expect("time"),
            ends_at: None,
            created_at: None,
            updated_at: None,
        }
    }

    fn chrono_entry(entry: &JiffEntry) -> ChronoEntry {
        ChronoEntry {
            id: 0,
            title: format!("{} (chrono)", entry.title),
            starts_at: chrono(entry.starts_at),
            local_start: naive(entry.local_start),
            day: NaiveDate::parse_from_str(&entry.day.to_string(), "%Y-%m-%d").expect("date"),
            opens: NaiveTime::parse_from_str(&entry.opens.to_string(), "%H:%M:%S%.f")
                .expect("time"),
            ends_at: entry.ends_at.map(chrono),
            created_at: None,
            updated_at: None,
        }
    }

    fn titles<T>(entries: &[T], title: impl Fn(&T) -> &str) -> Vec<String> {
        entries
            .iter()
            .map(|entry| title(entry).to_string())
            .collect()
    }

    pub(super) async fn exercise(driver: &str) {
        Schema::drop_if_exists(TABLE).await.expect("clean table");
        Schema::create(TABLE, calendar)
            .await
            .unwrap_or_else(|error| panic!("{driver} date/time columns: {error}"));

        // A post-2038 instant (MariaDB's TIMESTAMP limit), a pre-2000 one with
        // a sub-microsecond part (PostgreSQL truncates toward 2000) and an
        // ordinary one; MySQL/MariaDB start at year 1000.
        let mut rows = vec![
            jiff_entry(
                "ordinary",
                "2026-10-15T09:30:00.123456Z",
                "2026-02-28T06:30:00.25",
                "2026-02-28",
                "08:15:30.5",
            ),
            jiff_entry(
                "after-2038",
                "2040-02-29T23:59:59.5Z",
                "2040-02-29T23:59:59.999999",
                "2040-02-29",
                "23:59:59.999999",
            ),
            jiff_entry(
                "before-2000",
                "1999-12-31T23:59:59.9999995Z",
                "1999-12-31T23:59:59.9999995",
                "1999-12-31",
                "00:00:00",
            ),
        ];
        if driver == "sqlite" || driver == "postgres" {
            rows.push(jiff_entry(
                "ancient",
                "0001-01-01T00:00:00.000001Z",
                "0001-01-01T00:00:00",
                "0001-01-01",
                "00:00:00.000001",
            ));
        }
        rows[0].ends_at = Some("2026-10-15T11:30:00Z".parse().expect("instant"));

        let before = jiff::Timestamp::now();
        for row in &mut rows {
            row.save()
                .await
                .unwrap_or_else(|error| panic!("{driver} {}: {error}", row.title));
            let mut twin = chrono_entry(row);
            twin.save()
                .await
                .unwrap_or_else(|error| panic!("{driver} {}: {error}", twin.title));
        }
        let after = jiff::Timestamp::now();

        // Each type reads every row, whichever type wrote it, as the same
        // value.
        let as_jiff = JiffEntry::query()
            .order_by("id")
            .get()
            .await
            .expect("read as jiff");
        let as_chrono = ChronoEntry::query()
            .order_by("id")
            .get()
            .await
            .expect("read as chrono");
        assert_eq!(as_jiff.len(), rows.len() * 2, "{driver}");
        for (index, row) in rows.iter().enumerate() {
            for read in &as_jiff[index * 2..index * 2 + 2] {
                assert_eq!(
                    read.starts_at,
                    stored(driver, row.starts_at),
                    "{driver} {}",
                    read.title
                );
                assert_eq!(
                    read.local_start,
                    stored_civil(driver, row.local_start),
                    "{driver} {}",
                    read.title
                );
                assert_eq!(read.day, row.day, "{driver} {}", read.title);
                assert_eq!(read.opens, row.opens, "{driver} {}", read.title);
                assert_eq!(
                    read.ends_at,
                    row.ends_at.map(|at| stored(driver, at)),
                    "{driver}"
                );
                let created = read.created_at.expect("database default");
                assert!(
                    *created > before - SignedDuration::from_mins(10)
                        && *created < after + SignedDuration::from_mins(10),
                    "{driver}: created_at {created} outside {before}..{after}"
                );
            }
        }
        for (jiff_read, chrono_read) in as_jiff.iter().zip(&as_chrono) {
            assert_eq!(
                chrono(jiff_read.starts_at),
                chrono_read.starts_at,
                "{driver}"
            );
            assert_eq!(
                naive(jiff_read.local_start),
                chrono_read.local_start,
                "{driver}"
            );
            assert_eq!(
                jiff_read.day.to_string(),
                chrono_read.day.to_string(),
                "{driver}"
            );
            assert_eq!(
                jiff_read.created_at.map(chrono),
                chrono_read.created_at,
                "{driver}"
            );
        }

        // Filters bind the same parameter for either type, and order over
        // rows written by both.
        let ordinary = &rows[0];
        let by_jiff = JiffEntry::query()
            .where_eq("starts_at", stored(driver, ordinary.starts_at))
            .order_by("id")
            .get()
            .await
            .expect("filter by an instant");
        assert_eq!(
            titles(&by_jiff, |entry| &entry.title),
            ["ordinary", "ordinary (chrono)"],
            "{driver}"
        );
        let later = JiffEntry::query()
            .where_gt("starts_at", ordinary.starts_at)
            .order_by("id")
            .get()
            .await
            .expect("order by an instant");
        assert_eq!(
            titles(&later, |entry| &entry.title),
            ["after-2038", "after-2038 (chrono)"],
            "{driver}"
        );
        // Ascending instants, each pair (one row per type) adjacent.
        let ascending = ChronoEntry::query()
            .order_by("starts_at")
            .get()
            .await
            .expect("order mixed rows");
        let mut expected = vec!["before-2000", "ordinary", "after-2038"];
        if rows.len() == 4 {
            expected.insert(0, "ancient");
        }
        let mut order: Vec<String> = ascending
            .iter()
            .map(|entry| entry.title.trim_end_matches(" (chrono)").to_string())
            .collect();
        order.dedup();
        assert_eq!(order, expected, "{driver}");
        let on_day = ChronoEntry::query()
            .where_eq("day", ordinary.day)
            .where_eq("opens", ordinary.opens)
            .where_lt(
                "local_start",
                DateTime::from(
                    ordinary
                        .local_start
                        .checked_add(SignedDuration::from_secs(1))
                        .expect("later"),
                ),
            )
            .order_by("id")
            .get()
            .await
            .expect("filter by date, time and date-time");
        assert_eq!(
            titles(&on_day, |entry| &entry.title),
            ["ordinary", "ordinary (chrono)"],
            "{driver}"
        );
        let ended =
            Orm::raw("SELECT title FROM contract_jiff_calendar WHERE ends_at = ? ORDER BY id")
                .bind(ordinary.ends_at)
                .map_to::<(String,)>()
                .await
                .expect("raw filter by an instant");
        assert_eq!(
            ended,
            [
                ("ordinary".to_string(),),
                ("ordinary (chrono)".to_string(),)
            ],
            "{driver}"
        );

        if driver == "sqlite" {
            // Both wrote the same text.
            let stored_text: Vec<(String, String, String, String)> = rullst_orm::sqlx::query_as(
                "SELECT starts_at, local_start, day, opens FROM contract_jiff_calendar ORDER BY id",
            )
            .fetch_all(Orm::pool().expect("pool"))
            .await
            .expect("read stored text");
            for pair in stored_text.chunks(2) {
                assert_eq!(pair[0], pair[1], "sqlite");
            }
            assert_eq!(stored_text[0].0, "2026-10-15T09:30:00.123456+00:00");
            assert_eq!(stored_text[0].1, "2026-02-28 06:30:00.250");
            assert_eq!(stored_text[2].0, "2040-02-29T23:59:59.500+00:00");
            assert_eq!(stored_text[4].0, "1999-12-31T23:59:59.999999500+00:00");
        }

        // Updates through either type.
        let mut first = as_jiff[0].clone();
        first.ends_at = Some("2026-10-15T12:00:00Z".parse().expect("instant"));
        first.save().await.expect("full-row save");
        let reread = ChronoEntry::find(first.id)
            .await
            .expect("find")
            .expect("row");
        assert_eq!(reread.ends_at, first.ends_at.map(chrono), "{driver}");
        assert_eq!(
            reread.created_at,
            first.created_at.map(chrono),
            "{driver} keeps created_at"
        );

        Schema::drop_if_exists(TABLE).await.expect("drop calendar");
    }
}

#[cfg(not(any(
    feature = "strict-postgres",
    feature = "strict-mysql",
    feature = "strict-sqlite"
)))]
mod any_pool {
    use rullst_orm::chrono::{TimeZone, Utc};
    use rullst_orm::{Orm, RullstValue, Timestamp};

    /// The `Any` pool binds the jiff-based values as the same text as the
    /// `chrono` ones, so they filter SQLite text columns the same way.
    pub(super) async fn exercise(driver: &str) {
        if driver != "sqlite" {
            return;
        }
        Orm::raw("CREATE TABLE contract_jiff_text (id INTEGER PRIMARY KEY, starts_at TEXT)")
            .execute()
            .await
            .expect("create table");
        let at: Timestamp = "2026-10-15T09:30:00.5Z".parse().expect("instant");
        let chrono_at = Utc
            .timestamp_opt(1_792_056_600, 500_000_000)
            .single()
            .expect("instant");
        Orm::raw("INSERT INTO contract_jiff_text (starts_at) VALUES (?), (?)")
            .bind(at)
            .bind(RullstValue::DateTime(chrono_at))
            .execute()
            .await
            .expect("insert text");
        let rows = Orm::raw("SELECT starts_at FROM contract_jiff_text WHERE starts_at = ?")
            .bind(at)
            .map_to::<(String,)>()
            .await
            .expect("filter text");
        assert_eq!(
            rows,
            [
                ("2026-10-15T09:30:00.500+00:00".to_string(),),
                ("2026-10-15T09:30:00.500+00:00".to_string(),)
            ]
        );
        Orm::raw("DROP TABLE contract_jiff_text")
            .execute()
            .await
            .expect("drop table");
    }
}
