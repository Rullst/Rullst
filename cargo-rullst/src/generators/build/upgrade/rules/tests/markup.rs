//! `html!` returns `Markup`: `-> String` views and `RawHtml` around `html!`.

use super::*;

fn markup_findings(source: &str) -> Vec<(&'static str, usize)> {
    scan_with(source, "views.rs", &WorkspaceFacts::default())
        .into_iter()
        .filter(|(code, _)| *code == "V13-HTML-MARKUP-RETURN" || *code == "V13-HTML-RAW-MARKUP")
        .collect()
}

#[test]
fn functions_returning_a_string_built_by_html_must_change() {
    let source = r#"
fn badge(label: &str) -> String {
    html! { <span class="badge">{label}</span> }
}
async fn page() -> Html<String> {
    Html(html! { <p>"x"</p> })
}
fn fallible(ok: bool) -> Result<String, Error> {
    if ok { Ok(html! { <p>"ok"</p> }) } else { Err(Error) }
}
fn early(empty: bool)
    -> String
{
    if empty {
        return rullst::html!(<p>"none"</p>);
    }
    format!("<p>{}</p>", 1)
}
impl Card {
    fn render(&self) -> String {
        match self.kind {
            Kind::A => html! { <b>"a"</b> },
            Kind::B => String::new(),
        }
    }
}
trait View {
    fn view(&self) -> String { { html! { <i>"v"</i> } } }
}
fn parenthesized() -> String { (html! { <p>"p"</p> }) }
fn two_returns(first: bool) -> String {
    if first {
        return html! { <a>"1"</a> };
    }
    return html! { <a>"2"</a> };
}
"#;
    assert_eq!(
        markup_findings(source),
        vec![
            ("V13-HTML-MARKUP-RETURN", 2),
            ("V13-HTML-MARKUP-RETURN", 5),
            ("V13-HTML-MARKUP-RETURN", 8),
            ("V13-HTML-MARKUP-RETURN", 12),
            ("V13-HTML-MARKUP-RETURN", 20),
            ("V13-HTML-MARKUP-RETURN", 28),
            ("V13-HTML-MARKUP-RETURN", 30),
            ("V13-HTML-MARKUP-RETURN", 31),
        ]
    );
}

#[test]
fn markup_views_and_strings_built_otherwise_are_not_reported() {
    let source = r#"
fn badge(label: &str) -> Markup {
    html! { <span>{label}</span> }
}
fn checked(ok: bool) -> Result<Markup, String> {
    Ok(html! { <p>"ok"</p> })
}
fn text() -> String {
    let markup = html! { <p>"x"</p> };
    let _closure = || -> String { html! { <p>"in closure"</p> } };
    format!("{markup}")
}
fn statement() -> String {
    html! { <p>"discarded"</p> };
    String::new()
}
fn trailing_statement() -> String {
    todo();
    html! { <p>"discarded"</p> };
}
fn ok_text() -> Result<String, Error> { Ok(String::new()) }
fn converted() -> String { into_text(html! { <p>"x"</p> }) }
fn unit() { html! { <p>"x"</p> }; }
fn other() -> String { other! { <p>"x"</p> } }
fn trusted(markdown: &str) -> Markup {
    let rendered = render_markdown(markdown);
    html! { <article>{RawHtml(rendered)}{RawHtml::new(String::from("<hr>"))}</article> }
}
"#;
    assert_eq!(markup_findings(source), vec![]);
}

#[test]
fn raw_html_around_html_output_must_change_everywhere_it_is_written() {
    let source = r#"
fn badge(label: &str) -> String {
    html! { <span>{label}</span> }
}
fn table(rows: &[Row]) -> Markup {
    let cells = rows.iter().map(|row| html! { <td>{row.name}</td> }).collect::<String>();
    let direct = RawHtml(html! { <b>"b"</b> });
    let trusted = RawHtml::new(load_trusted());
    let typed: String = rows.iter().map(|row| html! { <th>{row.name}</th> }).collect();
    html! {
        <table>
            <tr>{RawHtml(cells)}</tr>
            <tr>{rullst::html::RawHtml::new(badge("new"))}</tr>
            <tr>{RawHtml(&cells)}{trusted}</tr>
            <tr title={RawHtml(badge("x"))}>{ html! { <td>{RawHtml(cells.clone())}</td> } }</tr>
            <tr>{RawHtml((typed))}</tr>
            <tr>{RawHtml([html! { <td>"1"</td> }, html! { <td>"2"</td> }].concat())}</tr>
        </table>
    }
}
fn other_scope(cells: String) -> Markup {
    html! { <tr>{RawHtml(cells)}</tr> }
}
"#;
    assert_eq!(
        markup_findings(source),
        vec![
            ("V13-HTML-MARKUP-RETURN", 2),
            ("V13-HTML-RAW-MARKUP", 7),
            ("V13-HTML-RAW-MARKUP", 12),
            ("V13-HTML-RAW-MARKUP", 13),
            ("V13-HTML-RAW-MARKUP", 14),
            ("V13-HTML-RAW-MARKUP", 15),
            ("V13-HTML-RAW-MARKUP", 16),
            ("V13-HTML-RAW-MARKUP", 17),
        ]
    );
}

#[test]
fn raw_html_around_islands_and_mounted_live_components_must_change() {
    let source = r#"
#[island]
fn counter(start: i32) -> String {
    format!("<b>{start}</b>")
}
async fn page() -> Markup {
    let mounted = rullst::live::Live::mount::<Counter>("/ws").await;
    html! {
        <main>
            {RawHtml(counter(1))}
            {RawHtml(mounted)}
            {RawHtml(Live::mount::<Counter>("/ws").await)}
            {RawHtml(mount_point())}
            {RawHtml(Cache::mount(1))}
            {RawHtml(Live::render(1))}
        </main>
    }
}
"#;
    assert_eq!(
        markup_findings(source),
        vec![
            ("V13-HTML-RAW-MARKUP", 10),
            ("V13-HTML-RAW-MARKUP", 11),
            ("V13-HTML-RAW-MARKUP", 12),
        ]
    );
}

#[test]
fn markup_rules_name_the_html_markup_migration_row() {
    for code in ["V13-HTML-MARKUP-RETURN", "V13-HTML-RAW-MARKUP"] {
        let rule = CATALOG.iter().find(|rule| rule.code == code).unwrap();
        assert_eq!(rule.row, "`html!` returns `Markup`");
        assert_eq!(rule.kind, FindingKind::MustChange);
    }
}

/// The framework's own generated views already return `Markup`, so a fresh
/// v13 project gets no `html!` migration finding.
#[test]
fn generated_starters_and_auth_views_have_no_markup_findings() {
    use crate::blueprints::{blank, saas};

    let (orm, frontend) = ("Active Record", "Zero-Bundle HTMX");
    let mut sources = vec![(
        "auth/views".to_string(),
        crate::generators::auth::views::AUTH_VIEWS_TEMPLATE.to_string(),
    )];
    for hot in [false, true] {
        for (name, manifest) in [
            (
                "blank",
                blank::file_manifest("demo", "demo", false, hot, true, orm, frontend),
            ),
            ("saas", saas::file_manifest("demo", hot, orm, frontend)),
        ] {
            sources.extend(
                manifest
                    .into_iter()
                    .filter(|(path, _)| path.ends_with(".rs"))
                    .map(|(path, source)| (format!("{name}/{path}"), source)),
            );
        }
    }
    let mut scanned_views = 0;
    for (path, source) in &sources {
        scanned_views += usize::from(source.contains("html!"));
        assert_eq!(markup_findings(source), vec![], "{path}");
    }
    assert!(scanned_views >= 7, "only {scanned_views} sources use html!");
}
