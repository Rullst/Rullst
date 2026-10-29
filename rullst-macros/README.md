# rullst-macros

`rullst-macros` contains the procedural macros re-exported by the Rullst
facade. Application code should normally import them from `rullst`, so the
generated paths and matching runtime features stay aligned: the expansions
call `rullst::…` paths, `#[require_role]` needs the facade's `auth` feature and
`#[derive(Billable)]` its `capital` feature.

**Maturity: Core** ([tiers](https://rullst.github.io/Rullst/book/maturity.html)).
The surfaces listed under
[Experimental compatibility surfaces](#experimental-compatibility-surfaces)
carry their own, weaker promise.

## Stable bounded contracts

| Macro | Contract | Important boundary |
| :--- | :--- | :--- |
| `html!` | Parses an HTML-shaped token tree, escapes dynamic text and attribute values, and rejects mismatched tags at compile time. Element names may be Rust keywords (SVG `<use>`), and a leading `<!DOCTYPE html>` is emitted as written. It returns `rullst::html::Markup`: interpolating a `Markup` (such as another `html!` result) inserts it as HTML, so views compose without wrappers, while `String`, `&str` and numbers are escaped; `Option<Markup>` renders nothing for `None` and an iterator of `Markup` collects into one. Attributes are `name="text"`, `name={value}` (always written, escaped), a bare `name` (`<input required>`) or a conditional `name?={value}`: a `bool` writes the bare name or nothing and an `Option<T>` writes the escaped value or nothing. `@if`/`@else if`/`@else` (with `if let`), `@for pattern in iterable` and `@match value { pattern if guard => { … } }` render nodes conditionally or repeatedly, with Rust's own pattern and exhaustiveness checks; other control flow is a `{…}` expression returning `Markup`. Dynamic URL attributes (`href`, `src`, `action`, `formaction`, `poster`, `data`, …) render `#` instead of a `javascript:`/`vbscript:` URL, or a `data:` URL outside media elements (`img`, `audio`, `video`, `source`, `track`, `image`). | `rullst::html::RawHtml` (and `Markup::from_trusted`) is an explicit trust boundary for HTML produced outside `html!`; never wrap untrusted text in it (its entity references are not decoded before the URL check). Static literals are author-owned source code. Dynamic event-handler values (`on*`, `hx-on`, `hx-on-*`, also `data-` prefixed) are rejected at compile time; static handler strings remain allowed. Dynamic `style` values are only escaped: untrusted CSS can still restyle the page or load remote resources, so `style` is a trusted-only context. |
| `i18n_catalog!` | Re-exported as `rullst::i18n::catalog!`. Reads `<dir>/<tag>.toml` catalogs at compile time and fails on keys missing from a locale, placeholder or plural-shape mismatches, malformed templates and plural forms the locale's rule does not use; defines `LOCALES` and a `t!` macro whose unknown keys and missing, unexpected or repeated arguments are compile errors. | `t!` returns plain text that `html!` escapes; only `*_html` keys are trusted catalog HTML (arguments still escaped). Plural forms follow CLDR's cardinal rules for 19 languages; catalogs are compiled in; a `build.rs` printing `cargo::rerun-if-changed=locales` rebuilds the crate when a locale file is added or removed (edits are tracked by the macro). |
| `#[require_role("Role")]` | Requires an async handler binding named `user`, checks `HasRole` before the body, and returns HTTP 403 on denial. | Authentication, user extraction, role persistence, tenant policy, and ownership checks remain application responsibilities. |
| `#[derive(Billable)]` | Implements the bounded Capital `Billable` facade for a named-field struct containing `email: String`; optional subscription, tier, and paired grace-period fields are recognized. | It does not charge by itself or invent provider/payment/authorization data. |
| `#[server_function]` | Takes a concrete async free function with at most 16 parameters and generates one `RpcResult<T>` function for native and Wasm targets and, on native targets, a matching `<name>_rpc_router()`. Arguments and results use the bounded, versioned `rullst.client` v1 JSON envelope. | Parameters and output must be owned Serde types. The generated router supplies transport, not identity: mount it inside the production security, authentication, tenant, authorization, rate-limit, and application idempotency policies. |
| `#[route]` | Deprecated compatibility marker that preserves an argument-free annotation. | It never registers a route. Use `rullst::routes!`. |

## Experimental compatibility surfaces

- `#[island]` emits the current native/Wasm island wrapper. It is not a
  complete hydration/RPC protocol and should not be treated as a stable ABI.
  Arguments that cannot be serialized to JSON leave `data-props` empty, so the
  server HTML renders without hydration instead of panicking.
- `#[live_component]` and `#[live_event]` generate the bounded process-local
  Live component bridge. Authentication, reconnect, ordering, backpressure,
  and browser interoperability belong to the host contract. Dispatch reads the
  event name only from the first present `rullst_event`, `action` or `event`
  string field (for example `<button name="rullst_event" value="save">`) and
  runs at most one handler per message; other payload keys and values, such as
  form inputs, never select a handler.
- `#[memoize]` uses Rullst's process-local memory cache. Keys combine the
  function's `module_path!()`, name and attribute location (`file!()`,
  `line!()`, `column!()`) with the JSON-serialized arguments, so same-named
  functions in different modules, crates or `impl` blocks never share entries.
  The store is bounded to 4,096 entries and 32 MiB of key plus value bytes;
  an entry larger than 256 KiB is not cached, and the oldest entries are
  evicted first. Entries expire after one hour. Arguments that cannot be
  serialized to JSON (for example a `u128` above `u64::MAX`) run uncached
  instead of panicking.
  It is not tenant-aware, distributed, invalidation-aware, or suitable for
  secrets/authorization decisions.

## Compile-time diagnostics

The UI test suite checks malformed HTML, dynamic event-handler attributes
(also as conditional `name?={…}` attributes), conditional attributes without a
braced expression, unsupported or malformed `@` control flow and a
non-exhaustive `@match`, unknown/duplicate `server_function`
options, unsafe RPC paths, synchronous/generic/method server functions,
borrowed and destructured RPC parameters, invalid return types, missing role
identity bindings, invalid legacy route arguments, invalid `Billable`
shapes, translation catalogs with a missing key, mismatched placeholders or a
missing plural form, and `t!` calls with an unknown key or wrong arguments.
Native facade tests exercise parameter/result transport, correlation,
redacted application failures, media-type/schema/body-limit rejection, and the
production CSRF layer. Both supported Wasm targets compile the client
expansion; browser-engine/network interoperability beyond CI remains external
evidence.

```console
cargo test -p rullst-macros
```

The macros deliberately avoid runtime reflection. Expansion errors should be
reported as `syn::Error` diagnostics instead of panicking the compiler.
