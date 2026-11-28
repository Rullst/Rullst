//! `@if`, `@for` and `@match`, including `continue` and `break` written
//! inside an `@for` body: the part of an iteration that was cut short is not
//! written.

use rullst::html;

use super::Case;

#[derive(Clone, Copy)]
enum State {
    Empty,
    One(&'static str),
    Two(&'static str, u8),
    Many { count: usize },
}

pub fn cases() -> Vec<Case> {
    let mut cases = branches();
    cases.extend(loops());
    cases.extend(matches());
    cases.extend(loop_exits());
    cases
}

fn branches() -> Vec<Case> {
    let user: Option<&str> = Some("<ana>");
    let nobody: Option<&str> = None;
    let page = |level: u8| {
        html! {
            @if level == 0 { <p>"zero"</p> } @else if level == 1 { "one" } @else if level < 5 { <b>{level}</b> } @else { "many" }
        }
    };
    vec![
        ("if/true", html! { @if true { <p>"yes"</p> } }),
        ("if/false", html! { <div>@if false { <p>"yes"</p> }</div> }),
        (
            "if/let",
            html! {
                @if let Some(name) = user { <b>{name}</b> } @else { "guest" }
                @if let Some(name) = nobody { <b>{name}</b> } @else { "guest" }
            },
        ),
        ("if/chain-0", page(0)),
        ("if/chain-1", page(1)),
        ("if/chain-3", page(3)),
        ("if/chain-9", page(9)),
        (
            "if/beside-text",
            html! { <p>a @if true { "b" } c @if false { "d" } e</p> },
        ),
    ]
}

fn loops() -> Vec<Case> {
    let items = ["a", "<b>", "c"];
    let empty: [&str; 0] = [];
    let pairs = vec![("x", 1), ("y", 2)];
    let owned = vec![String::from("o1"), String::from("o&2")];
    let grid = [[1, 2], [3, 4]];
    vec![
        (
            "for/slice",
            html! { <ul>@for item in items { <li>{item}</li> }</ul> },
        ),
        (
            "for/empty",
            html! { <ul>@for item in empty { <li>{item}</li> }</ul> },
        ),
        (
            "for/range-and-text",
            html! { <p>@for n in 0..4 { {n} "," }</p> },
        ),
        (
            "for/enumerate",
            html! {
                <ol>@for (index, item) in items.iter().enumerate() { <li data-i={index}>{index}": "{item}</li> }</ol>
            },
        ),
        (
            "for/patterns",
            html! { <dl>@for &(key, value) in &pairs { <dt>{key}</dt><dd>{value}</dd> }</dl> },
        ),
        (
            "for/owned",
            html! { <p>@for text in owned { <span>{text}</span> }</p> },
        ),
        (
            "for/nested",
            html! {
                <table>@for row in grid { <tr>@for cell in row { <td>{cell}</td> }</tr> }</table>
            },
        ),
        (
            "for/nested-control",
            html! {
                <ul>@for (index, item) in items.iter().enumerate() {
                    @if index % 2 == 0 { <li class="even">{item}</li> } @else { <li>{item}</li> }
                }</ul>
            },
        ),
        (
            "for/nested-html-call",
            html! { <div>@for item in items { {html! { <i>{item}</i> }} " " }</div> },
        ),
    ]
}

fn matches() -> Vec<Case> {
    let page = |state: State| {
        html! {
            <div>
                @match state {
                    State::Empty => { <p>"Nothing"</p> }
                    State::One(name) if name.is_empty() => { "anonymous" },
                    State::One(name) | State::Two(name, 0) => { <b>{name}</b> "!" }
                    State::Two(name, n) => { {name} " x" {n} }
                    many @ State::Many { .. } => {
                        @if let State::Many { count } = many { <i>{count}</i> }
                    }
                }
            </div>
        }
    };
    vec![
        ("match/empty", page(State::Empty)),
        ("match/guard", page(State::One(""))),
        ("match/or-pattern", page(State::Two("<two>", 0))),
        ("match/binding", page(State::Two("t", 3))),
        ("match/at-pattern", page(State::Many { count: 12 })),
        (
            "match/empty-arm",
            html! { <p>@match 1 { 0 => { "zero" } _ => {} }</p> },
        ),
    ]
}

/// `continue` and `break` reach the `@for` loop of the expansion: the output
/// of the node they cut short is dropped, earlier nodes of the iteration and
/// earlier iterations are kept.
fn loop_exits() -> Vec<Case> {
    let values = [Some(1), None, Some(3), Some(4)];
    let words = ["keep", "skip", "stop", "after"];
    vec![
        (
            "exit/continue-in-text",
            html! {
                <ul>@for value in values {
                    <b>"before"</b>
                    <li>"n=" {{ let Some(n) = value else { continue }; n }}</li>
                    <i>"after"</i>
                }</ul>
            },
        ),
        (
            "exit/break-in-text",
            html! {
                <ul>@for word in words {
                    <li>{{ if word == "stop" { break } word }}</li>
                    <hr />
                }</ul>
            },
        ),
        (
            "exit/continue-in-attribute",
            html! {
                <p>@for word in words {
                    <a href="/w" title={{ if word == "skip" { continue } word }}>{word}</a>
                }</p>
            },
        ),
        (
            "exit/continue-in-conditions",
            html! {
                <p>@for word in words {
                    "[" @if { if word == "skip" { continue } word.len() > 4 } { <b>{word}</b> } @else { {word} }
                    @match word { w if { if w == "stop" { break } true } => { "+" } _ => {} }
                    "]"
                }</p>
            },
        ),
        (
            "exit/nested-loops",
            html! {
                <div>@for row in [[1, 2, 3], [4, 5, 6]] {
                    <p>@for cell in row { <span>{{ if cell == 2 { continue } else if cell == 5 { break } cell }}</span> }</p>
                    <hr />
                }</div>
            },
        ),
        (
            "exit/inner-iterable",
            html! {
                <div>@for word in words {
                    <p>@for letter in { if word == "skip" { continue } word.chars().take(2) } { {letter.to_string()} }</p>
                }</div>
            },
        ),
        (
            "exit/own-loop-inside-a-block",
            html! {
                <p>@for word in words {
                    {{
                        let mut n = 0;
                        loop { n += 1; if n == 2 { break } }
                        for _ in 0..3 { if n > 0 { continue } }
                        n
                    }}
                    {word}
                }</p>
            },
        ),
        (
            "exit/continue-deep-in-an-element",
            html! {
                <ul>@for word in words {
                    <li><span>{word}</span><em>@if word == "skip" { <u>{{ if true { continue } 0 }}</u> }</em></li>
                    ";"
                }</ul>
            },
        ),
    ]
}
