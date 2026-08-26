# Examples and generated starters

The repository examples and CLI blueprints serve different purposes:

| Artifact | Purpose | Trust boundary |
| --- | --- | --- |
| `examples/blog` | Workspace integration showcase with local data, interactive demos, and offline provider fixtures. | Development-only; not a production template or compliance proof. |
| CLI blueprints | Small starting structures generated into a new project. | Generated output must be reviewed, configured, formatted, checked, and tested by the application owner. |

No example is expected to exercise 100% of workspace behavior. External provider
paths use deterministic mock credentials so CI and local development do not make
live purchases, send email, call cloud LLMs, or issue fiscal documents.

## Blog showcase

The blog package demonstrates:

- server-rendered HTML and Active Record persistence;
- a parameterized repository query;
- LiveView/WebSocket and Wasm-island presentation examples;
- Pico CSS and Tera presentation paths;
- `Billable` quota evaluation and payment-adapter mock fixtures;
- an escaped, unsigned DPS XML preview that is explicitly not an NFS-e
  authorization;
- bounded security-helper demonstrations and a local AI/vector fixture;
- a debug-only standalone Studio and Nexus access that is loopback-only in
  debug builds and credential-protected in release builds.

The complete, current route list is maintained in
`examples/blog/README.md` alongside its configuration requirements.

## Tenant selection

The example inserts a static test-only `TenantMembership` before the tenant layer.
The `X-Tenant-ID` header can select only one of those fixed memberships. This
models the separation between an untrusted selector and trusted authenticated
claims.

In a real application, authentication middleware must derive membership from a
verified session or token. Never construct membership from the same client header
used to select a tenant.

## Fiscal and provider fixtures

The `/pricing` page uses `mock_*` credentials and performs no live checkout. The
DPS snippet is not XMLDSig-signed, transmitted, homologated, or authorized.
`Homologation` and `Production` NFS-e modes remain fail-closed.

Mock URLs and sample provider metadata are test fixtures, not a promise of live
capability, pricing, tax treatment, or regional availability.

## Running locally

```bash
touch examples/blog/blog.db
cargo run -p rullst-blog-example
```

Before local startup, configure `APP_KEY` and `DATABASE_URL` as documented in the
example README. The debug build needs no Nexus password, but verifies the socket
peer as loopback. A release build does not start Studio and refuses to construct
Nexus without validated `NEXUS_ADMIN_USERNAME` and `NEXUS_ADMIN_PASSWORD`
values. Studio is local developer tooling; keep it on a trusted interface.

## Verifying examples and blueprints

For the checked-in example, run package tests and the workspace trifecta. For CLI
output, use a temporary directory and verify every generated project:

```bash
cargo fmt --all -- --check
cargo check --all-features
cargo test --all-features
```

CI smoke or DAST workflows cover only the routes and assertions present in those
files. Their logs, commit digest, toolchain, and skipped steps are the evidence;
the example itself is not evidence of production readiness.
