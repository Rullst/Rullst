# 43. Installable Apps: PWA, Google Play and Microsoft Store

A Rullst application is already a web application. `cargo rullst make:pwa`
makes it installable: browsers on Android, ChromeOS, Windows, macOS and Linux
offer to install it, iOS users add it to the home screen from Safari, and the
same site can be published in Google Play and the Microsoft Store. There is one
codebase, and the server stays the authority for every rule and every piece
of data.

`make:pwa` replaces the Omni shells (`make:omni`), which 13.0 removes; see the
[migration guide](../migration-v13.md).

## Make the application installable

Run the generator at the root of an application created from an HTML starter
(Blank, SaaS, LMS, Blog, Portfolio or ERP):

```bash
cargo rullst make:pwa --name "Field Notes" --short-name Notes
cargo rullst dev
```

Every flag is optional; the [CLI reference](../cli_reference.md#cargo-rullst-makepwa-1300-alpha3)
lists them with their defaults. The command adds these files:

| File | Purpose |
| :--- | :--- |
| `static/manifest.webmanifest` | Name, colours, `start_url`, `scope`, `display: standalone`, icons. |
| `static/icons/*.png` | Placeholder icons: 192 and 512 pixels, a maskable 512 and the 180-pixel apple-touch icon. |
| `static/pwa.js` | Registers the service worker and shows the update prompt. |
| `src/pwa.rs` | Serves `/sw.js` and `/offline`; `head()` returns the tags a layout needs. |
| `src/pwa/sw.js` | The service worker. |
| `PWA.md` | The checklist for the generated project. |

It also edits the application in three recognized places: the router mounts
`crate::pwa::router()` next to the health probes, every page layout gains the
PWA tags, and the crate root declares `mod pwa`. The whole plan is checked
before the first write. Running the command again changes nothing, and a
generated file you have edited stops the run instead of being overwritten.
The JSON API starter has no pages to install and is refused.

Open `http://localhost:3000`: browsers allow service workers on `localhost`
and on HTTPS origins only. Chrome's DevTools **Application** panel shows the
manifest, the worker and the cache.

Before release, replace the four icons with your own PNG files of the same sizes.
Keep the subject of `maskable-512.png` inside the central circle of 80% of
its width: launchers crop that icon to their own shape.

## A layout that make:pwa does not recognize

The generator recognizes an `html!` document, a document written in a string
and Core's `render_page_with_lang`. For any other layout, add the tags
yourself, inside `<head>`:

```rust,ignore
pub fn layout(title: &str, body: Markup) -> Markup {
    rullst::html! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <title>{title}</title>
                {crate::pwa::head()}
            </head>
            <body>{body}</body>
        </html>
    }
}
```

With Core's layout, `render_page_with_head` places the same tags at the end
of the page's `<head>`:

```rust,ignore
render_page_with_head(&htmx, "en", "Dashboard", crate::pwa::head(), content)
```

And mount the routes once, outside any CSRF layer (they answer GET only):

```rust,ignore
router.merge_axum(crate::pwa::router().into_axum())
```

## How it fits the security defaults

**Content-Security-Policy.** The default policy is
`default-src 'self'; script-src 'self' 'nonce-…'; … worker-src 'self' blob:`.
`static/pwa.js` is a same-origin file, so `script-src 'self'` loads it without
a nonce, and it styles the update prompt through the DOM rather than a `style`
attribute, which `style-src` refuses. `/sw.js` is allowed by `worker-src
'self'`, and the manifest, which has no `manifest-src` directive, falls back to
`default-src 'self'`. A policy with `'strict-dynamic'` ignores `'self'`; give
the registration script the request's nonce in that case.

**Cache-Control.** Rullst answers `Cache-Control: no-store` when a response
sets no policy of its own, and the worker never stores such a response, nor a
`private` one, a non-GET request or a response from another origin. Personal
and authenticated pages therefore never stay on the device. Static files are
served with `no-cache` (content-hashed names with a year-long `immutable`
policy), `/offline` and `/sw.js` with `no-cache`. A page that is the same for
every visitor, such as a landing page, can opt into storage with
[`rullst::cache_control`](10-static-assets-and-compression.md#downloads-and-feeds); the worker then keeps a copy for offline
visits.

## What the worker does

- **Install:** stores `/offline`, the manifest, `pwa.js`, the 192 and 512
  icons and the starter's `rullst.css`, htmx and logo when they exist. The
  install fails if `/offline` cannot be stored.
- **Navigations:** network first. A storable page is kept; without network
  the worker answers with the stored copy or the offline page.
- **Static files:** a name that carries a content hash
  (`app.3f9a2c1b.css`, the rule Rullst's static server uses for its
  year-long cache) is served from the cache first; other files go to the
  network first and fall back to the stored copy.
- **Everything else** (forms, htmx fragments, JSON APIs, other origins) goes
  to the network untouched.
- **Updates:** the cache is named after `[package].version`, which `src/pwa.rs`
  writes into the worker it serves. Release a new version and browsers install
  the new worker next to the old one; the page shows "A new version of this
  app is available" with a **Reload** button, which sends `SKIP_WAITING` to
  the waiting worker and reloads once it takes control. The old cache is then
  deleted. Without a version change, users keep the files stored at install.

## Data while offline

The worker never queues writes. When a page must accept changes without a
connection, keep the rule the server already applies to every client: the
server is the authority, and a queued change is only a proposal. Store each
pending write in IndexedDB with an idempotency key and send it again when the
connection returns:

```js
// static/outbox.js: load it like pwa.js, as a same-origin script.
const STORE = 'writes';

function database() {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open('app-outbox', 1);
    request.onupgradeneeded = () => request.result.createObjectStore(STORE, { keyPath: 'key' });
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
}

function done(transaction) {
  return new Promise((resolve, reject) => {
    transaction.oncomplete = resolve;
    transaction.onerror = () => reject(transaction.error);
  });
}

// Queue a write; the key travels with every attempt to send it.
export async function queue(url, body) {
  const db = await database();
  const write = { key: crypto.randomUUID(), url, body, queuedAt: Date.now() };
  const transaction = db.transaction(STORE, 'readwrite');
  transaction.objectStore(STORE).put(write);
  await done(transaction);
  return write.key;
}

// Send the queued writes in order; stop at the first network failure.
export async function flush(csrfToken) {
  const db = await database();
  const read = db.transaction(STORE).objectStore(STORE).getAll();
  const writes = await new Promise((resolve, reject) => {
    read.onsuccess = () => resolve(read.result);
    read.onerror = () => reject(read.error);
  });
  writes.sort((a, b) => a.queuedAt - b.queuedAt);
  for (const write of writes) {
    let response;
    try {
      response = await fetch(write.url, {
        method: 'POST',
        credentials: 'same-origin',
        headers: {
          'Content-Type': 'application/json',
          'Idempotency-Key': write.key,
          'X-CSRF-Token': csrfToken,
        },
        body: JSON.stringify(write.body),
      });
    } catch (error) {
      return; // Still offline: try again on the next `online` event.
    }
    // Accepted, already applied or refused by the server: it is settled.
    if (response.ok || response.status === 409 || response.status === 422) {
      const transaction = db.transaction(STORE, 'readwrite');
      transaction.objectStore(STORE).delete(write.key);
      await done(transaction);
    } else {
      return; // Sign-in expired or the server failed: keep it and stop.
    }
  }
}
```

Call `flush` with the page's CSRF token (the `_token` value its forms carry)
from an `online` listener and when the page loads. The outbox holds what the
user typed on this device until it is sent: queue only what the page needs and
clear the store when the user signs out.

On the server, accept each key once. Validate it, store it with the result in
the same transaction as the write (a unique index on the key), answer a
repeated key with the stored result, and check permissions, ownership and
revisions again on every replay. `IdempotencyKey` validates the header before
any work:

```rust
use rullst::client_contract::IdempotencyKey;
use rullst::http::HeaderMap;

/// The replay key of a queued write, or `None` when it is missing or malformed.
fn replay_key(headers: &HeaderMap) -> Option<IdempotencyKey> {
    let value = headers.get("idempotency-key")?.to_str().ok()?;
    IdempotencyKey::new(value).ok()
}

let mut headers = HeaderMap::new();
headers.insert(
    "idempotency-key",
    "3f6c2a9e-8b1d-4c57-9a0e-2d4f6b8c1e7a".parse().unwrap(),
);
assert!(replay_key(&headers).is_some());
headers.insert("idempotency-key", "short".parse().unwrap());
assert!(replay_key(&headers).is_none());
```

Never let a queued write carry a role, a tenant, a price, a score or a
timestamp the server trusts: the server derives them from the signed-in
session and its own clock.
