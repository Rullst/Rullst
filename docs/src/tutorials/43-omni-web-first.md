# Rullst Omni: Web-First, Platform-Enhanced Applications

Rullst Omni packages one canonical Rullst web application for desktop, Android
and iOS with Tauri. It is deliberately a secure web-shell foundation, not a
claim that a website automatically becomes a store-ready native product.

The architectural rule is simple:

1. the Rullst server owns domain rules, identity, authorization, persistence,
   realtime policy and security;
2. the web interface remains the universally reachable product;
3. platform shells reproduce that interface and add only narrowly scoped
   native capabilities that have a real product need and platform tests.

This keeps web, desktop and mobile behavior aligned without treating an
untrusted client as the authority.

## Generate a desktop development shell

From a Rullst application root:

```bash
cargo rullst make:omni --platform desktop
cargo rullst omni desktop
```

The product name and version inherit `[package].name` and `[package].version`.
Desktop development derives a `com.example.<package>` identifier when none is
provided. That namespace is a visible placeholder, not a distributable product
identity.

The default `http://localhost:3000` profile starts the parent Rullst server,
waits for it and owns only the child process it created. It refuses to attach
when port 3000 was already occupied, stops if the child exits before readiness
and fails after a bounded timeout. This prevents the shell from silently
connecting to an unrelated local process.

For an externally operated HTTPS application, set its public web URL:

```bash
cargo rullst make:omni \
  --platform desktop \
  --backend-url https://app.example.com \
  --identifier com.exampleowner.myapp \
  --product-name "My App" \
  --app-version 1.2.3
```

Use a reverse-DNS namespace that you or your organization actually control;
the value above is illustrative.

## Generate Android or iOS

Mobile requires both a reachable backend and an application-owned identifier:

```bash
cargo rullst make:omni \
  --platform android \
  --backend-url https://app.example.com \
  --identifier com.acme.myapp

cargo rullst make:omni \
  --platform ios \
  --backend-url https://app.example.com \
  --identifier com.acme.myapp
```

Android emulator development may use `http://10.0.2.2:3000`. Distributable
applications should use HTTPS. Android requires the Android SDK/NDK and Java;
iOS generation requires macOS and Xcode.

```bash
cargo rullst omni android
cargo rullst omni ios
```

The CLI initializes only platforms selected by the user. A requested toolchain
or Tauri initialization failure fails the command instead of printing a false
success.

## Security model

The generated local bootstrap has an origin-specific CSP and no inline script.
Remote content is not given a global Tauri object or privileged command
capability. A Rust-side navigation policy admits only:

- the packaged Tauri bootstrap origin; and
- the exact scheme, host and effective port of `--backend-url`.

Paths and query strings on that same backend remain usable. A lookalike host,
scheme downgrade, different port or third-party origin is rejected.

This secure default means cross-origin OAuth and ordinary external links do not
yet work inside the webview. Do not weaken the allowlist to `https:` or expose a
generic shell command. Add a reviewed system-browser opener plus an allowlisted,
single-use deep-link callback when the application needs that flow.

All normal web protections still apply. The server must enforce sessions,
CSRF, secure headers, ownership/RBAC, input validation and rate limits. A mobile
package does not make server-side authorization optional.

## Share one typed wire contract

`rullst::client_contract` is available to native server code and
`wasm32-unknown-unknown` clients. Its `rullst.client` v1 envelope gives web and
Omni code the same bounded JSON shape without inventing a second business API:

```rust
use rullst::client_contract::{
    ClientContractPolicy, ClientRequest, IdempotencyKey, RequestId,
    CURRENT_CLIENT_CONTRACT_VERSION,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct LessonAttempt {
    lesson_id: String,
    answer: String,
}

let request = ClientRequest::mutation(
    CURRENT_CLIENT_CONTRACT_VERSION,
    RequestId::new("req_01j8lesson")?,
    IdempotencyKey::new("attempt_01j8lesson")?,
    LessonAttempt {
        lesson_id: "lesson_1".into(),
        answer: "bonjour".into(),
    },
);
let encoded = ClientContractPolicy::default().encode_request(&request)?;
```

The server decodes through `ClientContractPolicy`, derives the user and tenant
from its authenticated session, requires the idempotency key before a mutation,
and computes grading, points, streaks and server time itself. Do not put a role,
`user_id`, trusted score or authorization decision in the client payload.

The generic codec has a configurable ceiling no larger than 2 MiB, rejects
unknown outer fields and selects only a mutually supported positive version.
Its key proves request shape, not exactly-once execution: the application still
needs a durable unique key plus an atomic result/effect transaction. This
contract is a prerequisite for future offline queues, not an offline queue.

## Offline behavior

The packaged bootstrap can explain that the device is offline and retry before
the first navigation. It still does **not** automatically cache application
data or mount background synchronization.

The opt-in native `offline-sync` feature now supplies a bounded foundation:
account-bound AES-256-GCM snapshots, FIFO idempotent mutations, authoritative
server revisions/cursors, explicit conflicts, full resync, quotas, recovery and
logical erasure. Follow the
[offline synchronization tutorial](44-omni-offline-sync.md) to use it without
moving authority into the client.

Calling the generated shell itself “offline-first” remains inaccurate. A real
application profile must still provide at least:

- reviewed platform persistence and Keychain/Keystore integration;
- network/background orchestration and application conflict UX;
- concrete migrations beyond the current versioned fail-closed schema;
- complete deletion of snapshots, backups and platform keys;
- browser, Android and iOS tests for airplane mode and reconnection.

## Adding native capabilities safely

Push notifications, biometrics, deep links, camera/file access, haptics and OS
secure storage can make Omni feel native. Add them as opt-in capabilities, one
at a time:

1. state the user-facing need and supported platforms;
2. grant the narrowest Tauri/platform permission;
3. keep device credentials in Keychain/Keystore-class storage, never in web
   local storage or generated source;
4. authenticate every server operation independently of the client signal;
5. add negative tests for denied/replayed/cross-account requests;
6. test a real device before documenting the capability as supported.

Biometrics may unlock a local credential; it must not manufacture server
authorization. Push payloads should be minimized and treated as untrusted input.

## What CI proves

Rullst maintains path-aware generation/compile workflows for three evidence
classes:

- desktop crate checks on Linux, macOS and Windows;
- an Android debug APK build;
- an iOS simulator build on macOS.

A green run proves that a fresh generated shell compiled for that runner and
commit. All three gates passed on
`755fbd61933bed04369e0eb5de50b11275db5e3d`. This does not prove
physical-device behavior, accessibility, signing,
privacy declarations, TestFlight/Play testing or store acceptance.

## Before distribution

Review the generated `omni-app/README.md`, then complete application-owned work:

- production identity, icons, versioning and metadata;
- HTTPS endpoint, authentication and retention policy;
- accessibility and poor/offline-network behavior;
- platform privacy manifests and usage descriptions;
- signing/provisioning and secret handling;
- physical-device and beta-channel tests;
- store policy, screenshots, disclosure and review.

Tauri supplies legitimate packages and installers; stores decide whether the
finished product meets their technical, functionality, privacy and content
requirements.
