use super::HtmlDocument;

fn parse_error(source: &str) -> String {
    match syn::parse_str::<HtmlDocument>(source) {
        Ok(_) => panic!("expected a parse error for {source}"),
        Err(error) => error.to_string(),
    }
}

#[test]
fn dynamic_event_handler_attributes_are_rejected() {
    for (source, name) in [
        (r#"<button onclick={handler}>"Go"</button>"#, "onclick"),
        (r#"<body ONLOAD={handler}></body>"#, "ONLOAD"),
        (r#"<div hx-on={handler}></div>"#, "hx-on"),
        (r#"<div hx-on-click={handler}></div>"#, "hx-on-click"),
        (
            r#"<div data-hx-on-click={handler}></div>"#,
            "data-hx-on-click",
        ),
    ] {
        let error = parse_error(source);
        assert!(
            error.starts_with(&format!("dynamic `{name}` values are not allowed")),
            "{error}"
        );
    }
}

#[test]
fn static_handlers_and_other_dynamic_attributes_are_accepted() {
    for source in [
        r#"<button onclick="go()">"Go"</button>"#,
        r#"<div hx-on-click="go()"></div>"#,
        r#"<div data-on={value} one-time={value} on={value}></div>"#,
        r#"<a href={url} style={style}>"x"</a>"#,
    ] {
        assert!(syn::parse_str::<HtmlDocument>(source).is_ok(), "{source}");
    }
}
