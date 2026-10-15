extern crate self as rullst;

/// Stand-ins for the `rullst::i18n` runtime: these cases fail before it is used.
pub mod i18n {
    pub struct Locales;

    impl Locales {
        pub const fn __from_catalog(_: &'static [&'static str], _: usize) -> Self {
            Locales
        }
    }

    pub use rullst_macros::__i18n_translate as __translate;
}

rullst_macros::i18n_catalog!(default = "en", dir = "tests/i18n/valid");

fn main() {
    let _ = t!((), "dashboard.greeting");
    let _ = t!((), "nav.home", name = "Ana");
}
