# Examples and generated starters

The repository examples and CLI starters serve different purposes:

| Artifact | Purpose | Trust boundary |
| --- | --- | --- |
| `examples/blog` | Workspace integration showcase with local data, interactive demos, and offline provider fixtures. | Development-only; not a production template or compliance proof. |
| `examples/saas` | Reproducible CLI-generated SQLite SaaS with authenticated notes, tenant membership and the same HTTP journey used in CI. | Disposable Linux development example; membership is an explicit local operator action. |
| `examples/webgpu` | Browser wave-interference lesson served by `rullst-core`, with WebGPU rendering and a Canvas fallback. | Public read-only demo on loopback; browser output establishes no server-owned result. |
| `examples/iot-gateway` | Arduino Uno boards streaming through per-board serial gateways and per-device tokens to `rullst::iot_ingest`, with an ORM device registry, Nexus and a live SSE dashboard. | Example to copy from; the registry table is application code, and the hardware path ran on two Unos, not a fleet. |
| `examples/lms` | The complete application that was the LMS starter of `cargo rullst new` until 13.0.0-alpha.4, kept as a workspace member with its own tests. | An example to copy from, not a supported starter: no generator or upgrade path behind it. |
| CLI starters (Blank, API, SaaS) | Small starting structures generated into a new project. | Generated output must be reviewed, configured, formatted, checked, and tested by the application owner. |

No example is expected to exercise 100% of workspace behavior. External provider
paths use deterministic mock credentials so CI and local development do not make
live purchases, send email, or call cloud LLMs.

## Blog showcase

The blog package demonstrates:

- server-rendered HTML and Active Record persistence;
- a parameterized repository query;
- Pico CSS and a small embedded file-template fixture (`include_str!` plus
  fixed placeholder replacement; not Tera or another template engine);
- `Billable` quota evaluation and offline Stripe/InfinitePay adapter fixtures;
- bounded security-helper demonstrations and a local AI/vector fixture;
- pages that render under the unrelaxed production Content Security Policy,
  with same-origin stylesheets, scripts and vendored HTMX/Pico.css files;
- a debug-only standalone Studio and Nexus access that is loopback-only in
  debug builds and credential-protected in release builds.

The complete, current route list is maintained in
`examples/blog/README.md` alongside its configuration requirements.

## Small SaaS example

The [SaaS example](https://github.com/Rullst/Rullst/tree/main/examples/saas)
installs the source CLI, generates the starter and adds a small notes API using
database-owned membership, parameterized SQL and owner authorization. Its
interactive mode opens a disposable loopback application:

```sh
python3 examples/saas/run.py --serve
```

Follow `examples/saas/README.md` to register a user, grant local membership and
call the notes API. Without `--serve`, the same command runs its automated
authentication, isolation, CSRF, persistence/restart and revocation journey.
The script requires Linux, Python 3.11+, the pinned toolchain and 32 GiB of
initial disk headroom. It removes generated data on exit and uses no real
provider account. This directory holds the canonical example recipe and
resource; it is not a duplicate checked-in copy of a generated application.

## LMS example

Until 13.0.0-alpha.4, `cargo rullst new --blueprint lms` generated this
application; the CLI now offers only the Blank, API and SaaS starters, and
`--blueprint lms` or `blog` fails before writing anything with a report that
names the example. It was generated with that CLI (SQLite, `--default`), then
adapted to the workspace: `publish = false`, workspace dependencies, a library
with `router()` plus a thin binary, and the generated tests (security baseline,
CSRF, rate limits, ownership checks) as the package's own tests.

| Example | What it shows |
| :--- | :--- |
| [`examples/lms`](https://github.com/Rullst/Rullst/tree/main/examples/lms) | Course catalog, modules and lessons, an accessible player, enrollment, owner-bound progress, sign-in with password reset and a Nexus admin for signed-in administrators. Its owner-boundary test is the threat-model evidence for `ACADEMY-02`. |

The ERP and Portfolio examples (the former `--blueprint erp` and `portfolio`
starters) were removed in 13.0.0-alpha.5: both showed CRUD with a Nexus back
office, which the blog example and the SaaS starter already cover.
`--blueprint erp` or `portfolio` fails before writing anything with a report
that points to `examples/blog`; their last version is in the `v13.0.0-alpha.4`
tag of the repository.

Run it from its directory, so the server finds `.env`, `Rullst.toml` and its
static files:

```bash
cd examples/lms
cp .env.example .env   # then set APP_KEY to 32 random characters, e.g. `openssl rand -hex 16`
cargo run -p rullst-lms-example -- db:migrate
cargo run -p rullst-lms-example
```

To start your own application from it, copy the directory and replace its
`workspace = true` dependencies with the published crates (each README shows
the line). Applications generated earlier from the LMS, Blog, Portfolio or ERP
blueprints keep working: their code is theirs.

## IoT gateway example

[`examples/iot-gateway`](https://github.com/Rullst/Rullst/tree/main/examples/iot-gateway)
turns Arduino Uno boards into authenticated telemetry sources. Each board runs
the sketch in `sketch/` with its own device id; one gateway process per board
reads its serial port and posts the readings with that board's token to the
server's `rullst::iot_ingest::TelemetryReceiver`. The server keeps the device
registry in its database (`iot_devices`, an ORM model with a migration and a
Nexus page whose token digest is hidden), refuses revoked tokens, and pushes
a dashboard with every device over Server-Sent Events. Its README has the
flashing, registration and run commands; its tests run two devices through
the production security baseline without hardware.

## Browser graphics example

The [WebGPU example](https://github.com/Rullst/Rullst/tree/main/examples/webgpu)
serves embedded HTML, CSS and JavaScript modules behind Rullst's production
CSRF/WAF/secure-header baseline. From the repository root:

```sh
cargo run --locked -p rullst-webgpu-example
```

Then open `http://127.0.0.1:3007/webgpu/`. All computation stays in the browser;
missing WebGPU support selects the Canvas fallback. Follow
`examples/webgpu/README.md` for the reusable modules and browser tests.

## Blog tenant selection

The blog example inserts a static test-only `TenantMembership` before the tenant layer.
The `X-Tenant-ID` header can select only one of those fixed memberships. This
models the separation between an untrusted selector and trusted authenticated
claims. Handlers read the membership-checked `TenantContext` that `TenantLayer`
inserts. The `Post` model declares `tenant_column`, so its generated queries fail
closed outside `with_tenant(...)`; the hand-written repository SQL binds the same
tenant itself.

In a real application, authentication middleware must derive membership from a
verified session or token. Never construct membership from the same client header
used to select a tenant.

## Provider fixtures

The `/pricing` page uses `mock_*` credentials and performs no live checkout. It
lists only the built-in Stripe and InfinitePay adapters.

Mock URLs and sample provider metadata are test fixtures, not a promise of live
capability, pricing, tax treatment, or regional availability.

## Running locally

```bash
cd examples/blog
cargo run -p rullst-blog-example
```

Before local startup, configure `APP_KEY` in `examples/blog/.env` as documented
in the example README. The showcase selects its database like `Server` and the
Artisan commands (`DATABASE_URL`, `.env`, then `Rullst.toml`), so `DATABASE_URL`
is optional locally and honored in deployments. The debug build needs no Nexus password, but verifies the socket
peer as loopback. A release build does not start Studio and refuses to construct
Nexus without validated `NEXUS_ADMIN_USERNAME` and `NEXUS_ADMIN_PASSWORD`
values. Studio is local developer tooling; keep it on a trusted interface.

## Verifying examples and starters

For the checked-in examples, run package tests and the workspace trifecta. For CLI
output, use a temporary directory and verify every generated project:

```bash
cargo fmt --all -- --check
cargo check --all-features
cargo test --all-features
```

CI smoke or DAST workflows cover only the routes and assertions present in those
files. Their logs, commit digest, toolchain, and skipped steps are the evidence;
the example itself is not evidence of production readiness.
