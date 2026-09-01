# rullst-macros

`rullst-macros` contains the procedural macros re-exported by the Rullst
facade. Application code should normally import them from `rullst`, so the
generated paths and matching runtime features stay aligned.

## Stable bounded contracts

| Macro | Contract | Important boundary |
| :--- | :--- | :--- |
| `html!` | Parses an HTML-shaped token tree, escapes dynamic text and attribute values, and rejects mismatched tags at compile time. | `rullst::html::RawHtml` is an explicit trust boundary; never wrap untrusted text in it. Static literals are author-owned source code. |
| `#[require_role("Role")]` | Requires an async handler binding named `user`, checks `HasRole` before the body, and returns HTTP 403 on denial. | Authentication, user extraction, role persistence, tenant policy, and ownership checks remain application responsibilities. |
| `#[derive(Billable)]` | Implements the bounded Capital `Billable` facade for a named-field struct containing `email: String`; optional subscription, tier, and paired grace-period fields are recognized. | It does not charge by itself or invent provider/payment/authorization data. |
| `#[server_function]` | Preserves the complete native async free-function signature. | Browser argument transport and server-side RPC registration are not implemented by this macro; the Wasm bridge is experimental. |
| `#[route]` | Deprecated compatibility marker that preserves an argument-free annotation. | It never registers a route. Use `rullst::routes!`. |

## Experimental compatibility surfaces

- `#[island]` emits the current native/Wasm island wrapper. It is not a
  complete hydration/RPC protocol and should not be treated as a stable ABI.
- `#[live_component]` and `#[live_event]` generate the bounded process-local
  Live component bridge. Authentication, reconnect, ordering, backpressure,
  and browser interoperability belong to the host contract.
- `#[memoize]` uses Rullst's process-local memory cache. It is not tenant-aware,
  distributed, invalidation-aware, or suitable for secrets/authorization
  decisions.

## Compile-time diagnostics

The UI test suite checks malformed HTML, unsupported `server_function`
arguments, synchronous server functions, methods with receivers, missing role
identity bindings, invalid legacy route arguments, and invalid `Billable`
shapes. These tests prove diagnostic boundaries only; runtime behavior is
tested through the facade and owning runtime crates.

```console
cargo test -p rullst-macros
```

The macros deliberately avoid runtime reflection. Expansion errors should be
reported as `syn::Error` diagnostics instead of panicking the compiler.
