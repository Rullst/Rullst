extern crate self as rullst;

// Deliberately recognizable stand-ins isolate generated name resolution and
// selection of text/attribute helpers. Real escaping is tested by the facade.
pub mod html {
    pub fn escape(value: &impl std::fmt::Display) -> String {
        format!("text:{value}")
    }

    pub fn escape_attr(value: &impl std::fmt::Display) -> String {
        format!("attr:{value}")
    }

    /// Stand-in for the presence of a conditional `name?={value}` attribute.
    pub enum AttributePresence {
        Absent,
        Bare,
        Value(String),
    }

    pub trait OptionalAttribute {
        fn presence(&self) -> AttributePresence;
    }

    impl OptionalAttribute for bool {
        fn presence(&self) -> AttributePresence {
            if *self {
                AttributePresence::Bare
            } else {
                AttributePresence::Absent
            }
        }
    }

    impl<T: std::fmt::Display> OptionalAttribute for Option<T> {
        fn presence(&self) -> AttributePresence {
            match self {
                Some(value) => AttributePresence::Value(format!("opt:{value}")),
                None => AttributePresence::Absent,
            }
        }
    }

    pub fn optional_attr<T: OptionalAttribute + ?Sized>(value: &T) -> AttributePresence {
        value.presence()
    }

    /// Stand-in for the escaped-HTML type every expansion returns.
    #[derive(Debug, PartialEq)]
    pub struct Markup(pub String);

    impl Markup {
        pub fn from_trusted(html: impl Into<String>) -> Self {
            Self(html.into())
        }
    }

    impl std::fmt::Display for Markup {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str(&self.0)
        }
    }

    impl PartialEq<&str> for Markup {
        fn eq(&self, other: &&str) -> bool {
            self.0 == *other
        }
    }
}

use rullst_macros::html;

struct Label {
    text: &'static str,
}

#[test]
fn caller_named_s_is_resolved_in_element_text() {
    let s = Label { text: "caller" };
    assert_eq!(html! { <p>{s.text}</p> }, "<p>text:caller</p>");
    assert_eq!(s.text, "caller");
}

#[test]
fn compatible_string_type_cannot_silently_read_the_output_buffer() {
    let s = String::from("caller string");
    assert_eq!(html! { <p>{s}</p> }, "<p>text:caller string</p>");
    assert_eq!(s, "caller string");
}

#[test]
fn document_fragments_resolve_the_caller_in_every_root() {
    let s = Label { text: "caller" };
    assert_eq!(
        html! { {s.text}<p>{s.text}</p>{s.text} },
        "text:caller<p>text:caller</p>text:caller"
    );
}

#[test]
fn dynamic_attributes_resolve_the_caller_on_normal_and_void_elements() {
    let s = Label { text: "caller" };
    assert_eq!(
        html! { <p title={s.text}>{s.text}</p> },
        "<p title=\"attr:caller\">text:caller</p>"
    );
    assert_eq!(
        html! { <input value={s.text} required="true" /> },
        "<input value=\"attr:caller\" required=\"true\" />"
    );
}

#[test]
fn nested_elements_keep_their_own_buffers_and_the_callers_bindings() {
    let s = Label { text: "caller" };
    assert_eq!(
        html! { <main><p title={s.text}><b>{s.text}</b></p><br />{s.text}</main> },
        "<main><p title=\"attr:caller\"><b>text:caller</b></p><br />text:caller</main>"
    );
}

#[test]
fn iterator_binding_and_nested_macro_invocation_remain_in_caller_scope() {
    let labels = [Label { text: "first" }, Label { text: "second" }];
    let rows = labels
        .iter()
        .enumerate()
        .map(|(i, s)| html! { <li data-index={i}>{s.text}</li> }.0)
        .collect::<String>();
    assert_eq!(
        rows,
        "<li data-index=\"attr:0\">text:first</li><li data-index=\"attr:1\">text:second</li>"
    );

    let s = Label { text: "outer" };
    assert_eq!(
        html! { <div>{html! { <p>{s.text}</p> }}</div> },
        "<div>text:<p>text:outer</p></div>"
    );
}

#[test]
fn caller_mutations_evaluate_once_in_attribute_then_child_order() {
    let mut s = 0;
    let output = html! {
        <p data-index={{ s += 1; s }}>{{ s += 1; s }}</p>
        <br data-index={{ s += 1; s }} />
    };
    assert_eq!(
        output,
        "<p data-index=\"attr:1\">text:2</p><br data-index=\"attr:3\" />"
    );
    assert_eq!(s, 3);
}

#[test]
fn url_attributes_keep_caller_bindings_temporaries_and_evaluation_order() {
    let s = Label { text: "caller" };
    assert_eq!(
        html! { <a href={s.text} title={s.text}>{s.text}</a> },
        "<a href=\"attr:caller\" title=\"attr:caller\">text:caller</a>"
    );
    assert_eq!(
        html! { <img src={format!("/{}", s.text)} /> },
        "<img src=\"attr:/caller\" />"
    );

    let mut s = 0;
    let output = html! {
        <a href={{ s += 1; s }} data-index={{ s += 1; s }}>{{ s += 1; s }}</a>
    };
    assert_eq!(
        output,
        "<a href=\"attr:1\" data-index=\"attr:2\">text:3</a>"
    );
}

#[test]
fn doctype_and_keyword_element_names_expand_to_markup() {
    let s = Label { text: "caller" };
    assert_eq!(
        html! {
            <!DOCTYPE html>
            <html><body><svg><use href="#i" /></svg>{s.text}</body></html>
        },
        "<!DOCTYPE html><html><body><svg><use href=\"#i\"></use></svg>text:caller</body></html>"
    );
}

#[test]
fn bare_and_conditional_attributes_resolve_the_caller() {
    // `value` is also the name of the generated match binding.
    let value = Some("caller");
    let s = true;
    assert_eq!(
        html! { <input required title?={value} checked?={s} hidden?={!s} /> },
        "<input required title=\"opt:caller\" checked />"
    );
    let value: Option<&str> = None;
    assert_eq!(
        html! { <a href?={value} data-on?={value}>"x"</a> },
        "<a>x</a>"
    );
    assert_eq!(
        html! { <a href?={Some(format!("/{}", "caller"))}>"x"</a> },
        "<a href=\"opt:/caller\">x</a>"
    );
}

#[test]
fn conditional_attributes_evaluate_once_in_source_order() {
    let mut s = 0;
    let output = html! {
        <p data-a={{ s += 1; s }} data-b?={{ s += 1; Some(s) }} data-c?={{ s += 1; s > 5 }}>
            {{ s += 1; s }}
        </p>
    };
    assert_eq!(output, "<p data-a=\"attr:1\" data-b=\"opt:2\">text:4</p>");
    assert_eq!(s, 4);
}

#[test]
fn control_blocks_resolve_caller_bindings_named_like_the_buffer() {
    let s = vec![Label { text: "a" }, Label { text: "b" }];
    assert_eq!(
        html! { <ul>@for s in &s { <li>{s.text}</li> }</ul> },
        "<ul><li>text:a</li><li>text:b</li></ul>"
    );
    assert_eq!(
        html! { @for (i, s) in s.iter().enumerate() { {i}{s.text} } },
        "text:0text:atext:1text:b"
    );
    assert_eq!(
        html! { @if s.is_empty() { "none" } @else if s.len() == 2 { {s[1].text} } @else { "many" } },
        "text:b"
    );
    let s: Option<Label> = None;
    assert_eq!(
        html! { <p>@if let Some(s) = &s { {s.text} } @else { "no label" }</p> },
        "<p>no label</p>"
    );
    let s = Some(Label { text: "caller" });
    assert_eq!(
        html! { <p>@if let Some(s) = &s { {s.text} } @else { "no label" }</p> },
        "<p>text:caller</p>"
    );
    assert_eq!(s.map(|label| label.text), Some("caller"));
}

#[test]
fn match_blocks_resolve_caller_bindings_named_like_the_buffer() {
    let s = Some(Label { text: "a" });
    assert_eq!(
        html! { <p>@match &s { Some(s) if s.text.is_empty() => { "empty" } Some(s) => { {s.text} } None => { "none" } }</p> },
        "<p>text:a</p>"
    );
    let s: Option<Label> = None;
    assert_eq!(
        html! { @match s { Some(s) => { {s.text} } None => { "none" } } },
        "none"
    );
    let s = 3;
    assert_eq!(
        html! { @match s { 0 => { "zero" }, n @ 1..=2 => { {n} }, n if n > 2 => { "many:" {n} } _ => {} } },
        "many:text:3"
    );
}
