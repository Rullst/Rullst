# Tutorial 06: HTMX-oriented server rendering 🎨

Rullst's default scaffold renders HTML on the server and can use HTMX attributes
for targeted requests and fragment swaps. It does not require a project-local
SPA bundle, but HTMX itself is browser JavaScript. The CLI's HTML starters write
a pinned copy to `static/htmx-1.9.12.min.js`, which `rullst::htmx::render_page`
loads from the same origin; an application without those generated assets must
serve that file itself. The application's CSP must permit it.

---

## 🛠️ Step 1: Render HTMX Components

In your controller or view:

```rust
use axum::response::Html;
use rullst::html;
use rullst::html::RawHtml;

pub async fn search_users() -> Html<String> {
    let results = vec!["Alice", "Bob", "Charlie"];
    let rows = results
        .into_iter()
        .map(|name| html! {
            <li class="py-2 text-slate-200">{name}</li>
        })
        .collect::<String>();

    Html(html! {
        <ul id="user-list" class="divide-y divide-slate-700">
            {RawHtml(rows)}
        </ul>
    })
}
```

`RawHtml` is appropriate here only because `rows` is composed exclusively from
already-escaped `html!` fragments. Do not wrap untrusted request data directly
in `RawHtml`.

---

## 💻 Step 2: Wire HTMX Attributes in Front-End HTML

```html
<div class="max-w-md mx-auto p-6 bg-slate-800 rounded-xl shadow-md">
    <input 
        type="text" 
        name="query" 
        placeholder="Search users..." 
        class="w-full px-4 py-2 bg-slate-900 text-white rounded border border-slate-700 focus:outline-none"
        hx-get="/api/users/search"
        hx-trigger="keyup changed delay:300ms" 
        hx-target="#user-list" 
        hx-swap="outerHTML" 
    />
    
    <div id="user-list" class="mt-4 text-slate-400">
        Start typing to search...
    </div>
</div>
```

Mount the example search handler as a **GET** route: this example only reads a
list. GET is not a workaround for CSRF. Commands that change data, send messages
or incur provider charges must not be converted to GET to avoid validation.
For POST forms, including an AI chat, follow the
[CSRF form contract](07-forms-and-validation.md#step-3-send-the-csrf-token-with-browser-forms).
HTMX does not automatically copy Rullst's CSRF cookie into a header or make
HTTP error bodies visible in the target element.

---

## 🧪 Step 3: Snapshot-test rendered HTML

`rullst::testing::assert_html_snapshot!` (v13) compares rendered HTML with
`tests/snapshots/<name>.html` in the crate that calls it, in unit and
integration tests alike. Both sides are normalised first: whitespace-only text
between tags is dropped, every tag that follows another tag starts a new line,
other whitespace runs collapse to one space, and quoted attribute values and
the content of `pre`, `textarea`, `script` and `style` stay as written.
`SnapshotOptions` can also replace per-request values with documented
placeholders: `mask_nonce()` writes `{NONCE}` for `nonce="…"` attributes and
`'nonce-…'` CSP sources, and `mask_csrf_token()` writes `{CSRF_TOKEN}` for a
`name="_token"` field, `<meta name="csrf-token">` and `X-CSRF-Token` values.

```rust,no_run
use rullst::html;
use rullst::testing::{SnapshotOptions, assert_html_snapshot};

fn search_form(nonce: &str, csrf_token: &str) -> String {
    html! {
        <form method="post" action="/search">
            <input type="hidden" name="_token" value={csrf_token} />
            <input type="search" name="q" />
            <script nonce={nonce} src="/static/search.js"></script>
        </form>
    }
}

// In a #[test] function:
assert_html_snapshot!(
    "search/form",
    search_form("per-response-nonce", "per-request-token"),
    SnapshotOptions::new().mask_nonce().mask_csrf_token()
);
```

A missing snapshot fails and names the file to create; a changed one fails
with a compact line diff (`-` snapshot, `+` rendered). Run the tests with
`RULLST_UPDATE_SNAPSHOTS=1` to write missing snapshots and overwrite changed
ones, then review the files and commit them. Snapshot names use ASCII letters,
digits, `_`, `-` and `.`, with `/` between folders; a name that would leave
`tests/snapshots` is refused. The helper is test code: it panics like
`assert_eq!` and has no production use.

---

## 💡 Key Takeaways
- **Small application-owned client surface:** business logic can remain on the
  server while HTMX coordinates browser requests.
- **Partial rendering:** handlers can return fragments instead of full pages.
  Measure page weight and latency for the actual application; no fixed size or
  load-time guarantee follows from the rendering style.
