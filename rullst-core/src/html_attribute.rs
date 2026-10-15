//! Conditional attributes, written `name?={value}` inside `html!`.

use std::borrow::Cow;

use super::HtmlEscape;

/// Whether and how `html!` writes a conditional attribute (`name?={value}`).
///
/// The set is closed: a conditional attribute is left out, written bare or
/// written with a value. Added in v13.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AttributePresence<'a> {
    /// The attribute is left out.
    Absent,
    /// The attribute is written without a value, as in `<input required>`.
    Bare,
    /// The attribute is written as `name="value"`; the value is already
    /// escaped for a double-quoted attribute, exactly as `name={value}` is.
    Value(Cow<'a, str>),
}

/// A value for a conditional attribute, `name?={value}`, in `html!`.
///
/// - `bool`: `true` writes the bare attribute and `false` leaves it out,
///   which is what HTML boolean attributes (`checked`, `disabled`,
///   `required`, `selected`, `hidden`, …) mean: their presence is the value,
///   so `disabled="false"` still disables a control.
/// - `Option<T>`: `Some(value)` writes `name="value"`, escaped exactly like
///   `name={value}` (URL attributes keep their `javascript:` check), and
///   `None` leaves the attribute out.
///
/// `name={value}` is unchanged: it always writes the attribute, a `bool` as
/// `"true"`/`"false"` (what `aria-*` and `data-*` attributes expect) and
/// `None` as an empty value. Added in v13.
///
/// ```
/// # extern crate rullst_core as rullst;
/// use rullst::html;
///
/// let done = true;
/// let note: Option<&str> = None;
/// assert_eq!(
///     html! { <input type="checkbox" checked?={done} title?={note} /> },
///     "<input type=\"checkbox\" checked />"
/// );
/// ```
///
/// Any other value is a compile-time error, so a plain string cannot be
/// mistaken for a condition:
///
/// ```compile_fail
/// # extern crate rullst_core as rullst;
/// let title = "always";
/// let _ = rullst::html! { <p title?={title}></p> };
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be the value of a conditional `name?={{value}}` attribute",
    label = "expected a `bool` or an `Option<_>`",
    note = "`name?={{flag}}` takes a `bool` (bare attribute or nothing) or an `Option<T>` \
            (`name=\"value\"` or nothing); write `name={{value}}` for an attribute that \
            is always present"
)]
pub trait OptionalAttribute {
    /// Returns whether and how the attribute is written.
    fn attribute_presence(&self) -> AttributePresence<'_>;
}

impl OptionalAttribute for bool {
    fn attribute_presence(&self) -> AttributePresence<'_> {
        if *self {
            AttributePresence::Bare
        } else {
            AttributePresence::Absent
        }
    }
}

impl<T: HtmlEscape> OptionalAttribute for Option<T> {
    fn attribute_presence(&self) -> AttributePresence<'_> {
        match self {
            Some(value) => AttributePresence::Value(value.escape_attribute()),
            None => AttributePresence::Absent,
        }
    }
}

impl<T: OptionalAttribute + ?Sized> OptionalAttribute for &T {
    fn attribute_presence(&self) -> AttributePresence<'_> {
        (**self).attribute_presence()
    }
}

/// Evaluates a conditional attribute; `html!` calls it for `name?={value}`.
#[inline(always)]
pub fn optional_attr<T: OptionalAttribute + ?Sized>(value: &T) -> AttributePresence<'_> {
    value.attribute_presence()
}
