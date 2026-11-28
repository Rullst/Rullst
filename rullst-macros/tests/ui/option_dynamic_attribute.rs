#![deny(deprecated)]
extern crate self as rullst;

/// Stand-ins for `rullst::html`: only the `Option` values fail.
pub mod html {
    pub struct Markup;

    impl Markup {
        pub fn from_trusted(_: String) -> Self {
            Markup
        }
    }

    pub fn push_escaped<T: ?Sized>(_: &mut String, _: &T) {
        // Writes nothing: only the `Option` values fail.
    }

    pub fn push_escaped_attr<T: ?Sized>(_: &mut String, _: &T) {
        // Writes nothing.
    }
}

use rullst_macros::html;

fn main() {
    let note: Option<&str> = None;
    let owned: Option<String> = None;
    let label = "Create";
    let busy = false;
    let _ = html! {
        <button disabled={note} title={&owned} aria-busy={busy} name={label}>{note}</button>
    };
}
