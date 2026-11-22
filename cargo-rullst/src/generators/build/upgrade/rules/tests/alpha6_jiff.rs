//! 13.0.0-alpha.6 replaces chrono with jiff in Rullst's APIs: the ORM's
//! model types and query values, Core's `Locale` formatting, Mail's
//! scheduled sends and account notices and Capital's invoice date. The rules
//! report chrono where it meets those APIs, never an application's own
//! chrono code.

use super::super::alpha6_jiff::ChronoBindings;
use super::super::files::manifest_findings;
use super::files::plan;
use super::*;
use proc_macro2::TokenStream;

const ORM: &str = "V13-ORM-CHRONO-TYPES";
const LOCALE: &str = "V13-LOCALE-JIFF-DATES";
const MAIL: &str = "V13-MAIL-JIFF-TIMESTAMPS";
const CAPITAL: &str = "V13-CAPITAL-INVOICE-JIFF-DATE";

/// The lines at which `code` is found in `source` with `facts`.
fn found_with(source: &str, code: &str, facts: &WorkspaceFacts) -> Vec<usize> {
    scan_with(source, "lib.rs", facts)
        .into_iter()
        .filter(|(found, _)| *found == code)
        .map(|(_, line)| line)
        .collect()
}

fn found(source: &str, code: &str) -> Vec<usize> {
    found_with(source, code, &WorkspaceFacts::default())
}

/// An application that declares chrono for its own code.
fn own_chrono() -> WorkspaceFacts {
    WorkspaceFacts {
        chrono_dependency: true,
        ..WorkspaceFacts::default()
    }
}

fn facts(manifest: &str) -> WorkspaceFacts {
    manifest_findings(&[plan(PathBuf::from("Cargo.toml"), manifest)]).1
}

#[test]
fn the_rules_are_alpha6_must_change_rules_on_one_row() {
    for (code, replacement) in [
        (ORM, "rullst::orm::Timestamp"),
        (LOCALE, "jiff::ZonedDateTime"),
        (MAIL, "SignedDuration::from_mins(5)"),
        (CAPITAL, "jiff::Timestamp::now()"),
    ] {
        let rule = CATALOG.iter().find(|rule| rule.code == code).unwrap();
        assert_eq!(rule.introduced, "13.0.0-alpha.6", "{code}");
        assert_eq!(rule.kind, FindingKind::MustChange, "{code}");
        assert_eq!(rule.row, "`chrono` replaced by `jiff`", "{code}");
        assert!(rule.message.contains("13.0.0-alpha.6"), "{code}");
        assert!(rule.guidance.contains(replacement), "{code}");
    }
}

#[test]
fn chrono_fields_of_orm_and_nexus_models_must_change_under_any_spelling() {
    let source = r#"
use chrono::{DateTime, FixedOffset, NaiveDate, Utc};
use chrono::NaiveTime as Clock;
type Stamp = chrono::DateTime<chrono::Utc>;
type Stamps = Vec<Stamp>;
#[derive(Debug, rullst::Orm)]
pub struct Event {
    pub id: i64,
    pub starts_at: DateTime<Utc>,
    pub ends_at: Option<chrono::DateTime<chrono::offset::Utc>>,
    pub day: NaiveDate,
    pub local: Option<rullst::orm::chrono::NaiveDateTime>,
    pub stamp: Stamp,
    pub opens: Clock,
    pub days: Vec<NaiveDate>,
    pub history: Stamps,
    pub offset_at: DateTime<FixedOffset>,
    pub created_at: rullst::orm::Timestamp,
    pub civil: Option<rullst::orm::DateTime>,
    pub name: String,
    #[sqlx(skip)]
    pub cached: Option<DateTime<Utc>>,
}
#[derive(Nexus)]
struct Panel {
    at: &'static NaiveTime,
    pair: (String, NaiveDate),
}
struct Plain {
    at: DateTime<Utc>,
}
mod nested {
    use chrono::NaiveDateTime as Moment;
    #[derive(Orm)]
    struct Visit {
        at: Moment,
        on: rullst::orm::Date,
    }
}
"#;
    assert_eq!(
        found_with(source, ORM, &own_chrono()),
        [9, 10, 11, 12, 13, 14, 15, 16, 17, 26, 27, 36]
    );
}

#[test]
fn paths_through_rullsts_or_sqlxs_chrono_must_change() {
    let source = r#"
use rullst::orm::chrono::{DateTime, Utc};
use rullst::orm::{chrono, Model};
use rullst::db::chrono::NaiveDate;
fn now() -> rullst_orm::chrono::DateTime<rullst_orm::chrono::Utc> { Utc::now() }
fn own() { let at = chrono::Utc::now(); }
fn read(at: sqlx::types::chrono::NaiveDate) {}
const TEXT: &str = "rullst::orm::chrono::Utc";
// rullst::orm::chrono::Utc
fn kept(at: rullst::orm::Timestamp) -> rullst::orm::jiff::Timestamp { *at }
fn other(at: other::types::chrono::Value, orm: my_orm::chrono::Value) {}
"#;
    // Without a chrono dependency, a bare `chrono::` path can only reach
    // Rullst's re-export.
    assert_eq!(found(source, ORM), [2, 3, 4, 5, 6, 7]);
    assert_eq!(found_with(source, ORM, &own_chrono()), [2, 3, 4, 5, 7]);
    let sqlx_chrono = WorkspaceFacts {
        sqlx_chrono: true,
        ..WorkspaceFacts::default()
    };
    assert_eq!(found_with(source, ORM, &sqlx_chrono), [2, 3, 4, 5]);
}

#[test]
fn chrono_values_given_to_rullst_value_must_change() {
    let source = r#"
use rullst::orm::RullstValue;
fn values(cutoff: DateTime<Utc>, day: rullst::orm::Date, mut since_day: NaiveDate) {
    let a = RullstValue::DateTime(Utc::now());
    let b = RullstValue::from(Utc::now());
    let mut since = Utc::now() - Duration::days(7);
    let rows = Post::query().where_gt("created_at", since);
    let rows = Post::query().where_eq("day", day);
    let rows = Post::query().where_between("at", cutoff, Local::now());
    let raw = Orm::raw("SELECT 1 WHERE at > ?").bind(NaiveDate::from_ymd_opt(2026, 1, 1));
    let mine = sqlx::query("SELECT 1 WHERE at > ?").bind(Utc::now());
    match value { RullstValue::NaiveDateTime(at) => {} RullstValue::Timestamp(at) => {} _ => {} }
    let kept = RullstValue::from(rullst::orm::Timestamp::now());
    let rows = Post::query().where_eq("driver", Driver::Local);
    let fixed = DateTime::from_timestamp(0, 0);
    let rows = Post::query().or_where_lt("at", fixed);
    let rows = Post::query().where_in("day", vec![since_day]);
    let civil = Post::query().where_eq("at", jiff::civil::DateTime::new(2026, 1, 1, 0, 0, 0, 0));
    if let Some(found) = maybe { Post::query().where_eq("id", Some(3)); }
    let typed: RullstValue = rullst::orm::Date::from(date).into();
    let rows = Post::query().where_raw("day = ?", vec![RullstValue::Date(day)]);
    let fresh = Post::query().where_gt("at", rullst::orm::Timestamp::now());
}
"#;
    assert_eq!(found(source, ORM), [4, 5, 7, 9, 10, 11, 12, 16, 17, 21]);
    // An application that enables SQLx's `chrono` feature keeps binding
    // chrono values with SQLx itself, not through the ORM's raw queries.
    let sqlx_chrono = WorkspaceFacts {
        sqlx_chrono: true,
        ..WorkspaceFacts::default()
    };
    assert_eq!(
        found_with(source, ORM, &sqlx_chrono),
        [4, 5, 7, 9, 10, 12, 16, 17, 21]
    );
}

#[test]
fn chrono_bindings_ignore_patterns_and_follow_earlier_bindings() {
    let bindings = alpha6_jiff_bindings(
        "fn f(&self, a: chrono::NaiveDate, b: i64, mut c: Option<DateTime<Utc>>, d: rullst::orm::DateTime) {\n    let e = a;\n    let mut g: TimeDelta = h;\n    let Some(i) = Utc::now().checked_sub(x) else { return };\n    if let Some(j) = Local::now() {}\n    let k = Local::new();\n    let l = DateTime::parse_from_rfc3339(text);\n    let m = DateTime::new(1);\n    let n = 3\n}",
    );
    for name in ["a", "c", "e", "g", "l"] {
        assert!(bindings.contains(name), "{name}: {bindings:?}");
    }
    for name in ["b", "d", "Some", "i", "j", "k", "m", "n", "self"] {
        assert!(!bindings.contains(name), "{name}: {bindings:?}");
    }
}

/// The names [`ChronoBindings`] finds in `source`, read back
/// through `carries_chrono`.
fn alpha6_jiff_bindings(source: &str) -> BTreeSet<String> {
    let stream: proc_macro2::TokenStream = source.parse().unwrap();
    let bindings = ChronoBindings::of(stream);
    [
        "a", "b", "c", "d", "e", "g", "i", "j", "k", "l", "m", "n", "self", "Some",
    ]
    .into_iter()
    .filter(|name| bindings.carries_chrono(&name.parse().unwrap()))
    .map(str::to_string)
    .collect()
}

#[test]
fn locale_formatting_with_rullst_styles_must_change() {
    let source = r#"
use rullst::i18n::{DateStyle, Locale, TimeStyle};
fn show(locale: &Locale, at: &chrono::NaiveDate, report: &Report) {
    locale.format_date(at, DateStyle::Short);
    locale.format_time(&time, TimeStyle::Short);
    locale.format_datetime(&value, DateStyle::Long, TimeStyle::Short);
    report.format_date(at);
    report.format_time(at, "%H:%M");
}
"#;
    assert_eq!(found_with(source, LOCALE, &own_chrono()), [4, 5, 6]);
}

/// Code that finished the move to jiff passes jiff values, which the rule
/// recognises by their types and paths and leaves alone, also with
/// `--all-rules` (#844).
#[test]
fn locale_formatting_of_jiff_values_needs_no_change() {
    let source = r#"
use rullst::i18n::{DateStyle, Locale, TimeStyle};
use jiff::{Timestamp, Zoned, civil::Date};
struct Event { starts_at: rullst::orm::Timestamp, day: Date }
fn today() -> Zoned { Zoned::now() }
fn show(locale: &Locale, day: Date, zoned: &Zoned, event: &Event, at: rullst::orm::Timestamp) {
    locale.format_date(day, DateStyle::Short);
    locale.format_datetime(zoned, DateStyle::Long, TimeStyle::Short);
    locale.format_date(jiff::civil::date(2026, 11, 21), DateStyle::Long);
    locale.format_time(&Zoned::now(), TimeStyle::Short);
    let now = Timestamp::now().in_tz("America/Sao_Paulo")?;
    locale.format_datetime(&now, DateStyle::Short, TimeStyle::Short);
    locale.format_date(event.day, DateStyle::Short);
    locale.format_datetime(&at.in_tz("UTC")?, DateStyle::Short, TimeStyle::Short);
    locale.format_date(&today(), DateStyle::Short);
    let local: jiff::civil::DateTime = event.starts_at.in_tz("UTC")?.datetime();
    locale.format_datetime(local, DateStyle::Short, TimeStyle::Short);
    locale.format_date(rullst::orm::Date::from(day), DateStyle::Short);
    locale.format_date(unknown, DateStyle::Short);
    locale.format_date(chrono::Utc::now().date_naive(), DateStyle::Short);
    locale.format_date(&user.profile.birthday, DateStyle::Short);
    locale.format_date(user.birthday(), DateStyle::Short);
}
struct Old { at: chrono::NaiveDate }
fn old(locale: &Locale, old: &Old) { locale.format_date(&old.at, DateStyle::Short); }
"#;
    // The value the file does not show, the chrono values and a method's
    // result; a field of a struct declared elsewhere is the model's.
    assert_eq!(found_with(source, LOCALE, &own_chrono()), [19, 20, 22, 25]);
}

#[test]
fn capital_invoices_built_with_a_jiff_date_need_no_change() {
    let source = r#"
use rullst::capital::Invoice;
use rullst::orm::Timestamp;
struct Payment { paid_at: Timestamp }
fn settled(payment: &Payment) -> rullst::orm::Timestamp { payment.paid_at }
fn bill(payment: &Payment, at: jiff::Timestamp, zoned: &jiff::Zoned) {
    let a = Invoice::new("INV-1", "a@example.com", jiff::Timestamp::now(), "BRL");
    let b = Invoice::new("INV-2", "a@example.com", Timestamp::now(), "BRL");
    let c = Invoice::new("INV-3", "a@example.com", payment.paid_at, "BRL");
    let d = Invoice::new("INV-4", "a@example.com", settled(payment), "BRL");
    let e = Invoice::new("INV-5", "a@example.com", at, "BRL");
    let f = Invoice::new("INV-6", "a@example.com", zoned, "BRL");
    let g = Invoice::new("INV-7", "a@example.com", rullst::orm::Timestamp::now(), "BRL");
    let h = Invoice::new("INV-8", "a@example.com", chrono::Utc::now(), "BRL");
    let i = Invoice::new("INV-9", "a@example.com", other.date, "BRL");
    let j = Invoice::new("INV-10", "a@example.com", date, "BRL");
}
"#;
    assert_eq!(found_with(source, CAPITAL, &own_chrono()), [14, 16]);
}

#[test]
fn jiff_bindings_follow_types_paths_fields_and_functions() {
    let bindings = ChronoBindings::of(
        "struct S { at: jiff::Timestamp, old: DateTime<Utc> }\nfn make() -> Date { todo() }\nfn f(day: Date, when: chrono::NaiveDate, zone: Zoned) {\n    let a = Zoned::now();\n    let b: Timestamp = parse();\n    let c = a;\n    let d = Utc::now();\n    let e = s.at;\n}\n"
            .parse()
            .unwrap(),
    );
    let holds = |expression: &str| {
        let tokens: Vec<_> = expression
            .parse::<TokenStream>()
            .unwrap()
            .into_iter()
            .collect();
        bindings.holds_jiff(&tokens)
    };
    for jiff in [
        "day",
        "zone",
        "&zone",
        "a",
        "b",
        "c",
        "e",
        "make()",
        "s.at",
        "(day)",
        "x.in_tz(\"UTC\")",
        "x.to_zoned(tz)",
        "civil::date(1, 1, 1)",
        "rullst_orm::Date::MIN",
        "rullst::db::Time::MIDNIGHT",
    ] {
        assert!(holds(jiff), "{jiff}");
    }
    for other in [
        "when",
        "d",
        "s.old",
        "Utc::now()",
        "unknown",
        "s.at.other",
        "\"2026\"",
        "Date::new(1, 1, 1)",
        "other::DateTime::now()",
        "make",
        "day.await.field",
    ] {
        assert!(!holds(other), "{other}");
    }
    let args: Vec<_> = "id, email, a, currency"
        .parse::<TokenStream>()
        .unwrap()
        .into_iter()
        .collect();
    assert!(bindings.jiff_argument(&args, 2));
    assert!(!bindings.jiff_argument(&args, 0));
    assert!(!bindings.jiff_argument(&args, 4));
    // Field reads count unless the file declares the field as chrono.
    let args: Vec<_> = "p.paid_at, &p.a.b, *p.c, s.old, p.f(), p, p.0, .x, p."
        .parse::<TokenStream>()
        .unwrap()
        .into_iter()
        .collect();
    let taken: Vec<bool> = (0..9)
        .map(|index| bindings.jiff_argument(&args, index))
        .collect();
    assert_eq!(
        taken,
        [true, true, true, false, false, false, false, false, false]
    );
}

#[test]
fn mail_scheduling_and_account_notice_expiries_must_change() {
    let source = r#"
use rullst::mail::{AccountEvent, Message};
fn schedule(at: DateTime<Utc>, link: ActionLink) {
    let message = Message::new().to("a@example.com").send_at(at);
    if let Some(when) = message.send_at {}
    sent.with_scheduled_at(at);
    let event = AccountEvent::PasswordResetAt { link, expires_at: at };
    let other = matches!(event, AccountEvent::EmailLoginAt { .. });
    match event { AccountEvent::EmailLoginAt { expires_at, .. } => {} _ => {} }
    let reset = AccountEvent::PasswordReset { link, expires_in_minutes: 30 };
}
"#;
    assert_eq!(found_with(source, MAIL, &own_chrono()), [4, 5, 6, 7, 9]);

    // A file that does not name Rullst's Mail keeps its own `send_at`.
    let own = "fn queue(job: Job) {\n    job.send_at(at);\n    let when = job.send_at;\n}\n";
    assert!(found_with(own, MAIL, &own_chrono()).is_empty());
}

#[test]
fn capital_invoices_built_with_a_chrono_date_must_change() {
    let source = r#"
use rullst::capital::{Invoice, InvoiceItem};
fn bill() {
    let invoice = Invoice::new("INV-1", "a@example.com", chrono::Utc::now(), "BRL",);
    let item = InvoiceItem::new("Plan", 4_990);
    let other = Invoice::new(id);
}
"#;
    assert_eq!(found_with(source, CAPITAL, &own_chrono()), [4]);

    let own = "fn bill() {\n    let invoice = Invoice::new(id, email, date, currency);\n}\n";
    assert!(found_with(own, CAPITAL, &own_chrono()).is_empty());
}

#[test]
fn manifests_decide_whether_bare_chrono_and_sqlx_chrono_are_the_applications() {
    const PACKAGE: &str = "[package]\nname = \"app\"\n\n";
    let none = facts(&format!(
        "{PACKAGE}[dependencies]\nsqlx = {{ version = \"0.9\", features = [\"postgres\"] }}\n"
    ));
    assert!(!none.chrono_dependency);
    assert!(!none.sqlx_chrono);

    let declared = facts(&format!(
        "{PACKAGE}[dev-dependencies]\ntime = {{ package = \"chrono\", version = \"0.4\" }}\n"
    ));
    assert!(declared.chrono_dependency);
    assert!(!declared.sqlx_chrono);

    // Two declarations that both enable it still enable it.
    let twice = facts(&format!(
        "{PACKAGE}[dependencies]\nsqlx = {{ version = \"0.9\", features = [\"chrono\"] }}\n\n[workspace.dependencies]\nsqlx = {{ version = \"0.9\", features = [\"chrono\", \"macros\"] }}\n"
    ));
    assert!(twice.sqlx_chrono);
    assert!(!twice.chrono_dependency);

    let referenced = facts(&format!(
        "{PACKAGE}[dependencies]\nsqlx = {{ version = \"0.9\", optional = true }}\n\n[features]\ndates = [\"sqlx?/chrono\"]\nother = [\"sqlx/json\"]\n"
    ));
    assert!(referenced.sqlx_chrono);
    // A declaration and a `[features]` entry that both enable it.
    let both = facts(&format!(
        "{PACKAGE}[dependencies]\nsqlx = {{ version = \"0.9\", optional = true, features = [\"chrono\"] }}\n\n[features]\ndates = [\"sqlx?/chrono\"]\n"
    ));
    assert!(both.sqlx_chrono);
    let other = facts(&format!(
        "{PACKAGE}[dependencies]\nsqlx = {{ version = \"0.9\", optional = true }}\nserde = \"1\"\n\n[features]\ndates = [\"serde/chrono\", \"sqlx/json\"]\n"
    ));
    assert!(!other.sqlx_chrono);
}

/// Patterns met in applications while dogfooding the rule: values built
/// from chrono but turned into text, fields of structs that hold one, and
/// helpers of the file that return one.
#[test]
fn only_expressions_that_still_hold_chrono_are_query_values() {
    let source = r#"
fn cutoff(now: DateTime<Utc>) -> DateTime<Utc> { now - Duration::days(30) }
struct Roll { day: NaiveDate, session_id: i32 }
async fn queries(today: NaiveDate, text: &str, charge: &Charge, roll: Roll) -> Result<(), Error> {
    let start = today - Duration::days(6);
    let since = format!("{} 00:00:00", today - Duration::days(7));
    let label = (today + Duration::days(1)).to_string();
    let window = (Utc::now() - Duration::hours(2)).format("%F %T").to_string();
    let issued = Issued { code: code(), valid_until: Utc::now() };
    let limit = cutoff(Utc::now());
    let parsed: NaiveDate = "2026-10-12".parse()?;
    let opened = text.parse::<chrono::NaiveDate>().ok();
    let local = (Utc::now() + Duration::hours(OFFSET)).date_naive();
    let fresh = Charge::find(&charge.reference).await?;
    sqlx::query(SQL).bind(start.to_string()).bind(&issued.code).bind(since).bind(&label).bind(window).execute(pool).await?;
    sqlx::query(SQL).bind(fresh.id).bind(i32::try_from(charge.cents).unwrap_or(0)).bind(start.timestamp()).execute(pool).await?;
    sqlx::query(SQL).bind(limit).execute(pool).await?;
    sqlx::query(SQL).bind(parsed).execute(pool).await?;
    sqlx::query(SQL).bind(opened).execute(pool).await?;
    sqlx::query(SQL).bind(&local).execute(pool).await?;
    sqlx::query(SQL).bind((start, 1)).execute(pool).await?;
    sqlx::query(SQL).bind(start).execute(pool).await?;
    sqlx::query(SQL).bind(roll.day).bind(roll.session_id).execute(pool).await?;
    Ok(())
}
"#;
    assert_eq!(
        found_with(source, ORM, &own_chrono()),
        [17, 18, 19, 20, 21, 22, 23]
    );
}

#[test]
fn sqlx_reads_that_decode_chrono_must_change_without_sqlxs_feature() {
    let source = r#"
async fn read(pool: &Pool, row: &Row) -> Result<(), Error> {
    let days: Vec<NaiveDate> = sqlx::query_scalar("SELECT day FROM visits").fetch_all(pool).await?;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM visits").fetch_one(pool).await?;
    let last = rullst::db::sqlx::query_scalar::<_, DateTime<Utc>>("SELECT MAX(at) FROM visits").fetch_one(pool).await?;
    let pairs = sqlx::query_as::<_, (i32, String)>("SELECT id, name FROM users").fetch_all(pool).await?;
    let at = row.try_get::<DateTime<Utc>, _>("created_at")?;
    let locale = request.extensions().get::<Locale>();
    Ok(())
}
#[derive(Debug, sqlx::FromRow)]
struct Visit {
    id: i32,
    day: NaiveDate,
    note: String,
}
#[derive(sqlx::FromRow)]
struct Consent {
    created_at: String,
}
"#;
    assert_eq!(found(source, ORM), [3, 5, 7, 14]);
    let sqlx_chrono = WorkspaceFacts {
        sqlx_chrono: true,
        ..WorkspaceFacts::default()
    };
    assert!(found_with(source, ORM, &sqlx_chrono).is_empty());
}
