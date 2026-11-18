//! The ORM's date/time types against the golden corpora under
//! `src/temporal/golden/`, recorded from chrono and SQLx's `chrono` codecs
//! before `chrono` left the ORM (the generator, `generate_golden_vectors`, is
//! in the commit that recorded them): text readings, `FromStr`, the text and
//! encodings of values, binary and SQLite storage-class readings, type
//! descriptors and compatible column types, and Redis query-cache keys. Each
//! line is a JSON array; values are jiff's text forms (`null` where chrono
//! refused the input or the value is outside jiff's range).

use std::fmt::Write as _;

use serde_json::{Value, json};
use sqlx::{Encode, MySql, Postgres, Sqlite, Type};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut text, byte| {
        let _ = write!(text, "{byte:02x}");
        text
    })
}

fn sqlite_text<'q, T: Encode<'q, Sqlite>>(value: T) -> String {
    let mut buffer = sqlx::sqlite::SqliteArgumentsBuffer::default();
    let _ = value.encode_by_ref(&mut buffer).expect("encodes");
    format!("{buffer:?}")
}

fn postgres_bytes<'q, T: Encode<'q, Postgres>>(value: T) -> Value {
    let mut buffer = sqlx::postgres::PgArgumentBuffer::default();
    let _ = value.encode_by_ref(&mut buffer).expect("encodes");
    json!([hex(&buffer), value.size_hint()])
}

fn mysql_bytes<'q, T: Encode<'q, MySql>>(value: T) -> Value {
    let mut buffer = Vec::new();
    match value.encode_by_ref(&mut buffer) {
        Ok(_) => json!([hex(&buffer), value.size_hint()]),
        Err(_) => Value::Null,
    }
}

/// The other SQLx types whose descriptors the compatibility rows test; the
/// ORM's date/time types stood for chrono's, whose descriptors are the same.
pub(super) fn sqlite_samples() -> Vec<sqlx::sqlite::SqliteTypeInfo> {
    vec![
        <str as Type<Sqlite>>::type_info(),
        <i64 as Type<Sqlite>>::type_info(),
        <i32 as Type<Sqlite>>::type_info(),
        <f64 as Type<Sqlite>>::type_info(),
        <bool as Type<Sqlite>>::type_info(),
        <Vec<u8> as Type<Sqlite>>::type_info(),
        <super::DateTime as Type<Sqlite>>::type_info(),
        <super::Date as Type<Sqlite>>::type_info(),
        <super::Time as Type<Sqlite>>::type_info(),
    ]
}

pub(super) fn postgres_samples() -> Vec<sqlx::postgres::PgTypeInfo> {
    vec![
        <super::Timestamp as Type<Postgres>>::type_info(),
        <super::DateTime as Type<Postgres>>::type_info(),
        <super::Date as Type<Postgres>>::type_info(),
        <super::Time as Type<Postgres>>::type_info(),
        <String as Type<Postgres>>::type_info(),
        <i64 as Type<Postgres>>::type_info(),
        <i32 as Type<Postgres>>::type_info(),
    ]
}

pub(super) fn mysql_samples() -> Vec<sqlx::mysql::MySqlTypeInfo> {
    vec![
        <super::Timestamp as Type<MySql>>::type_info(),
        <super::DateTime as Type<MySql>>::type_info(),
        <super::Date as Type<MySql>>::type_info(),
        <super::Time as Type<MySql>>::type_info(),
        <String as Type<MySql>>::type_info(),
        <i64 as Type<MySql>>::type_info(),
        <u8 as Type<MySql>>::type_info(),
        <Vec<u8> as Type<MySql>>::type_info(),
    ]
}

// The checks: the ORM's types against the corpora.

mod checks {
    use serde_json::Value;
    use sqlx::{Encode, MySql, Postgres, Sqlite, Type};

    use super::super::{Date, DateTime, Time, Timestamp, mysql, parse, postgres, sqlite};
    use super::{hex, mysql_bytes, postgres_bytes, sqlite_text};

    fn corpus(text: &str) -> Vec<Vec<Value>> {
        text.lines()
            .map(|line| serde_json::from_str(line).expect("a golden line"))
            .collect()
    }

    fn string(value: &Value) -> Option<String> {
        value.as_str().map(str::to_string)
    }

    fn shown<T: ToString>(value: Option<T>) -> Option<String> {
        value.map(|value| value.to_string())
    }

    #[test]
    fn text_is_read_as_chrono_read_it() {
        for line in corpus(include_str!("golden/text_readings.jsonl")) {
            let input = line[1].as_str().expect("input");
            match line[0].as_str() {
                Some("sqlite_date_time") => {
                    let moment = parse::sqlite_date_time(input);
                    assert_eq!(
                        shown(moment.as_ref().map(|moment| moment.local)),
                        string(&line[2]),
                        "{input:?}"
                    );
                    assert_eq!(
                        shown(moment.and_then(|moment| moment.instant())),
                        string(&line[3]),
                        "{input:?}"
                    );
                }
                Some("sqlite_time") => {
                    assert_eq!(
                        shown(parse::sqlite_time(input)),
                        string(&line[2]),
                        "{input:?}"
                    );
                }
                _ => {
                    let plain = parse::parse(input, parse::DATE_TIME).and_then(|p| p.date_time());
                    assert_eq!(shown(plain), string(&line[2]), "{input:?}");
                    let with_offset = parse::parse(input, parse::DATE_TIME_OFFSET);
                    let local = with_offset.as_ref().and_then(|p| p.date_time());
                    assert_eq!(shown(local), string(&line[3]), "{input:?}");
                    assert_eq!(
                        shown(postgres::moment_from_text(input).and_then(|m| m.instant())),
                        string(&line[4]),
                        "{input:?}"
                    );
                    let date = parse::parse(input, parse::DATE_ONLY).and_then(|p| p.date());
                    assert_eq!(shown(date), string(&line[5]), "{input:?}");
                    let time = parse::parse(input, parse::TIME).and_then(|p| p.time());
                    assert_eq!(shown(time), string(&line[6]), "{input:?}");
                }
            }
        }
    }

    #[test]
    fn from_str_reads_what_chrono_read_then_jiff() {
        for line in corpus(include_str!("golden/from_str.jsonl")) {
            let input = line[1].as_str().expect("input");
            let read = match line[0].as_str() {
                Some("timestamp") => shown(input.parse::<Timestamp>().ok().map(|at| *at)),
                Some("date_time") => shown(input.parse::<DateTime>().ok().map(|at| *at)),
                Some("date") => shown(input.parse::<Date>().ok().map(|at| *at)),
                _ => shown(input.parse::<Time>().ok().map(|at| *at)),
            };
            assert_eq!(read, string(&line[2]), "{} {input:?}", line[0]);
        }
    }

    /// Display, Debug, serde and the three drivers' encodings.
    fn check_written<'q, T>(line: &[Value], value: T, size_hints: bool)
    where
        T: Encode<'q, Sqlite> + Encode<'q, Postgres> + Encode<'q, MySql>,
        T: std::fmt::Display + std::fmt::Debug + serde::Serialize + Clone,
    {
        assert_eq!(Some(value.to_string()), string(&line[2]));
        assert_eq!(Some(format!("{value:?}")), string(&line[3]));
        assert_eq!(
            Some(serde_json::to_string(&value).unwrap()),
            string(&line[4])
        );
        assert_eq!(Some(sqlite_text(value.clone())), string(&line[5]));
        assert_eq!(postgres_bytes(value.clone()), line[6]);
        let mysql = mysql_bytes(value);
        if size_hints {
            assert_eq!(mysql, line[7]);
        } else {
            assert_eq!(mysql.get(0), line[7].get(0));
        }
    }

    #[test]
    fn values_are_written_as_chrono_wrote_them() {
        for line in corpus(include_str!("golden/values.jsonl")) {
            let key = line[1].as_str().expect("key");
            match line[0].as_str() {
                // chrono's `DateTime<Utc>` hinted its own size on MySQL.
                Some("timestamp") => {
                    check_written(
                        &line,
                        Timestamp::from(key.parse::<jiff::Timestamp>().unwrap()),
                        false,
                    );
                }
                Some("date_time") => {
                    check_written(
                        &line,
                        DateTime::from(key.parse::<jiff::civil::DateTime>().unwrap()),
                        true,
                    );
                }
                Some("date") => check_written(
                    &line,
                    Date::from(key.parse::<jiff::civil::Date>().unwrap()),
                    true,
                ),
                _ => check_written(
                    &line,
                    Time::from(key.parse::<jiff::civil::Time>().unwrap()),
                    true,
                ),
            }
        }
    }

    #[test]
    fn binary_values_are_read_as_chrono_read_them() {
        use sqlx::mysql::types::{MySqlTime, MySqlTimeSign};
        for line in corpus(include_str!("golden/binary_readings.jsonl")) {
            match line[0].as_str() {
                Some("pg_micros") => {
                    let micros = line[1].as_i64().expect("micros");
                    assert_eq!(
                        shown(postgres::instant_from_micros(micros)),
                        string(&line[2]),
                        "{micros}"
                    );
                    assert_eq!(
                        shown(postgres::date_time_from_micros(micros)),
                        string(&line[3]),
                        "{micros}"
                    );
                    assert_eq!(
                        Some(postgres::time_from_micros(micros).to_string()),
                        string(&line[4]),
                        "{micros}"
                    );
                }
                Some("pg_days") => {
                    let days = i32::try_from(line[1].as_i64().expect("days")).expect("i32");
                    assert_eq!(
                        shown(postgres::date_from_days(days)),
                        string(&line[2]),
                        "{days}"
                    );
                }
                Some("mysql_binary") => {
                    let text = line[1].as_str().expect("hex");
                    let bytes: Vec<u8> = (0..text.len())
                        .step_by(2)
                        .map(|index| u8::from_str_radix(&text[index..index + 2], 16).unwrap())
                        .collect();
                    assert_eq!(hex(&bytes), text);
                    assert_eq!(
                        shown(mysql::date_time_from_binary(&bytes).ok()),
                        string(&line[2]),
                        "{text}"
                    );
                }
                Some("mysql_time") => {
                    let sign = if line[1].as_bool().unwrap() {
                        MySqlTimeSign::Negative
                    } else {
                        MySqlTimeSign::Positive
                    };
                    let field = |index: usize| line[index].as_u64().unwrap();
                    let value = MySqlTime::new(
                        sign,
                        u32::try_from(field(2)).unwrap(),
                        u8::try_from(field(3)).unwrap(),
                        u8::try_from(field(4)).unwrap(),
                        u32::try_from(field(5)).unwrap(),
                    )
                    .expect("a MySqlTime");
                    assert_eq!(
                        shown(mysql::time_from_mysql_time(&value)),
                        string(&line[6]),
                        "{value}"
                    );
                }
                Some("sqlite_integer") => {
                    let seconds = line[1].as_i64().expect("seconds");
                    let moment = sqlite::from_unix_seconds(seconds);
                    assert_eq!(
                        shown(moment.as_ref().map(|m| m.local)),
                        string(&line[2]),
                        "{seconds}"
                    );
                    assert_eq!(
                        shown(moment.and_then(|m| m.instant())),
                        string(&line[3]),
                        "{seconds}"
                    );
                }
                _ => {
                    let bits: u64 = line[1].as_str().unwrap().parse().unwrap();
                    let moment = sqlite::from_julian_day(f64::from_bits(bits));
                    assert_eq!(
                        shown(moment.as_ref().map(|m| m.local)),
                        string(&line[2]),
                        "{bits}"
                    );
                    assert_eq!(
                        shown(moment.and_then(|m| m.instant())),
                        string(&line[3]),
                        "{bits}"
                    );
                }
            }
        }
    }

    fn descriptor<D: sqlx::Database, T: Type<D>>(others: &[D::TypeInfo]) -> (String, Vec<bool>)
    where
        D::TypeInfo: std::fmt::Debug,
    {
        (
            format!("{:?}", T::type_info()),
            others.iter().map(|other| T::compatible(other)).collect(),
        )
    }

    #[test]
    fn type_descriptors_are_chronos() {
        use sqlx::postgres::PgHasArrayType;
        let sqlite = super::sqlite_samples();
        let pg = super::postgres_samples();
        let my = super::mysql_samples();
        for line in corpus(include_str!("golden/type_descriptors.jsonl")) {
            let ours = match (line[0].as_str().unwrap(), line[1].as_str().unwrap_or("")) {
                ("sqlite", "timestamp") => descriptor::<Sqlite, Timestamp>(&sqlite),
                ("sqlite", "date_time") => descriptor::<Sqlite, DateTime>(&sqlite),
                ("sqlite", "date") => descriptor::<Sqlite, Date>(&sqlite),
                ("sqlite", "time") => descriptor::<Sqlite, Time>(&sqlite),
                ("postgres", "timestamp") => descriptor::<Postgres, Timestamp>(&pg),
                ("postgres", "date_time") => descriptor::<Postgres, DateTime>(&pg),
                ("postgres", "date") => descriptor::<Postgres, Date>(&pg),
                ("postgres", "time") => descriptor::<Postgres, Time>(&pg),
                ("mysql", "timestamp") => descriptor::<MySql, Timestamp>(&my),
                ("mysql", "date_time") => descriptor::<MySql, DateTime>(&my),
                ("mysql", "date") => descriptor::<MySql, Date>(&my),
                ("mysql", "time") => descriptor::<MySql, Time>(&my),
                _ => {
                    let arrays = format!(
                        "{:?}",
                        [
                            Timestamp::array_type_info(),
                            DateTime::array_type_info(),
                            Date::array_type_info(),
                            Time::array_type_info(),
                        ]
                    );
                    assert_eq!(Some(arrays), string(&line[1]));
                    continue;
                }
            };
            assert_eq!(Some(ours.0), string(&line[2]), "{line:?}");
            let expected: Vec<bool> = serde_json::from_value(line[3].clone()).unwrap();
            assert_eq!(ours.1, expected, "{line:?}");
        }
    }

    #[tokio::test]
    async fn sqlite_storage_classes_are_read_as_chrono_read_them() {
        use sqlx::{ConnectOptions, Row};
        let mut connection = sqlx::sqlite::SqliteConnectOptions::new()
            .in_memory(true)
            .connect()
            .await
            .expect("in-memory SQLite");
        for line in corpus(include_str!("golden/sqlite_readings.jsonl")) {
            let sql = line[0].as_str().expect("sql");
            let row = sqlx::query(sqlx::AssertSqlSafe(format!("SELECT {sql}")))
                .fetch_one(&mut connection)
                .await
                .expect("select");
            assert_eq!(
                shown(row.try_get::<Timestamp, _>(0).ok().map(|at| *at)),
                string(&line[1]),
                "{sql}"
            );
            assert_eq!(
                shown(row.try_get::<DateTime, _>(0).ok().map(|at| *at)),
                string(&line[2]),
                "{sql}"
            );
            assert_eq!(
                shown(row.try_get::<Date, _>(0).ok().map(|at| *at)),
                string(&line[3]),
                "{sql}"
            );
            assert_eq!(
                shown(row.try_get::<Time, _>(0).ok().map(|at| *at)),
                string(&line[4]),
                "{sql}"
            );
        }
    }

    #[cfg(feature = "redis")]
    #[test]
    fn cache_keys_are_the_chrono_variants_keys() {
        use crate::RullstValue;
        for line in corpus(include_str!("golden/cache_keys.jsonl")) {
            let Some(key) = line[1].as_str() else {
                continue;
            };
            let value = match line[0].as_str() {
                Some("timestamp") => {
                    RullstValue::Timestamp(Timestamp::from(key.parse::<jiff::Timestamp>().unwrap()))
                }
                Some("date_time") => RullstValue::CivilDateTime(DateTime::from(
                    key.parse::<jiff::civil::DateTime>().unwrap(),
                )),
                Some("date") => {
                    RullstValue::CivilDate(Date::from(key.parse::<jiff::civil::Date>().unwrap()))
                }
                _ => RullstValue::CivilTime(Time::from(key.parse::<jiff::civil::Time>().unwrap())),
            };
            assert_eq!(
                Some(crate::query_cache::golden_key(value)),
                string(&line[2]),
                "{line:?}"
            );
        }
    }
}
