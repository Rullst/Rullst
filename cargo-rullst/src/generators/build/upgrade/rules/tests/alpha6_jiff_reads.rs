//! A field read keeps the type of a struct declared in another file through
//! `?` and the calls that return the value or the fallback they are given,
//! so `V13-CAPITAL-INVOICE-JIFF-DATE` and `V13-LOCALE-JIFF-DATES` leave such
//! a value alone, as they do a bare field read (#894).

use super::super::alpha6_jiff::ChronoBindings;
use super::*;
use proc_macro2::TokenStream;

const LOCALE: &str = "V13-LOCALE-JIFF-DATES";
const CAPITAL: &str = "V13-CAPITAL-INVOICE-JIFF-DATE";

fn found(source: &str, code: &str) -> Vec<usize> {
    scan_with(source, "lib.rs", &WorkspaceFacts::default())
        .into_iter()
        .filter(|(found, _)| *found == code)
        .map(|(_, line)| line)
        .collect()
}

#[test]
fn an_invoice_dated_by_a_field_read_and_unwrap_or_else_needs_no_change() {
    // The reproduction of #894: `Charge` is declared in `src/models/m.rs`.
    let source = r#"
use rullst::orm::Timestamp;
use rullst::capital::Invoice;
fn f(c: &crate::models::m::Charge) {
    let _ = Invoice::new("r", "e@x", c.paid_at.unwrap_or_else(Timestamp::now), "BRL");
}
"#;
    assert!(found(source, CAPITAL).is_empty());
}

#[test]
fn field_reads_through_kept_calls_need_no_change_unless_they_hold_chrono() {
    let source = r#"
use rullst::capital::Invoice;
use rullst::orm::Timestamp;
struct Old { at: Option<chrono::DateTime<chrono::Utc>> }
fn now_utc() -> DateTime<Utc> { Utc::now() }
fn bill(c: &Charge, old: &Old, since: Option<DateTime<Utc>>) -> Result<(), Error> {
    let fallback = chrono::Utc::now();
    let a = Invoice::new("1", "a@x", c.paid_at.unwrap_or(Timestamp::UNIX_EPOCH), "BRL");
    let b = Invoice::new("2", "a@x", c.paid_at?, "BRL");
    let d = Invoice::new("3", "a@x", c.paid_at.clone(), "BRL");
    let e = Invoice::new("4", "a@x", &c.charge.paid_at.clone().unwrap_or_default(), "BRL");
    let g = Invoice::new("5", "a@x", c.paid_at.to_owned(), "BRL");
    let o = Invoice::new("6", "a@x", c.paid_at.unwrap_or_else(|| Timestamp::now()), "BRL");
    let h = Invoice::new("7", "a@x", c.paid_at.unwrap_or_else(chrono::Utc::now), "BRL");
    let i = Invoice::new("8", "a@x", c.paid_at.unwrap_or(fallback), "BRL");
    let j = Invoice::new("9", "a@x", c.paid_at.unwrap_or_else(|| Utc::now()), "BRL");
    let q = Invoice::new("10", "a@x", c.paid_at.unwrap_or_else(|| fallback), "BRL");
    let r = Invoice::new("11", "a@x", c.paid_at.unwrap_or_else(now_utc), "BRL");
    let k = Invoice::new("12", "a@x", old.at.clone().unwrap_or_default(), "BRL");
    let l = Invoice::new("13", "a@x", since.unwrap_or_else(Utc::now), "BRL");
    let m = Invoice::new("14", "a@x", chrono::Utc::now().clone(), "BRL");
    let n = Invoice::new("15", "a@x", c.paid_at.other(), "BRL");
    Ok(())
}
"#;
    // A chrono fallback (a path, a binding, a closure that returns one, a
    // function of the file), a chrono field of the file, a chrono binding
    // or path, and a call that may return another type are still reported.
    assert_eq!(found(source, CAPITAL), [14, 15, 16, 17, 18, 19, 20, 21, 22]);
}

#[test]
fn locale_formatting_of_a_field_read_through_kept_calls_needs_no_change() {
    let source = r#"
use rullst::i18n::{DateStyle, Locale};
fn show(locale: &Locale, user: &User) {
    locale.format_date(user.birthday.unwrap_or_default(), DateStyle::Short);
    locale.format_date(&user.profile.birthday.clone()?, DateStyle::Short);
    locale.format_date(user.birthday.unwrap_or(chrono::NaiveDate::MIN), DateStyle::Short);
    locale.format_date(user.birthday.unwrap_or_else(|| Local::now().date_naive()), DateStyle::Short);
}
"#;
    assert_eq!(found(source, LOCALE), [6, 7]);
}

#[test]
fn kept_calls_keep_a_field_read_unless_their_arguments_hold_chrono() {
    let bindings = ChronoBindings::of(
        "struct S { old: DateTime<Utc> }\nfn now_utc() -> DateTime<Utc> { Utc::now() }\nfn f(when: chrono::NaiveDate) {\n    let d = Utc::now();\n}\n"
            .parse()
            .unwrap(),
    );
    let taken = |expression: &str| {
        let args: Vec<_> = expression
            .parse::<TokenStream>()
            .unwrap()
            .into_iter()
            .collect();
        bindings.jiff_argument(&args, 0)
    };
    // Each kept method, given no argument or one that is not chrono. The
    // reads are built here because `generators::tests` refuses literals
    // that contain a panicking call.
    let calls = [
        ("clone", ""),
        ("to_owned", ""),
        ("cloned", ""),
        ("copied", ""),
        ("unwrap", ""),
        ("expect", "\"paid\""),
        ("unwrap_or", "Timestamp::MIN"),
        ("unwrap_or_else", "Timestamp::now"),
        ("unwrap_or_default", ""),
    ];
    for (method, argument) in calls {
        let read = format!("p.paid_at.{method}({argument})");
        assert!(taken(&read), "{read}");
    }
    for kept in [
        "p.paid_at?",
        "&p.paid_at.to_owned()",
        "*p.paid_at.copied()",
        "p.a.b.unwrap_or(Timestamp::MIN)",
        "p.paid_at.clone()?.clone()?",
        "p.paid_at.unwrap_or_else(|| Timestamp::now())",
        "p.paid_at.unwrap_or_else(move || Timestamp::now())",
        "p.paid_at.unwrap_or_else(|_| Timestamp::UNIX_EPOCH)",
    ] {
        assert!(taken(kept), "{kept}");
    }
    for other in [
        "p.paid_at.unwrap_or_else(Utc::now)",
        "p.paid_at.unwrap_or(d)",
        "p.paid_at.unwrap_or(when)",
        "p.paid_at.unwrap_or_else(|| Utc::now())",
        "p.paid_at.unwrap_or_else(|| chrono::Local::now())",
        "p.paid_at.unwrap_or_else(|| Local::now().date_naive())",
        "p.paid_at.unwrap_or_else(|| d)",
        "p.paid_at.unwrap_or_else(move || d)",
        "p.paid_at.unwrap_or_else(|_| d)",
        "p.paid_at.unwrap_or_else(|| unknown())",
        "p.paid_at.unwrap_or_else(now_utc)",
        "p.paid_at.unwrap_or_else(crate::time::now_utc)",
        "p.paid_at.unwrap_or(convert(Utc::now()))",
        "s.old.clone()",
        "s.old?",
        "when.clone()",
        "p.clone()",
        "p?",
        "p.paid_at.f()",
        "p.paid_at.f()?",
        "p.paid_at.clone().f()",
        "p.paid_at.",
    ] {
        assert!(!taken(other), "{other}");
    }
}
