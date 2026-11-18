//! Records chrono's behaviour as golden corpora under `src/temporal/golden/`,
//! before `chrono` leaves the ORM: the readings of chrono's text parsers and
//! SQLx's `chrono` decoders, the bytes and text SQLx's `chrono` encoders and
//! chrono's formatting write, the type descriptors and compatible column types,
//! and the Redis query-cache keys of the `chrono` `RullstValue` variants.
//!
//! Run with every feature (`cargo test -p rullst-orm --all-features --lib
//! generate_golden_vectors -- --ignored`). Inputs come from the differential
//! strategies with a fixed ChaCha seed, so a rerun writes the same files. Each
//! line is a JSON array; values are jiff's text forms (`null` where chrono
//! refused the input or the value is outside jiff's range).

use std::fmt::Write as _;
use std::io::Write as _;

use chrono::{DateTime as ChronoDateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime};
use chrono::{Offset as _, TimeZone, Utc};
use proptest::strategy::{Strategy, ValueTree};
use proptest::test_runner::{Config, RngAlgorithm, TestRng, TestRunner};
use serde_json::{Value, json};
use sqlx::{Encode, MySql, Postgres, Sqlite, Type};

use super::test_support::{
    chrono_date, chrono_date_time, chrono_instant, chrono_time, date, date_time, date_time_text,
    instant, jiff_date, jiff_date_time, jiff_instant, jiff_time, time, time_text,
};

const DIRECTORY: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src/temporal/golden");

fn runner() -> TestRunner {
    TestRunner::new_with_rng(
        Config::default(),
        TestRng::from_seed(RngAlgorithm::ChaCha, &[13; 32]),
    )
}

fn sample<S: Strategy>(runner: &mut TestRunner, strategy: &S, count: usize) -> Vec<S::Value> {
    (0..count)
        .map(|_| strategy.new_tree(runner).expect("a value").current())
        .collect()
}

fn write(name: &str, lines: &[Value]) {
    std::fs::create_dir_all(DIRECTORY).expect("golden directory");
    let mut file = std::fs::File::create(format!("{DIRECTORY}/{name}")).expect("golden file");
    for line in lines {
        writeln!(file, "{line}").expect("golden line");
    }
}

fn text<T: ToString>(value: Option<T>) -> Value {
    value.map_or(Value::Null, |value| Value::String(value.to_string()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut text, byte| {
        let _ = write!(text, "{byte:02x}");
        text
    })
}

/// sqlx-sqlite 0.9 `decode_datetime_from_text`.
fn chrono_sqlite_date_time(value: &str) -> Option<ChronoDateTime<FixedOffset>> {
    if let Ok(dt) = ChronoDateTime::parse_from_rfc3339(value) {
        return Some(dt);
    }
    for format in [
        "%F %T%.f",
        "%F %R",
        "%F %RZ",
        "%F %R%:z",
        "%F %T%.fZ",
        "%F %T%.f%:z",
        "%FT%R",
        "%FT%RZ",
        "%FT%R%:z",
        "%FT%T%.f",
        "%FT%T%.fZ",
        "%FT%T%.f%:z",
    ] {
        if let Ok(dt) = ChronoDateTime::parse_from_str(value, format) {
            return Some(dt);
        }
        if let Ok(dt) = NaiveDateTime::parse_from_str(value, format) {
            return Some(Utc.fix().from_utc_datetime(&dt));
        }
    }
    None
}

/// sqlx-sqlite 0.9's `NaiveTime` decoder.
fn chrono_sqlite_time(value: &str) -> Option<NaiveTime> {
    ["%T.f", "%T%.f", "%R", "%RZ", "%T%.fZ", "%R%:z", "%T%.f%:z"]
        .into_iter()
        .find_map(|format| NaiveTime::parse_from_str(value, format).ok())
}

/// sqlx-mysql 0.9's binary `NaiveDateTime` decoder, with its panics on short
/// buffers as `None`.
fn chrono_mysql_binary(buf: &[u8]) -> Option<NaiveDateTime> {
    let (&len, rest) = buf.split_first()?;
    if rest.len() < 4 {
        return None;
    }
    let date = NaiveDate::from_ymd_opt(
        i32::from(u16::from_le_bytes([rest[0], rest[1]])),
        u32::from(rest[2]),
        u32::from(rest[3]),
    )?;
    if len <= 4 {
        return date.and_hms_opt(0, 0, 0);
    }
    let [hour, minute, second, micros @ ..] = rest.get(4..)? else {
        return None;
    };
    let micros = if len > 7 {
        if micros.len() > 8 {
            return None;
        }
        let mut le = [0; 8];
        le[..micros.len()].copy_from_slice(micros);
        u32::try_from(u64::from_le_bytes(le)).ok()?
    } else {
        0
    };
    let time = NaiveTime::from_hms_micro_opt(
        u32::from(*hour),
        u32::from(*minute),
        u32::from(*second),
        micros,
    )?;
    Some(date.and_time(time))
}

fn server_readings(input: &str) -> Value {
    let plain = NaiveDateTime::parse_from_str(input, "%Y-%m-%d %H:%M:%S%.f").ok();
    let offset = NaiveDateTime::parse_from_str(input, "%Y-%m-%d %H:%M:%S%.f%#z").ok();
    let instant = ChronoDateTime::parse_from_str(input, "%Y-%m-%d %H:%M:%S%.f%#z").ok();
    json!([
        "server",
        input,
        text(plain.and_then(jiff_date_time)),
        text(offset.and_then(jiff_date_time)),
        text(instant.and_then(|at| jiff_instant(at.with_timezone(&Utc)))),
        text(
            NaiveDate::parse_from_str(input, "%Y-%m-%d")
                .ok()
                .and_then(jiff_date)
        ),
        text(
            NaiveTime::parse_from_str(input, "%H:%M:%S%.f")
                .ok()
                .and_then(jiff_time)
        ),
    ])
}

fn text_readings(runner: &mut TestRunner) -> Vec<Value> {
    let mut inputs = sample(runner, &date_time_text(), 700);
    inputs.extend(sample(runner, &time_text(), 250));
    inputs.extend(sample(runner, &"[0-9T:+\\-. Zz\u{2212}tf]{0,40}", 250));
    for at in sample(runner, &instant(), 150) {
        let theirs = chrono_instant(at);
        inputs.push(theirs.to_rfc3339_opts(chrono::SecondsFormat::AutoSi, false));
        inputs.push(theirs.naive_utc().format("%F %T%.f").to_string());
        inputs.push(theirs.time().format("%T%.f").to_string());
    }
    let mut lines = Vec::new();
    for input in &inputs {
        let theirs = chrono_sqlite_date_time(input);
        lines.push(json!([
            "sqlite_date_time",
            input,
            text(theirs.and_then(|dt| jiff_date_time(dt.naive_local()))),
            text(theirs.and_then(|dt| jiff_instant(dt.with_timezone(&Utc)))),
        ]));
        lines.push(json!([
            "sqlite_time",
            input,
            text(chrono_sqlite_time(input).and_then(jiff_time)),
        ]));
        lines.push(server_readings(input));
    }
    lines
}

/// chrono's value for every text chrono's `FromStr` read, else jiff's.
fn from_str_readings(runner: &mut TestRunner) -> Vec<Value> {
    let mut lines = Vec::new();
    let mut instants = sample(runner, &date_time_text(), 800);
    instants.extend(sample(runner, &"[0-9T:+\\-. ZzUTCu]{0,40}", 400));
    for input in instants {
        let expected = match input.parse::<ChronoDateTime<Utc>>().ok().map(jiff_instant) {
            Some(Some(value)) => Some(value),
            _ => input.parse::<jiff::Timestamp>().ok(),
        };
        lines.push(json!(["timestamp", input, text(expected)]));
    }
    let mut civils = sample(runner, &date_time_text(), 800);
    civils.extend(sample(runner, &"[0-9T:+\\-. ]{0,30}", 400));
    for input in civils {
        let expected = match input.parse::<NaiveDateTime>().ok().map(jiff_date_time) {
            Some(Some(value)) => Some(value),
            _ => input.parse::<jiff::civil::DateTime>().ok(),
        };
        lines.push(json!(["date_time", input, text(expected)]));
    }
    let mut dates = sample(runner, &super::tests::date_text(), 600);
    dates.extend(sample(runner, &"[0-9+\\- T]{0,16}", 300));
    for input in dates {
        let expected = match input.parse::<NaiveDate>().ok().map(jiff_date) {
            Some(Some(value)) => Some(value),
            _ => input.parse::<jiff::civil::Date>().ok(),
        };
        lines.push(json!(["date", input, text(expected)]));
    }
    let mut times = sample(runner, &time_text(), 600);
    times.extend(sample(runner, &"[0-9:. ]{0,16}", 300));
    for input in times {
        let expected = match input.parse::<NaiveTime>().ok().map(jiff_time) {
            Some(Some(value)) => Some(value),
            _ => input.parse::<jiff::civil::Time>().ok(),
        };
        lines.push(json!(["time", input, text(expected)]));
    }
    lines
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

/// Every chrono form of a value, keyed by jiff's text of it.
fn written_forms<'q, T>(kind: &str, key: String, value: T) -> Value
where
    T: Encode<'q, Sqlite> + Encode<'q, Postgres> + Encode<'q, MySql>,
    T: std::fmt::Display + std::fmt::Debug + serde::Serialize + Clone,
{
    json!([
        kind,
        key,
        value.to_string(),
        format!("{value:?}"),
        serde_json::to_string(&value).expect("serializes"),
        sqlite_text(value.clone()),
        postgres_bytes(value.clone()),
        mysql_bytes(value),
    ])
}

fn values(runner: &mut TestRunner) -> Vec<Value> {
    let mut lines = Vec::new();
    for at in sample(runner, &instant(), 500) {
        lines.push(written_forms(
            "timestamp",
            at.0.to_string(),
            chrono_instant(at),
        ));
    }
    for civil in sample(runner, &date_time(), 500) {
        lines.push(written_forms(
            "date_time",
            civil.0.to_string(),
            chrono_date_time(civil),
        ));
    }
    for day in sample(runner, &date(), 300) {
        lines.push(written_forms("date", day.0.to_string(), chrono_date(day.0)));
    }
    for clock in sample(runner, &time(), 300) {
        lines.push(written_forms(
            "time",
            clock.0.to_string(),
            chrono_time(clock.0),
        ));
    }
    lines
}

fn binary_readings(runner: &mut TestRunner) -> Vec<Value> {
    use chrono::Duration;
    use proptest::prelude::*;

    let epoch = NaiveDate::from_ymd_opt(2000, 1, 1)
        .and_then(|day| day.and_hms_opt(0, 0, 0))
        .expect("2000-01-01");
    let mut lines = Vec::new();
    let micros = prop_oneof![
        any::<i64>(),
        -400_000_000_000_000_000_i64..400_000_000_000_000_000,
        -1_000_000_i64..1_000_000,
    ];
    for micros in sample(runner, &micros, 700) {
        let theirs = epoch.checked_add_signed(Duration::microseconds(micros));
        let clock = NaiveTime::default()
            .overflowing_add_signed(Duration::microseconds(micros))
            .0;
        lines.push(json!([
            "pg_micros",
            micros,
            text(theirs.and_then(|at| jiff_instant(at.and_utc()))),
            text(theirs.and_then(jiff_date_time)),
            text(jiff_time(clock)),
        ]));
    }
    for days in sample(runner, &any::<i32>(), 300) {
        let theirs = epoch
            .date()
            .checked_add_signed(Duration::days(i64::from(days)));
        lines.push(json!(["pg_days", days, text(theirs.and_then(jiff_date))]));
    }
    let structured = (
        prop_oneof![Just(4_u8), Just(7), Just(11)],
        0_u16..=10_000,
        0_u8..=13,
        0_u8..=32,
        0_u8..=24,
        0_u8..=60,
        0_u8..=60,
        any::<u32>(),
    )
        .prop_map(|(len, year, month, day, hour, minute, second, micros)| {
            let mut bytes = vec![len];
            bytes.extend_from_slice(&year.to_le_bytes());
            bytes.extend_from_slice(&[month, day, hour, minute, second]);
            bytes.extend_from_slice(&(micros % 2_100_000).to_le_bytes());
            bytes.truncate(usize::from(len) + 1);
            bytes
        });
    let mut buffers = sample(runner, &structured, 600);
    buffers.extend(sample(
        runner,
        &prop::collection::vec(any::<u8>(), 0..16),
        600,
    ));
    for bytes in buffers {
        lines.push(json!([
            "mysql_binary",
            hex(&bytes),
            text(chrono_mysql_binary(&bytes).and_then(jiff_date_time)),
        ]));
    }
    let times = (
        any::<bool>(),
        0_u32..=840,
        0_u8..=60,
        0_u8..=60,
        0_u32..=1_000_000,
    );
    for (negative, hours, minutes, seconds, micros) in sample(runner, &times, 600) {
        use sqlx::mysql::types::{MySqlTime, MySqlTimeSign};
        let sign = if negative {
            MySqlTimeSign::Negative
        } else {
            MySqlTimeSign::Positive
        };
        if let Ok(value) = MySqlTime::new(sign, hours, minutes, seconds, micros) {
            lines.push(json!([
                "mysql_time",
                negative,
                hours,
                minutes,
                seconds,
                micros,
                text(NaiveTime::try_from(value).ok().and_then(jiff_time)),
            ]));
        }
    }
    for seconds in sample(runner, &any::<i64>(), 300) {
        let theirs = ChronoDateTime::<Utc>::from_timestamp(seconds, 0);
        lines.push(json!([
            "sqlite_integer",
            seconds,
            text(theirs.and_then(|at| jiff_date_time(at.naive_utc()))),
            text(theirs.and_then(jiff_instant)),
        ]));
    }
    let reals = prop_oneof![
        -1.0e7_f64..1.0e7,
        2_440_587.5_f64..2_500_000.0,
        any::<f64>()
    ];
    for day in sample(runner, &reals, 450) {
        let seconds = (day - 2_440_587.5) * 86_400.0;
        let theirs = seconds
            .is_finite()
            .then(|| {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let (whole, nanos) = (seconds.trunc() as i64, (seconds.fract() * 1E9).abs() as u32);
                ChronoDateTime::<Utc>::from_timestamp(whole, nanos)
            })
            .flatten();
        lines.push(json!([
            "sqlite_real",
            day.to_bits().to_string(),
            text(theirs.and_then(|at| jiff_date_time(at.naive_utc()))),
            text(theirs.and_then(jiff_instant)),
        ]));
    }
    lines
}

/// The type descriptors of SQLx's `chrono` codecs and whether each accepts
/// the descriptors of other SQLx types, in a fixed order.
fn type_descriptors() -> Vec<Value> {
    fn row<D: sqlx::Database, T: Type<D>>(db: &str, kind: &str, others: &[D::TypeInfo]) -> Value
    where
        D::TypeInfo: std::fmt::Debug,
    {
        json!([
            db,
            kind,
            format!("{:?}", T::type_info()),
            others
                .iter()
                .map(|other| T::compatible(other))
                .collect::<Vec<_>>(),
        ])
    }
    let sqlite = sqlite_samples();
    let postgres = postgres_samples();
    let mysql = mysql_samples();
    vec![
        row::<Sqlite, ChronoDateTime<Utc>>("sqlite", "timestamp", &sqlite),
        row::<Sqlite, NaiveDateTime>("sqlite", "date_time", &sqlite),
        row::<Sqlite, NaiveDate>("sqlite", "date", &sqlite),
        row::<Sqlite, NaiveTime>("sqlite", "time", &sqlite),
        row::<Postgres, ChronoDateTime<Utc>>("postgres", "timestamp", &postgres),
        row::<Postgres, NaiveDateTime>("postgres", "date_time", &postgres),
        row::<Postgres, NaiveDate>("postgres", "date", &postgres),
        row::<Postgres, NaiveTime>("postgres", "time", &postgres),
        row::<MySql, ChronoDateTime<Utc>>("mysql", "timestamp", &mysql),
        row::<MySql, NaiveDateTime>("mysql", "date_time", &mysql),
        row::<MySql, NaiveDate>("mysql", "date", &mysql),
        row::<MySql, NaiveTime>("mysql", "time", &mysql),
        json!([
            "postgres_arrays",
            format!(
                "{:?}",
                [
                    <ChronoDateTime<Utc> as sqlx::postgres::PgHasArrayType>::array_type_info(),
                    <NaiveDateTime as sqlx::postgres::PgHasArrayType>::array_type_info(),
                    <NaiveDate as sqlx::postgres::PgHasArrayType>::array_type_info(),
                    <NaiveTime as sqlx::postgres::PgHasArrayType>::array_type_info(),
                ]
            ),
        ]),
    ]
}

/// The other SQLx types whose descriptors the compatibility rows test; the
/// ORM's date/time types stand for chrono's, whose descriptors are the same.
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

/// The Redis query-cache keys of the `chrono` variants.
#[cfg(feature = "redis")]
fn cache_keys(runner: &mut TestRunner) -> Vec<Value> {
    use crate::RullstValue;
    let key = |value: RullstValue| crate::query_cache::golden_key(value);
    let mut lines = Vec::new();
    for at in sample(runner, &instant(), 100) {
        let naive = chrono_instant(at);
        lines.push(json!([
            "timestamp",
            at.0.to_string(),
            key(RullstValue::DateTime(naive))
        ]));
        let civil = naive.naive_utc();
        lines.push(json!([
            "date_time",
            jiff_date_time(civil).map(|value| value.to_string()),
            key(RullstValue::NaiveDateTime(civil))
        ]));
        lines.push(json!([
            "date",
            jiff_date(civil.date()).map(|value| value.to_string()),
            key(RullstValue::Date(civil.date()))
        ]));
        lines.push(json!([
            "time",
            jiff_time(civil.time()).map(|value| value.to_string()),
            key(RullstValue::Time(civil.time()))
        ]));
    }
    lines
}

/// What chrono read from a SQLite value of each storage class.
async fn sqlite_readings() -> Vec<Value> {
    use sqlx::{ConnectOptions, Row};
    let mut connection = sqlx::sqlite::SqliteConnectOptions::new()
        .in_memory(true)
        .connect()
        .await
        .expect("in-memory SQLite");
    let mut lines = Vec::new();
    for sql in [
        "'2026-10-15T09:30:00.123456+00:00'",
        "'2026-10-15 12:30:00+03:00'",
        "'2026-10-15 09:30'",
        "'2026-10-15'",
        "'09:30:00.5'",
        "'2016-12-31T23:59:60Z'",
        "'not a date'",
        "'+10000-01-01 00:00'",
        "'-10000-12-31T00:00:00Z'",
        "'9999-12-31 23:59:59'",
        "1792056600",
        "-62135596800",
        "253402300799",
        "253402300800",
        "-377705116800",
        "2461328.89583333",
        "5373484.0",
        "0.5",
        "x'00'",
        "NULL",
    ] {
        let row = sqlx::query(sqlx::AssertSqlSafe(format!("SELECT {sql}")))
            .fetch_one(&mut connection)
            .await
            .expect("select");
        lines.push(json!([
            sql,
            text(
                row.try_get::<ChronoDateTime<Utc>, _>(0)
                    .ok()
                    .and_then(jiff_instant)
            ),
            text(
                row.try_get::<NaiveDateTime, _>(0)
                    .ok()
                    .and_then(jiff_date_time)
            ),
            text(row.try_get::<NaiveDate, _>(0).ok().and_then(jiff_date)),
            text(row.try_get::<NaiveTime, _>(0).ok().and_then(jiff_time)),
        ]));
    }
    lines
}

#[tokio::test]
#[ignore = "writes src/temporal/golden/; run before changing the corpora"]
async fn generate_golden_vectors() {
    let mut runner = runner();
    write("text_readings.jsonl", &text_readings(&mut runner));
    write("from_str.jsonl", &from_str_readings(&mut runner));
    write("values.jsonl", &values(&mut runner));
    write("binary_readings.jsonl", &binary_readings(&mut runner));
    write("type_descriptors.jsonl", &type_descriptors());
    write("sqlite_readings.jsonl", &sqlite_readings().await);
    #[cfg(feature = "redis")]
    write("cache_keys.jsonl", &cache_keys(&mut runner));
}
