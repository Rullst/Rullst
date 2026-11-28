//! Every interpolated value type in text position, and composed fragments.

use std::borrow::Cow;

use rullst::html;
use rullst::html::{Markup, RawHtml};

use super::Case;

pub fn cases() -> Vec<Case> {
    let mut cases = strings();
    cases.extend(numbers());
    cases.extend(wrappers());
    cases.extend(fragments());
    cases
}

fn strings() -> Vec<Case> {
    let plain = "plain text";
    let special = "<script>alert(\"x\" & 'y')</script>";
    let owned = String::from("a < b && c > d");
    let borrowed: Cow<'_, str> = Cow::Borrowed("<borrowed>");
    let cow: Cow<'_, str> = Cow::Owned("\"owned\"".to_string());
    vec![
        (
            "values/strings",
            html! {
                <p>{plain}</p><p>{special}</p><p>{owned}</p><p>{&owned}</p>
                <p>{borrowed}</p><p>{cow}</p><p>{&&plain}</p><p>{""}</p>
            },
        ),
        (
            "values/only-specials",
            html! { <p>{"<<>>&&\"\"''"}</p><p>{"&"}</p><p>{"'"}</p> },
        ),
        (
            "values/multibyte-around-specials",
            html! { <p>{"é<ü>ñ&ç\"ã'õ"}</p><p>{"フ<レ>"}</p><p>{"🏠&🛠️"}</p> },
        ),
        (
            "values/specials-at-the-edges",
            html! { <p>{"<a"}</p><p>{"a>"}</p><p>{"<"}</p><p>{"a&b&c"}</p> },
        ),
        (
            "values/fortune-messages",
            html! {
                <p>{"A computer scientist is someone who fixes things that aren't broken."}</p>
                <p>{"<script>alert(\"This should not be displayed in a browser alert box.\");</script>"}</p>
                <p>{"Emacs is a nice operating system, but I prefer UNIX. \u{2014} Tom Christaensen"}</p>
            },
        ),
    ]
}

#[allow(clippy::approx_constant)]
fn numbers() -> Vec<Case> {
    vec![
        (
            "values/integers",
            html! {
                <p>{i8::MIN} {i16::MIN} {-42_i32} {i64::MIN} {i128::MAX} {isize::MAX}</p>
                <p>{u8::MAX} {u16::MAX} {7_u32} {u64::MAX} {u128::MAX} {0_usize} {0}</p>
            },
        ),
        (
            "values/floats",
            html! {
                <p>{1.5_f32} {0.1 + 0.2} {-0.0_f64} {f64::NAN} {f64::INFINITY} {f64::NEG_INFINITY}</p>
                <p>{1e21} {1e-7} {f32::MAX} {f64::MIN_POSITIVE} {3.0_f64} {2.5e10_f32}</p>
            },
        ),
        ("values/bools", html! { <p>{true} {false}</p> }),
        (
            "values/expressions",
            html! {
                <p>{1 + 2}</p><p>{"ana".to_uppercase()}</p><p>{format!("<{}>", 5)}</p>
                <p>{{ let x = 2; x * 3 }}</p><p>{match 3 { 3 => "three", _ => "other" }}</p>
                <p>{(|n: i32| n * 2)(21)}</p>
            },
        ),
    ]
}

fn wrappers() -> Vec<Case> {
    let markup = html! { <i>"<already> escaped"</i> };
    let some_markup: Option<Markup> = Some(html! { <b>"b"</b> });
    let no_markup: Option<Markup> = None;
    vec![
        (
            "values/options",
            html! {
                <p>{Some("<a>")}</p><p>{None::<&str>}</p><p>{Some(5)}</p><p>{None::<i32>}</p>
                <p>{Some(Some("x"))}</p><p>{Some(None::<&str>)}</p><p>{&Some(String::from("&"))}</p>
                <p>{some_markup}</p><p>{no_markup}</p>
            },
        ),
        (
            "values/markup-and-raw",
            html! {
                <p>{markup}</p><p>{&markup}</p>
                <p>{RawHtml("<em>raw & ready</em>".to_string())}</p>
                <p>{Some(RawHtml::new("<u>u</u>"))}</p>
                <p>{Markup::from_trusted("<s>trusted</s>")}</p>
            },
        ),
    ]
}

fn badge(label: &str) -> Markup {
    html! { <span class="badge">{label}</span> }
}

fn fragments() -> Vec<Case> {
    let names = ["Ana", "<Bob>", "Cé"];
    let rows: Markup = names
        .iter()
        .map(|name| html! { <li>{name}{badge("new")}</li> })
        .collect();
    let admin = true;
    vec![
        ("fragments/collected", html! { <ul>{rows}</ul> }),
        (
            "fragments/nested-calls",
            html! {
                <div class="dashboard">
                    <header>
                        <h2>"Welcome, "{names[0]}</h2>
                        {admin.then(|| html! { <span class="badge">"Admin"</span> })}
                        {(!admin).then(|| html! { <span>"never"</span> })}
                    </header>
                    <ul>{names.iter().map(|name| html! { <li title={name}>{name}</li> }).collect::<Markup>()}</ul>
                    {badge("<x>")}
                </div>
            },
        ),
    ]
}
