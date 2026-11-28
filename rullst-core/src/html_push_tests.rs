//! `HtmlEscape::push_html` and `push_attribute`, which `html!` calls to write
//! values into its one buffer: each appends the text `escape_html` and
//! `escape_attribute` return, and numbers are formatted as `to_string`
//! formats them.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::borrow::Cow;

use super::{HtmlEscape, Markup, RawHtml, escape_str, push_escaped, push_escaped_attr};

fn html<T: HtmlEscape + ?Sized>(value: &T) -> String {
    let mut out = String::from("[");
    value.push_html(&mut out);
    out
}

fn attribute<T: HtmlEscape + ?Sized>(value: &T) -> String {
    let mut out = String::from("[");
    value.push_attribute(&mut out);
    out
}

#[test]
fn text_is_escaped_as_escape_str_escapes_it() {
    for (text, escaped) in [
        ("", "["),
        ("plain", "[plain"),
        ("<", "[&lt;"),
        (">", "[&gt;"),
        ("&", "[&amp;"),
        ("\"", "[&quot;"),
        ("'", "[&#x27;"),
        ("a<b>c&d\"e'f", "[a&lt;b&gt;c&amp;d&quot;e&#x27;f"),
        ("<<>>", "[&lt;&lt;&gt;&gt;"),
        ("<x", "[&lt;x"),
        ("x>", "[x&gt;"),
        ("é<ü>ñ", "[é&lt;ü&gt;ñ"),
        ("フ&レ", "[フ&amp;レ"),
    ] {
        assert_eq!(html(&text), escaped, "{text:?}");
        assert_eq!(attribute(&text), escaped, "{text:?}");
        assert_eq!(html(&String::from(text)), escaped, "{text:?}");
        assert_eq!(attribute(&String::from(text)), escaped, "{text:?}");
        assert_eq!(html(&Cow::Borrowed(text)), escaped, "{text:?}");
        assert_eq!(attribute(&Cow::<str>::Owned(text.to_string())), escaped);
        assert_eq!(&escaped[1..], escape_str(text), "{text:?}");
    }
    // Every ASCII character, against `escape_str`.
    for byte in 0u8..128 {
        let text = char::from(byte).to_string();
        assert_eq!(html(&text.as_str())[1..], escape_str(&text), "{byte}");
    }
}

#[test]
fn numbers_and_booleans_are_formatted_as_to_string_formats_them() {
    assert_eq!(html(&-42_i32), "[-42");
    assert_eq!(attribute(&-42_i32), "[-42");
    assert_eq!(html(&u64::MAX), "[18446744073709551615");
    assert_eq!(
        attribute(&i128::MIN),
        "[-170141183460469231731687303715884105728"
    );
    assert_eq!(html(&0_u8), "[0");
    assert_eq!(html(&1.5_f64), "[1.5");
    assert_eq!(attribute(&-0.0_f32), "[-0");
    assert_eq!(html(&f64::NAN), "[NaN");
    assert_eq!(html(&f64::NEG_INFINITY), "[-inf");
    assert_eq!(html(&1e21_f64), "[1000000000000000000000");
    assert_eq!(html(&true), "[true");
    assert_eq!(attribute(&false), "[false");
}

#[test]
fn integers_are_written_digit_for_digit() {
    for (value, text) in [
        (0_u64, "[0"),
        (9, "[9"),
        (10, "[10"),
        (99, "[99"),
        (100, "[100"),
        (1_234_567_890, "[1234567890"),
        (u64::MAX, "[18446744073709551615"),
    ] {
        assert_eq!(html(&value), text);
    }
    assert_eq!(html(&-1_i64), "[-1");
    assert_eq!(html(&i64::MIN), "[-9223372036854775808");
    assert_eq!(html(&i64::MAX), "[9223372036854775807");
    assert_eq!(html(&i8::MIN), "[-128");
    assert_eq!(html(&-10_isize), "[-10");
    assert_eq!(html(&u128::MAX), "[340282366920938463463374607431768211455");
    assert_eq!(html(&0_u128), "[0");
    assert_eq!(html(&-7_i128), "[-7");
    assert_eq!(html(&i128::MAX), "[170141183460469231731687303715884105727");
    // Every 16-bit value, and powers of ten around every width, against
    // `to_string`.
    for value in i16::MIN..=i16::MAX {
        assert_eq!(html(&value)[1..], value.to_string());
    }
    for value in u16::MIN..=u16::MAX {
        assert_eq!(attribute(&value)[1..], value.to_string());
    }
    let mut power = 1_u128;
    while let Some(next) = power.checked_mul(10) {
        for value in [power - 1, power, power + 1] {
            assert_eq!(html(&value)[1..], value.to_string());
            if let Ok(narrow) = u64::try_from(value) {
                assert_eq!(html(&narrow)[1..], narrow.to_string());
                assert_eq!(html(&(narrow as usize))[1..], (narrow as usize).to_string());
            }
            if let Ok(signed) = i128::try_from(value) {
                assert_eq!(html(&-signed)[1..], (-signed).to_string());
            }
            if let Ok(signed) = i64::try_from(value) {
                assert_eq!(html(&-signed)[1..], (-signed).to_string());
                assert_eq!(attribute(&signed)[1..], signed.to_string());
            }
        }
        power = next;
    }
    assert_eq!(html(&u32::MAX), "[4294967295");
    assert_eq!(html(&i32::MIN), "[-2147483648");
    assert_eq!(html(&u8::MAX), "[255");
}

#[test]
fn wrappers_delegate_to_their_value() {
    assert_eq!(html(&&"<a>"), "[&lt;a&gt;");
    assert_eq!(attribute(&&&"<a>"), "[&lt;a&gt;");
    assert_eq!(html(&Some("<a>")), "[&lt;a&gt;");
    assert_eq!(attribute(&Some(7_u16)), "[7");
    assert_eq!(html(&None::<&str>), "[");
    assert_eq!(attribute(&None::<i32>), "[");
    assert_eq!(html(&Some(Some("&"))), "[&amp;");
}

#[test]
fn markup_is_inserted_as_html_and_escaped_in_attributes() {
    let markup = Markup::from_trusted("<b title=\"t\">x & y</b>");
    assert_eq!(html(&markup), "[<b title=\"t\">x & y</b>");
    assert_eq!(
        attribute(&markup),
        "[&lt;b title=&quot;t&quot;&gt;x &amp; y&lt;/b&gt;"
    );
    let raw = RawHtml::new("<i>'raw'</i>");
    assert_eq!(html(&raw), "[<i>'raw'</i>");
    assert_eq!(attribute(&raw), "[<i>'raw'</i>");
}

/// An application type that implements only the required method, and one
/// that also escapes attributes its own way.
struct Shout(&'static str);

impl HtmlEscape for Shout {
    fn escape_html(&self) -> Cow<'_, str> {
        Cow::Owned(self.0.to_uppercase())
    }
}

struct Quoted;

impl HtmlEscape for Quoted {
    fn escape_html(&self) -> Cow<'_, str> {
        Cow::Borrowed("text")
    }

    fn escape_attribute(&self) -> Cow<'_, str> {
        Cow::Borrowed("attribute")
    }
}

#[test]
fn the_default_methods_append_what_escape_html_and_escape_attribute_return() {
    assert_eq!(html(&Shout("hi")), "[HI");
    assert_eq!(attribute(&Shout("hi")), "[HI");
    assert_eq!(html(&Quoted), "[text");
    assert_eq!(attribute(&Quoted), "[attribute");
}

#[test]
fn the_expansion_helpers_dispatch_to_the_value() {
    let mut out = String::from("<p>");
    push_escaped(&mut out, &Quoted);
    push_escaped(&mut out, &3_i8);
    push_escaped_attr(&mut out, &Quoted);
    push_escaped_attr(&mut out, &"\"");
    assert_eq!(out, "<p>text3attribute&quot;");
}
