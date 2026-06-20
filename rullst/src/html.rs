use std::borrow::Cow;

/// Trait implemented by types that can be rendered safely into HTML.
pub trait HtmlEscape {
    /// Escapes characters in `self` so they can be rendered safely as HTML.
    fn escape_html(&self) -> Cow<'_, str>;
}

/// A wrapper to mark a string as safe raw HTML that should NOT be escaped.
pub struct RawHtml(pub String);

impl RawHtml {
    /// Wraps a raw string as HTML that is trusted not to be escaped.
    pub fn new<S: Into<String>>(s: S) -> Self {
        RawHtml(s.into())
    }
}

impl HtmlEscape for RawHtml {
    fn escape_html(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.0)
    }
}

impl HtmlEscape for String {
    fn escape_html(&self) -> Cow<'_, str> {
        escape_str(self)
    }
}

impl HtmlEscape for &str {
    fn escape_html(&self) -> Cow<'_, str> {
        escape_str(self)
    }
}

impl<T: HtmlEscape + ?Sized> HtmlEscape for &T {
    fn escape_html(&self) -> Cow<'_, str> {
        (*self).escape_html()
    }
}

// Automatically treat numbers and booleans as safe since they cannot contain HTML characters
macro_rules! impl_safe_primitives {
    ($($t:ty),*) => {
        $(
            impl HtmlEscape for $t {
                fn escape_html(&self) -> Cow<'_, str> {
                    Cow::Owned(self.to_string())
                }
            }
        )*
    };
}

impl_safe_primitives!(
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64, bool
);

/// Helper function to escape standard strings
pub fn escape_str(s: &str) -> Cow<'_, str> {
    let bytes = s.as_bytes();
    let mut last_pos = 0;
    let mut escaped = String::with_capacity(0);

    for (i, &b) in bytes.iter().enumerate() {
        let replacement = match b {
            b'<' => "&lt;",
            b'>' => "&gt;",
            b'&' => "&amp;",
            b'"' => "&quot;",
            b'\'' => "&#x27;",
            _ => continue,
        };

        if last_pos == 0 {
            escaped.reserve_exact(s.len() + 16);
        }

        escaped.push_str(&s[last_pos..i]);
        escaped.push_str(replacement);
        last_pos = i + 1;
    }

    if last_pos == 0 {
        Cow::Borrowed(s)
    } else {
        escaped.push_str(&s[last_pos..]);
        Cow::Owned(escaped)
    }
}

/// The core escape function invoked by the `html!` macro
pub fn escape<T: HtmlEscape + ?Sized>(val: &T) -> Cow<'_, str> {
    val.escape_html()
}

/// Helper function to escape attribute values
pub fn escape_attr<T: HtmlEscape + ?Sized>(val: &T) -> Cow<'_, str> {
    val.escape_html()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_complex_javascript_types() {
        let js = r#"<script>let a = {"b": 1, c: '2', d: [1,2,3]}; alert(a);</script>"#;
        let expected = "&lt;script&gt;let a = {&quot;b&quot;: 1, c: &#x27;2&#x27;, d: [1,2,3]}; alert(a);&lt;/script&gt;";
        assert_eq!(escape_str(js), expected);
    }

    #[test]
    fn test_escape_str_edge_cases() {
        assert_eq!(escape_str(""), "");
        assert_eq!(escape_str("Café & croissant"), "Café &amp; croissant");
        assert_eq!(escape_str("<script>"), "&lt;script&gt;");
        assert_eq!(escape_str("\"'"), "&quot;&#x27;");
    }

    #[test]
    fn test_raw_html_new() {
        let raw = RawHtml::new("<b>bold</b>");
        assert_eq!(raw.0, "<b>bold</b>");
    }
}
