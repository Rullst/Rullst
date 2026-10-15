//! The `Markup` type returned by `html!`.

use std::borrow::Cow;
use std::fmt;

use axum::response::{Html, IntoResponse, Response};

use super::{HtmlEscape, RawHtml};

/// Escaped HTML produced by `html!`.
///
/// `html!` returns `Markup`, and interpolating a `Markup` (or `&Markup`) with
/// `{value}` inside another `html!` inserts it as it is, so views compose
/// without wrappers. `String`, `&str`, numbers and other values are still
/// escaped, `Option<T>` renders nothing for `None`, and an iterator of
/// `Markup` collects into one `Markup`. In an attribute value
/// (`title={markup}`) a `Markup` is escaped like text, since HTML there
/// could end the attribute.
///
/// # Composing views
///
/// ```
/// # extern crate rullst_core as rullst;
/// use rullst::html;
/// use rullst::html::Markup;
///
/// fn badge(label: &str) -> Markup {
///     html! { <span class="badge">{label}</span> }
/// }
///
/// let rows: Markup = ["Ana", "<Bob>"]
///     .iter()
///     .map(|name| html! { <li>{name}{badge("new")}</li> })
///     .collect();
/// let list = html! { <ul>{rows}</ul> };
/// assert_eq!(
///     list,
///     "<ul><li>Ana<span class=\"badge\">new</span></li>\
///      <li>&lt;Bob&gt;<span class=\"badge\">new</span></li></ul>"
/// );
///
/// let note: Option<Markup> = None;
/// assert_eq!(html! { <p>"Total"{note}</p> }, "<p>Total</p>");
/// ```
///
/// A handler can return `Markup` directly: it responds with
/// `text/html; charset=utf-8`, like `axum::response::Html`.
///
/// # Trust boundary
///
/// A `String` never becomes `Markup` implicitly, so text cannot reach a page
/// unescaped by accident:
///
/// ```compile_fail
/// # extern crate rullst_core as rullst;
/// let name = String::from("<script>alert(1)</script>");
/// let markup: rullst::html::Markup = name.into();
/// ```
///
/// HTML produced elsewhere (a Markdown renderer, a sanitizer, a trusted
/// template) is inserted with [`Markup::from_trusted`] or [`RawHtml`], which
/// both trust their input. Never pass request data to them.
///
/// `Markup` reads like a `&str` (`contains`, `len`, comparisons with strings)
/// but offers no mutable access to its text: use [`Markup::into_string`] to
/// take the `String` out.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Markup(String);

impl Markup {
    /// Wraps HTML that is trusted to be safe as it is, without escaping it.
    ///
    /// Only for HTML from a trusted source, such as a sanitizer or a
    /// renderer you control. Text from a request must be interpolated with
    /// `html!` instead, which escapes it.
    pub fn from_trusted(html: impl Into<String>) -> Self {
        Self(html.into())
    }

    /// The HTML as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Takes the HTML `String` out of the `Markup`.
    pub fn into_string(self) -> String {
        self.0
    }
}

impl HtmlEscape for Markup {
    fn escape_html(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.0)
    }

    // HTML is never valid inside an attribute value, and its quotes could end
    // the attribute: escape it like text there.
    fn escape_attribute(&self) -> Cow<'_, str> {
        super::escape_str(&self.0)
    }
}

impl From<RawHtml> for Markup {
    fn from(raw: RawHtml) -> Self {
        Self(raw.0)
    }
}

impl From<Markup> for String {
    fn from(markup: Markup) -> Self {
        markup.0
    }
}

impl From<Markup> for axum::body::Body {
    fn from(markup: Markup) -> Self {
        Self::from(markup.0)
    }
}

impl fmt::Display for Markup {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl AsRef<str> for Markup {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::ops::Deref for Markup {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

impl FromIterator<Markup> for Markup {
    fn from_iter<I: IntoIterator<Item = Markup>>(iter: I) -> Self {
        let mut markup = Self::default();
        markup.extend(iter);
        markup
    }
}

impl Extend<Markup> for Markup {
    fn extend<I: IntoIterator<Item = Markup>>(&mut self, iter: I) {
        for markup in iter {
            self.0.push_str(&markup.0);
        }
    }
}

impl IntoResponse for Markup {
    fn into_response(self) -> Response {
        Html(self.0).into_response()
    }
}

impl PartialEq<str> for Markup {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for Markup {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl PartialEq<String> for Markup {
    fn eq(&self, other: &String) -> bool {
        self.0 == *other
    }
}

impl PartialEq<Markup> for str {
    fn eq(&self, other: &Markup) -> bool {
        self == other.0
    }
}

impl PartialEq<Markup> for &str {
    fn eq(&self, other: &Markup) -> bool {
        *self == other.0
    }
}

impl PartialEq<Markup> for String {
    fn eq(&self, other: &Markup) -> bool {
        *self == other.0
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "html_markup_tests.rs"]
mod tests;
