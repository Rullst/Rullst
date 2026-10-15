//! Runtime output of `html!` bare and conditional attributes and of
//! `@if`/`@for` blocks, with the real escaping functions.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::borrow::Cow;

use super::{AttributePresence, Markup, OptionalAttribute, RawHtml, optional_attr};

#[test]
fn optional_attr_maps_bools_and_options_to_presence() {
    assert_eq!(optional_attr(&true), AttributePresence::Bare);
    assert_eq!(optional_attr(&false), AttributePresence::Absent);
    assert_eq!(optional_attr(&None::<&str>), AttributePresence::Absent);
    assert_eq!(
        optional_attr(&Some("a<b>\"c\"&'d'")),
        AttributePresence::Value(Cow::Borrowed("a&lt;b&gt;&quot;c&quot;&amp;&#x27;d&#x27;"))
    );
    assert_eq!(
        optional_attr(&Some(42_u16)),
        AttributePresence::Value(Cow::Borrowed("42"))
    );
    // References delegate to the referenced value.
    let flag = true;
    let reference: &&bool = &&flag;
    assert_eq!(optional_attr(reference), AttributePresence::Bare);
    assert_eq!(
        (&Some(String::from("x"))).attribute_presence(),
        AttributePresence::Value(Cow::Borrowed("x"))
    );
}

#[test]
fn valueless_attributes_are_written_bare_and_quoted_forms_still_work() {
    assert_eq!(
        crate::html! { <input name="title" required disabled /> },
        "<input name=\"title\" required disabled />"
    );
    assert_eq!(
        crate::html! { <select><option value="a" selected>"A"</option></select> },
        "<select><option value=\"a\" selected>A</option></select>"
    );
    assert_eq!(
        crate::html! { <input required="true" disabled="disabled" /> },
        "<input required=\"true\" disabled=\"disabled\" />"
    );
    assert_eq!(
        crate::html! { <form hx-boost novalidate></form> },
        "<form hx-boost novalidate></form>"
    );
}

#[test]
fn conditional_bool_attributes_are_present_or_absent() {
    let render = |done: bool, locked: bool| {
        crate::html! { <input type="checkbox" checked?={done} disabled?={locked} /> }
    };
    assert_eq!(render(true, false), "<input type=\"checkbox\" checked />");
    assert_eq!(render(false, true), "<input type=\"checkbox\" disabled />");
    assert_eq!(render(false, false), "<input type=\"checkbox\" />");
    // `name={bool}` still writes the text value, as `aria-*` expects.
    let open = false;
    assert_eq!(
        crate::html! { <button aria-expanded={open} hidden?={!open}>"Menu"</button> },
        "<button aria-expanded=\"false\" hidden>Menu</button>"
    );
}

#[test]
fn conditional_option_attributes_escape_like_dynamic_values() {
    let hostile = Some("\"><script>alert('x')</script>");
    let expected = "<p title=\"&quot;&gt;&lt;script&gt;alert(&#x27;x&#x27;)&lt;/script&gt;\">x</p>";
    assert_eq!(crate::html! { <p title?={hostile}>"x"</p> }, expected);
    assert_eq!(crate::html! { <p title={hostile}>"x"</p> }, expected);

    let missing: Option<String> = None;
    assert_eq!(crate::html! { <p title?={missing}>"x"</p> }, "<p>x</p>");
    // `name={None}` keeps its previous output: the attribute, empty.
    assert_eq!(
        crate::html! { <p title={missing}>"x"</p> },
        "<p title=\"\">x</p>"
    );

    // Markup is escaped like text inside an attribute; RawHtml is trusted.
    let markup = Some(crate::html! { <b>"bold"</b> });
    assert_eq!(
        crate::html! { <p title?={markup}></p> },
        "<p title=\"&lt;b&gt;bold&lt;/b&gt;\"></p>"
    );
    let raw = Some(RawHtml::new("a&amp;b"));
    assert_eq!(
        crate::html! { <p title?={raw}></p> },
        "<p title=\"a&amp;b\"></p>"
    );

    // Borrowed values and temporaries.
    let label = Some(String::from("Ana & Bob"));
    assert_eq!(
        crate::html! { <p data-label?={&label} data-upper?={label.as_ref().map(|l| l.to_uppercase())}></p> },
        "<p data-label=\"Ana &amp; Bob\" data-upper=\"ANA &amp; BOB\"></p>"
    );
}

#[test]
fn conditional_url_attributes_keep_the_scheme_check() {
    for url in ["javascript:alert(1)", " JaVaScRiPt:alert(1)", "vbscript:x"] {
        assert_eq!(
            crate::html! { <a href?={Some(url)}>"x"</a> },
            "<a href=\"#\">x</a>",
            "{url:?}"
        );
    }
    let image = "data:image/png;base64,iVBORw0KGgo=";
    assert_eq!(
        crate::html! { <img src?={Some(image)} /> },
        format!("<img src=\"{image}\" />")
    );
    assert_eq!(
        crate::html! { <a href?={Some(image)}>"x"</a> },
        "<a href=\"#\">x</a>"
    );
    assert_eq!(
        crate::html! { <a href?={Some("/a?b=1&c=2")}>"x"</a> },
        "<a href=\"/a?b=1&amp;c=2\">x</a>"
    );
    assert_eq!(crate::html! { <a href?={None::<&str>}>"x"</a> }, "<a>x</a>");
}

struct Row {
    name: &'static str,
    admin: bool,
}

#[test]
fn if_blocks_render_the_matching_branch_with_escaping() {
    let badge = |user: Option<&str>, admin: bool| {
        crate::html! {
            <nav>
                @if let Some(name) = user {
                    <b>{name}</b>
                    @if admin { " (admin)" }
                } @else if admin {
                    "anonymous admin"
                } @else {
                    <a href="/login">"Sign in"</a>
                }
            </nav>
        }
    };
    assert_eq!(
        badge(Some("<Ana>"), true),
        "<nav><b>&lt;Ana&gt;</b> (admin)</nav>"
    );
    assert_eq!(badge(Some("Bob"), false), "<nav><b>Bob</b></nav>");
    assert_eq!(badge(None, true), "<nav>anonymous admin</nav>");
    assert_eq!(
        badge(None, false),
        "<nav><a href=\"/login\">Sign in</a></nav>"
    );
    // Without `@else`, a false condition renders nothing.
    let empty = false;
    assert_eq!(crate::html! { @if empty { "x" } }, "");
}

#[test]
fn for_blocks_repeat_their_body_and_compose_with_markup() {
    let rows = [
        Row {
            name: "Ana",
            admin: true,
        },
        Row {
            name: "<Bob>",
            admin: false,
        },
    ];
    let icon: Markup = crate::html! { <i class="star"></i> };
    let list = crate::html! {
        <ul>
            @for (index, row) in rows.iter().enumerate() {
                <li data-index={index} aria-current?={row.admin.then_some("true")}>
                    {row.name}
                    @if row.admin { {&icon} }
                </li>
            }
        </ul>
    };
    assert_eq!(
        list,
        "<ul><li data-index=\"0\" aria-current=\"true\">Ana<i class=\"star\"></i></li>\
         <li data-index=\"1\">&lt;Bob&gt;</li></ul>"
    );
    let none: [Row; 0] = [];
    assert_eq!(
        crate::html! { <ul>@for row in &none { <li>{row.name}</li> }</ul> },
        "<ul></ul>"
    );
}
