//! `Markup` composition, conversions and responses.

use super::Markup;
use crate::html;
use crate::html::{HtmlEscape, RawHtml};
use axum::response::IntoResponse;

fn status_badge(active: bool) -> Markup {
    let label = if active { "Ativo" } else { "<Inativo>" };
    html! { <span class="badge">{label}</span> }
}

#[test]
fn a_nested_view_is_inserted_as_html_in_a_table_cell() {
    // Dogfooding #14: a badge built by `html!` showed up as `&lt;span …`.
    let badge = status_badge(true);
    let row = html! { <tr><td>"Ana"</td><td>{badge}</td></tr> };
    assert_eq!(
        row,
        "<tr><td>Ana</td><td><span class=\"badge\">Ativo</span></td></tr>"
    );

    let badge = status_badge(false);
    let row = html! { <tr><td>{&badge}</td><td>{status_badge(true)}</td></tr> };
    assert_eq!(
        row,
        "<tr><td><span class=\"badge\">&lt;Inativo&gt;</span></td>\
         <td><span class=\"badge\">Ativo</span></td></tr>"
    );
}

#[test]
fn strings_stay_escaped_next_to_markup() {
    let name = String::from("<b>Bob</b>");
    let markup = html! { <b>"Bob"</b> };
    assert_eq!(
        html! { <p>{name}{markup}{"<i>"}{7}</p> },
        "<p>&lt;b&gt;Bob&lt;/b&gt;<b>Bob</b>&lt;i&gt;7</p>"
    );
    // A Markup taken apart into a String is text again.
    let taken: String = html! { <b>"x"</b> }.into();
    assert_eq!(html! { <p>{taken}</p> }, "<p>&lt;b&gt;x&lt;/b&gt;</p>");
}

#[test]
fn markup_in_an_attribute_value_is_escaped_like_text() {
    // Unescaped, the quotes of the inner markup would end `title` and turn
    // `onmouseover=alert(1)//` into an event handler.
    let tip = html! { <b title={"onmouseover=alert(1)//"}>"x"</b> };
    let expected =
        "<a title=\"&lt;b title=&quot;onmouseover=alert(1)//&quot;&gt;x&lt;/b&gt;\">y</a>";
    assert_eq!(html! { <a title={&tip}>"y"</a> }, expected);
    assert_eq!(html! { <a title={Some(&tip)}>"y"</a> }, expected);
    assert_eq!(html! { <a title={tip}>"y"</a> }, expected);
    assert_eq!(
        html! { <a title={None::<Markup>}>"y"</a> },
        "<a title=\"\">y</a>"
    );
    // Text is escaped the same way in both contexts.
    assert_eq!(Some("<\"").escape_attribute(), "&lt;&quot;");
    assert_eq!(None::<&str>.escape_attribute(), "");
}

#[test]
fn raw_html_and_from_trusted_insert_trusted_html() {
    let trusted = String::from("<em>reviewed</em>");
    assert_eq!(
        html! { <p>{RawHtml(trusted.clone())}</p> },
        "<p><em>reviewed</em></p>"
    );
    let markup = Markup::from(RawHtml::new("<em>raw</em>"));
    assert_eq!(markup, "<em>raw</em>");
    let markup = Markup::from_trusted(trusted);
    assert_eq!(html! { <p>{markup}</p> }, "<p><em>reviewed</em></p>");
}

#[test]
fn options_render_nothing_for_none() {
    let missing: Option<Markup> = None;
    let present = Some(html! { <b>"!"</b> });
    let text = Some("<x>");
    let no_text: Option<&str> = None;
    assert_eq!(
        html! { <p>{missing}{present}{text}{no_text}</p> },
        "<p><b>!</b>&lt;x&gt;</p>"
    );
    assert_eq!(Some(7).escape_html(), "7");
    assert_eq!(None::<i32>.escape_html(), "");
}

#[test]
fn markup_collects_and_extends_in_order() {
    let rows: Markup = ["a", "<b>"]
        .iter()
        .map(|item| html! { <li>{item}</li> })
        .collect();
    assert_eq!(rows, "<li>a</li><li>&lt;b&gt;</li>");

    let mut list = Markup::from_trusted("<li>0</li>");
    list.extend([html! { <li>"1"</li> }, html! { <li>"2"</li> }]);
    assert_eq!(list, "<li>0</li><li>1</li><li>2</li>");
    assert_eq!(std::iter::empty::<Markup>().collect::<Markup>(), "");
}

#[test]
fn markup_reads_like_a_string() {
    let markup = html! { <p>"a&b"</p> };
    assert_eq!(markup.as_str(), "<p>a&b</p>");
    assert_eq!(markup.to_string(), "<p>a&b</p>");
    assert_eq!(format!("[{markup}]"), "[<p>a&b</p>]");
    assert_eq!(AsRef::<str>::as_ref(&markup), "<p>a&b</p>");
    assert_eq!(&*markup, "<p>a&b</p>");
    assert_eq!(markup.len(), 10);
    assert!(markup.contains("a&b"));
    assert_eq!(markup.clone().into_string(), "<p>a&b</p>");
    assert_eq!(String::from(markup), "<p>a&b</p>");
    assert_eq!(Markup::default().into_string(), "");
}

#[test]
fn markup_compares_with_strings_both_ways() {
    let markup = Markup::from_trusted("<p></p>");
    let same = String::from("<p></p>");
    let other = String::from("<div></div>");

    assert!(markup == *"<p></p>");
    assert!(markup == "<p></p>");
    assert!(markup == same);
    assert!(*"<p></p>" == markup);
    assert!("<p></p>" == markup);
    assert!(same == markup);

    assert!(markup != *"<div></div>");
    assert!(markup != "<div></div>");
    assert!(markup != other);
    assert!(*"<div></div>" != markup);
    assert!("<div></div>" != markup);
    assert!(other != markup);
}

async fn response_parts(response: axum::response::Response) -> (String, String) {
    let content_type = response
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    (content_type, String::from_utf8(body.to_vec()).unwrap())
}

#[tokio::test]
async fn markup_responds_as_utf8_html() {
    let response = html! { <h1>"Olá"</h1> }.into_response();
    assert_eq!(response.status(), axum::http::StatusCode::OK);
    assert_eq!(
        response_parts(response).await,
        (
            "text/html; charset=utf-8".to_owned(),
            "<h1>Olá</h1>".to_owned()
        )
    );

    // `Html(html! { .. })` keeps working through `From<Markup> for Body`.
    let response = axum::response::Html(html! { <p>"x"</p> }).into_response();
    assert_eq!(
        response_parts(response).await,
        ("text/html; charset=utf-8".to_owned(), "<p>x</p>".to_owned())
    );
}
