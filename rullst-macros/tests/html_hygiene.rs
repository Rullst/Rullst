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
        .map(|(i, s)| html! { <li data-index={i}>{s.text}</li> })
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
