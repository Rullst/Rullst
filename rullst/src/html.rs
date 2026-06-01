/// Trait implemented by types that can be rendered safely into HTML.
pub trait HtmlEscape {
    fn escape_html(&self) -> String;
}

/// A wrapper to mark a string as safe raw HTML that should NOT be escaped.
pub struct RawHtml(pub String);

impl RawHtml {
    pub fn new<S: Into<String>>(s: S) -> Self {
        RawHtml(s.into())
    }
}

impl HtmlEscape for RawHtml {
    fn escape_html(&self) -> String {
        self.0.clone()
    }
}

impl HtmlEscape for String {
    fn escape_html(&self) -> String {
        escape_str(self)
    }
}

impl HtmlEscape for &str {
    fn escape_html(&self) -> String {
        escape_str(self)
    }
}

impl<T: HtmlEscape + ?Sized> HtmlEscape for &T {
    fn escape_html(&self) -> String {
        (*self).escape_html()
    }
}

// Automatically treat numbers and booleans as safe since they cannot contain HTML characters
macro_rules! impl_safe_primitives {
    ($($t:ty),*) => {
        $(
            impl HtmlEscape for $t {
                fn escape_html(&self) -> String {
                    self.to_string()
                }
            }
        )*
    };
}

impl_safe_primitives!(
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64, bool
);

/// Helper function to escape standard strings
pub fn escape_str(s: &str) -> String {
    let mut escaped = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '&' => escaped.push_str("&amp;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#x27;"),
            _ => escaped.push(c),
        }
    }
    escaped
}

/// The core escape function invoked by the `html!` macro
pub fn escape<T: HtmlEscape + ?Sized>(val: &T) -> String {
    val.escape_html()
}

/// Helper function to escape attribute values
pub fn escape_attr<T: HtmlEscape + ?Sized>(val: &T) -> String {
    val.escape_html()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_complex_javascript_types() {
        let js = r#"<script>let a = {"b": 1, c: '2', d: [1,2,3]}; alert(a);</script>"#;
        let expected = "&lt;script&gt;let a = {&quot;b&quot;: 1, c: &#x27;2&#x27;, d: [1,2,3]}; alert(a);&lt;/script&gt;";
        assert_eq!(escape_str(js), expected);
    }
}
