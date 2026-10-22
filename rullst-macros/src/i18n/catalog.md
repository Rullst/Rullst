Loads the application's translation catalogs at compile time and defines
`LOCALES` and the `t!` macro in the module where it is invoked.

```rust,ignore
// src/i18n.rs
rullst::i18n::catalog!(default = "en");
// or: rullst::i18n::catalog!(default = "pt-BR", dir = "config/locales");
```

`dir` (default `"locales"`) is relative to the crate's `Cargo.toml` and holds
one file per locale named by its tag: `locales/en.toml`,
`locales/pt-BR.toml`. A tag is `language[-Script][-REGION]` in canonical case.
Nested TOML tables form dotted keys, and a table whose entries are CLDR
plural categories (`zero`, `one`, `two`, `few`, `many`, `other`) with
`other` is a plural message:

```toml
[dashboard]
title = "Dashboard"
greeting = "Hello, {name}!"

[inbox.unread]
one = "You have one unread message"
other = "You have {count} unread messages"
```

The build fails when a key is missing from any locale, when the same key has
different placeholders or shapes (plural or single) in two locales, when a
plural message lacks a category its language's CLDR rule selects (the error
names it) or has one it never selects, when a template has an unbalanced
brace, and when `default` has no file. `{{` and
`}}` write literal braces.

The generated items are:

- `pub static LOCALES: rullst::i18n::Locales`, the supported locales for
  negotiation (`LOCALES.layer()`, `LOCALES.switch_route()`);
- `t!(locale, "key", name = value, …)`, imported elsewhere with
  `use crate::i18n::t;`. An unknown key, a missing or unexpected argument is
  a compile error. It returns `Cow<'static, str>`, plain text that `html!`
  escapes; a key ending in `_html` is trusted HTML and returns `Markup` with
  its arguments escaped. A plural message takes `count = n` (any integer or a
  `rullst::i18n::Decimal`) and selects the form with the locale's CLDR rule.

Edits to existing files rebuild the crate. To rebuild when a locale file is
added or removed, add a `build.rs` next to `Cargo.toml` (stable Rust gives
a macro no way to watch a directory):

```rust,ignore
// build.rs
fn main() {
    println!("cargo::rerun-if-changed=locales");
}
```

This example is compiled and run in `rullst-core/tests/i18n_catalog.rs`.
