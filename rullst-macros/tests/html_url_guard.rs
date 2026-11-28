extern crate self as rullst;

// Stand-ins that pass values through unchanged (escaping only `"`), so the
// generated URL guard sees the scheme a browser would. Real escaping is
// tested by the facade; these tests check which attributes and elements the
// expansion guards.
pub mod html {
    pub fn escape(value: &impl std::fmt::Display) -> String {
        value.to_string()
    }

    pub fn escape_attr(value: &impl std::fmt::Display) -> String {
        value.to_string().replace('"', "&quot;")
    }

    pub fn push_escaped(out: &mut String, value: &impl std::fmt::Display) {
        out.push_str(&escape(value));
    }

    pub fn push_escaped_attr(out: &mut String, value: &impl std::fmt::Display) {
        out.push_str(&escape_attr(value));
    }

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
                Some(value) => AttributePresence::Value(escape_attr(value)),
                None => AttributePresence::Absent,
            }
        }
    }

    pub fn optional_attr<T: OptionalAttribute + ?Sized>(value: &T) -> AttributePresence {
        value.presence()
    }

    #[derive(Debug, PartialEq)]
    pub struct Markup(pub String);

    impl Markup {
        pub fn from_trusted(html: impl Into<String>) -> Self {
            Self(html.into())
        }
    }

    impl PartialEq<&str> for Markup {
        fn eq(&self, other: &&str) -> bool {
            self.0 == *other
        }
    }
}

use rullst_macros::html;

const SCRIPT: &str = "javascript:alert(1)";
const DATA: &str = "data:text/html,<b>x</b>";

#[test]
fn every_url_attribute_replaces_a_script_url() {
    let url = SCRIPT;
    assert_eq!(
        html! {
            <form action={url} data={url} formaction={url}>
                <a href={url} cite={url} xlink:href={url}>"a"</a>
                <img src={url} longdesc={url} usemap={url} />
                <video poster={url}></video>
                <body background={url}></body>
                <object codebase={url}></object>
                <menuitem icon={url}></menuitem>
                <html manifest={url}></html>
            </form>
        },
        "<form action=\"#\" data=\"#\" formaction=\"#\"><a href=\"#\" cite=\"#\" \
         xlink:href=\"#\">a</a><img src=\"#\" longdesc=\"#\" usemap=\"#\" /><video \
         poster=\"#\"></video><body background=\"#\"></body><object codebase=\"#\"></object>\
         <menuitem icon=\"#\"></menuitem><html manifest=\"#\"></html></form>"
    );
}

#[test]
fn url_attribute_names_match_in_any_case() {
    let url = SCRIPT;
    assert_eq!(
        html! { <a HREF={url} Src={url}>"a"</a> },
        "<a HREF=\"#\" Src=\"#\">a</a>"
    );
}

#[test]
fn other_attributes_keep_a_script_like_value() {
    let url = SCRIPT;
    assert_eq!(
        html! { <a title={url} data-href={url} hrefs={url}>"a"</a> },
        "<a title=\"javascript:alert(1)\" data-href=\"javascript:alert(1)\" \
         hrefs=\"javascript:alert(1)\">a</a>"
    );
}

#[test]
fn data_urls_stay_only_on_media_elements() {
    let url = DATA;
    assert_eq!(
        html! {
            <div>
                <img src={url} /><IMG src={url} /><image href={url}></image>
                <audio src={url}></audio><video src={url}></video>
                <source src={url} /><track src={url} />
                <a href={url}>"a"</a><iframe src={url}></iframe>
            </div>
        },
        "<div><img src=\"data:text/html,<b>x</b>\" /><IMG src=\"data:text/html,<b>x</b>\"></IMG>\
         <image href=\"data:text/html,<b>x</b>\"></image><audio src=\"data:text/html,<b>x</b>\">\
         </audio><video src=\"data:text/html,<b>x</b>\"></video><source \
         src=\"data:text/html,<b>x</b>\" /><track src=\"data:text/html,<b>x</b>\" /><a \
         href=\"#\">a</a><iframe src=\"#\"></iframe></div>"
    );
    // A script URL is replaced on media elements too.
    let url = SCRIPT;
    assert_eq!(html! { <img src={url} /> }, "<img src=\"#\" />");
}

#[test]
fn conditional_url_attributes_are_guarded() {
    let script = Some(SCRIPT);
    let data = Some(DATA);
    let safe = Some("/path?a=1");
    assert_eq!(
        html! { <a href?={script} cite?={safe}>"a"</a> },
        "<a href=\"#\" cite=\"/path?a=1\">a</a>"
    );
    assert_eq!(
        html! { <img src?={data} /> },
        "<img src=\"data:text/html,<b>x</b>\" />"
    );
    assert_eq!(html! { <a href?={data}>"a"</a> }, "<a href=\"#\">a</a>");
    // A conditional value that is not a URL attribute is written unchanged.
    assert_eq!(
        html! { <a title?={script}>"a"</a> },
        "<a title=\"javascript:alert(1)\">a</a>"
    );
}

#[test]
fn safe_urls_and_static_values_are_written_unchanged() {
    let url = "https://example.com/a?b=\"c\"";
    assert_eq!(
        html! { <a href={url} title="javascript:static">"a"</a> },
        "<a href=\"https://example.com/a?b=&quot;c&quot;\" title=\"javascript:static\">a</a>"
    );
}
