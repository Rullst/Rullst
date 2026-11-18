//! The date/time types (`rullst_orm::Timestamp`, `DateTime`, `Date`, `Time`)
//! on a live database: rows written by a model and by `Orm::raw` bindings read
//! back as the same values, filter and order together, and the database
//! stores what it stored for the `chrono` types they replaced (a golden list
//! of each server's text, checked against chrono-written rows before chrono
//! left the ORM).

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
    use rullst_orm::jiff::{self, SignedDuration, civil};
    use rullst_orm::schema::{Blueprint, Schema};
    use rullst_orm::{Date, DateTime, Orm, RullstValue, Time, Timestamp};

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

    /// Writes `entry` again through `Orm::raw`, binding each value as a
    /// `RullstValue` (on MySQL/MariaDB an instant goes as a `DATETIME`).
    async fn insert_raw(driver: &str, entry: &JiffEntry) {
        let placeholders = if driver == "postgres" {
            "$1, $2, $3, $4, $5, $6"
        } else {
            "?, ?, ?, ?, ?, ?"
        };
        Orm::raw(format!(
            "INSERT INTO contract_jiff_calendar (title, starts_at, local_start, day, opens, ends_at) VALUES ({placeholders})"
        ))
        .bind(format!("{} (raw)", entry.title))
        .bind(RullstValue::from(entry.starts_at))
        .bind(entry.local_start)
        .bind(entry.day)
        .bind(entry.opens)
        .bind(entry.ends_at)
        .execute()
        .await
        .unwrap_or_else(|error| panic!("{driver} {} (raw): {error}", entry.title));
    }

    /// The text each server keeps for the rows of `exercise`, in order:
    /// `starts_at`, `local_start`, `day`, `opens`.
    fn stored_golden(driver: &str) -> Vec<[&'static str; 4]> {
        match driver {
            "postgres" => vec![
                [
                    "2026-10-15 09:30:00.123456+00",
                    "2026-02-28 06:30:00.25",
                    "2026-02-28",
                    "08:15:30.5",
                ],
                [
                    "2040-02-29 23:59:59.5+00",
                    "2040-02-29 23:59:59.999999",
                    "2040-02-29",
                    "23:59:59.999999",
                ],
                [
                    "2000-01-01 00:00:00+00",
                    "2000-01-01 00:00:00",
                    "1999-12-31",
                    "00:00:00",
                ],
                [
                    "0001-01-01 00:00:00.000001+00",
                    "0001-01-01 00:00:00",
                    "0001-01-01",
                    "00:00:00.000001",
                ],
            ],
            "mysql" => vec![
                [
                    "2026-10-15 09:30:00.123456",
                    "2026-02-28 06:30:00.250000",
                    "2026-02-28",
                    "08:15:30.500000",
                ],
                [
                    "2040-02-29 23:59:59.500000",
                    "2040-02-29 23:59:59.999999",
                    "2040-02-29",
                    "23:59:59.999999",
                ],
                [
                    "1999-12-31 23:59:59.999999",
                    "1999-12-31 23:59:59.999999",
                    "1999-12-31",
                    "00:00:00.000000",
                ],
            ],
            _ => vec![
                [
                    "2026-10-15T09:30:00.123456+00:00",
                    "2026-02-28 06:30:00.250",
                    "2026-02-28",
                    "08:15:30.500",
                ],
                [
                    "2040-02-29T23:59:59.500+00:00",
                    "2040-02-29 23:59:59.999999",
                    "2040-02-29",
                    "23:59:59.999999",
                ],
                [
                    "1999-12-31T23:59:59.999999500+00:00",
                    "1999-12-31 23:59:59.999999500",
                    "1999-12-31",
                    "00:00:00",
                ],
                [
                    "0001-01-01T00:00:00.000001+00:00",
                    "0001-01-01 00:00:00",
                    "0001-01-01",
                    "00:00:00.000001",
                ],
            ],
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
            insert_raw(driver, row).await;
        }
        let after = jiff::Timestamp::now();

        // Every row reads back as the value written, whichever path wrote it.
        let as_jiff = JiffEntry::query()
            .order_by("id")
            .get()
            .await
            .expect("read as jiff");
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
        // chrono's text: serde (`to_json()`), `Display` and `Debug`.
        let first = &as_jiff[0];
        for field in [
            "\"starts_at\":\"2026-10-15T09:30:00.123456Z\"",
            "\"local_start\":\"2026-02-28T06:30:00.250\"",
            "\"day\":\"2026-02-28\"",
            "\"opens\":\"08:15:30.500\"",
        ] {
            assert!(
                first.to_json().contains(field),
                "{driver}: {}",
                first.to_json()
            );
        }
        assert_eq!(
            format!(
                "{} | {:?} | {}",
                first.starts_at, first.starts_at, first.local_start
            ),
            "2026-10-15 09:30:00.123456 UTC | 2026-10-15T09:30:00.123456Z | 2026-02-28 06:30:00.250",
            "{driver}"
        );

        // Text-protocol results (`raw_sql`, PostgreSQL's and MySQL's simple
        // queries) read as the binary ones, by either type.
        let text_rows = rullst_orm::sqlx::raw_sql(
            "SELECT starts_at, local_start, day, opens FROM contract_jiff_calendar ORDER BY id",
        )
        .fetch_all(Orm::pool().expect("pool"))
        .await
        .expect("text-protocol rows");
        assert_eq!(text_rows.len(), as_jiff.len(), "{driver}");
        for (row, read) in text_rows.iter().zip(&as_jiff) {
            use rullst_orm::sqlx::Row;
            let starts_at: Timestamp = row.try_get(0).expect("text instant");
            assert_eq!(starts_at, read.starts_at, "{driver} {}", read.title);
            let local_start: DateTime = row.try_get(1).expect("text date-time");
            assert_eq!(local_start, read.local_start, "{driver} {}", read.title);
            let day: Date = row.try_get(2).expect("text date");
            assert_eq!(day, read.day, "{driver} {}", read.title);
            let opens: Time = row.try_get(3).expect("text time");
            assert_eq!(opens, read.opens, "{driver} {}", read.title);
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
            ["ordinary", "ordinary (raw)"],
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
            ["after-2038", "after-2038 (raw)"],
            "{driver}"
        );
        // Ascending instants, each pair (one row per type) adjacent.
        let ascending = JiffEntry::query()
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
            .map(|entry| entry.title.trim_end_matches(" (raw)").to_string())
            .collect();
        order.dedup();
        assert_eq!(order, expected, "{driver}");
        let on_day = JiffEntry::query()
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
            ["ordinary", "ordinary (raw)"],
            "{driver}"
        );
        // `Orm::raw` passes its SQL through, so PostgreSQL numbers the
        // placeholder.
        let placeholder = if driver == "postgres" { "$1" } else { "?" };
        let ended = Orm::raw(format!(
            "SELECT title FROM contract_jiff_calendar WHERE ends_at = {placeholder} ORDER BY id"
        ))
        .bind(ordinary.ends_at)
        .map_to::<(String,)>()
        .await
        .expect("raw filter by an instant");
        assert_eq!(
            ended,
            [("ordinary".to_string(),), ("ordinary (raw)".to_string(),)],
            "{driver}"
        );

        // What each server stores, as its own text: the golden list, which
        // the chrono-written rows matched before chrono left the ORM.
        let cast = |column: &str| match driver {
            "postgres" => format!("{column}::text"),
            "mysql" => format!("CAST({column} AS CHAR)"),
            _ => column.to_string(),
        };
        let stored_text: Vec<(String, String, String, String)> =
            rullst_orm::sqlx::query_as(rullst_orm::sqlx::AssertSqlSafe(format!(
                "SELECT {}, {}, {}, {} FROM contract_jiff_calendar ORDER BY id",
                cast("starts_at"),
                cast("local_start"),
                cast("day"),
                cast("opens")
            )))
            .fetch_all(Orm::pool().expect("pool"))
            .await
            .expect("read stored text");
        let golden = stored_golden(driver);
        assert_eq!(stored_text.len(), golden.len() * 2, "{driver}");
        for (pair, expected) in stored_text.chunks(2).zip(golden) {
            for row in pair {
                assert_eq!(
                    [
                        row.0.as_str(),
                        row.1.as_str(),
                        row.2.as_str(),
                        row.3.as_str()
                    ],
                    expected,
                    "{driver}"
                );
            }
        }

        // A full-row save keeps the database default and writes the change.
        let mut first = as_jiff[0].clone();
        first.ends_at = Some("2026-10-15T12:00:00Z".parse().expect("instant"));
        first.save().await.expect("full-row save");
        let reread = JiffEntry::find(first.id).await.expect("find").expect("row");
        assert_eq!(reread.ends_at, first.ends_at, "{driver}");
        assert_eq!(
            reread.created_at, first.created_at,
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
    use rullst_orm::{Orm, RullstValue, Timestamp};

    /// The `Any` pool binds the values as RFC 3339 text, the form a SQLite
    /// `timestamp_tz()` column stores, so they filter SQLite text columns.
    pub(super) async fn exercise(driver: &str) {
        if driver != "sqlite" {
            return;
        }
        Orm::raw("CREATE TABLE contract_jiff_text (id INTEGER PRIMARY KEY, starts_at TEXT)")
            .execute()
            .await
            .expect("create table");
        let at: Timestamp = "2026-10-15T09:30:00.5Z".parse().expect("instant");
        Orm::raw("INSERT INTO contract_jiff_text (starts_at) VALUES (?), (?)")
            .bind(at)
            .bind(RullstValue::Timestamp(at))
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
