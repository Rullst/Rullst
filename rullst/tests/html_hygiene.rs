use rullst::html;

struct Label {
    text: &'static str,
}

#[test]
fn caller_fields_are_escaped_in_fragments_attributes_and_nested_elements() {
    let s = Label { text: "<&\"'" };
    let rendered = html! {
        {s.text}
        <p title={s.text}><b>{s.text}</b></p>
        <input value={s.text} />
    };
    assert_eq!(
        rendered,
        "&lt;&amp;&quot;&#x27;<p title=\"&lt;&amp;&quot;&#x27;\"><b>&lt;&amp;&quot;&#x27;</b></p><input value=\"&lt;&amp;&quot;&#x27;\" />"
    );
    assert_eq!(s.text, "<&\"'");
}

#[test]
fn caller_string_is_borrowed_instead_of_capturing_generated_markup() {
    let s = String::from("<caller>");
    assert_eq!(html! { <p>{s}</p> }, "<p>&lt;caller&gt;</p>");
    // An owned expression used to compile while reading the generated buffer.
    assert_eq!(html! { <p>{s.clone()}</p> }, "<p>&lt;caller&gt;</p>");
    assert_eq!(s, "<caller>");
}

#[test]
fn closure_binding_keeps_escaped_content_and_nested_markup() {
    let values = [Label { text: "<one>" }, Label { text: "&two" }];
    let s = values
        .iter()
        .enumerate()
        .map(|(i, s)| html! { <li data-index={i}>{s.text}</li> })
        .collect::<html::Markup>();
    assert_eq!(
        html! { <ul>{s}</ul> },
        "<ul><li data-index=\"0\">&lt;one&gt;</li><li data-index=\"1\">&amp;two</li></ul>"
    );
    // A String taken out of the Markup is text again.
    let s = html! { <b>"x"</b> }.into_string();
    assert_eq!(html! { <p>{s}</p> }, "<p>&lt;b&gt;x&lt;/b&gt;</p>");
}

#[test]
fn caller_mutation_occurs_once_per_expression_in_document_order() {
    let mut s = 0;
    let output = html! {
        <p data-index={{ s += 1; s }}>{{ s += 1; s }}</p>
        <br data-index={{ s += 1; s }} />
    };
    assert_eq!(output, "<p data-index=\"1\">2</p><br data-index=\"3\" />");
    assert_eq!(s, 3);
}
