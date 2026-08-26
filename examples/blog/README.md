# Rullst blog showcase

This application is a development showcase and integration fixture for the
workspace. It intentionally uses local data and deterministic mock credentials
for external services. It is not a production starter, compliance demonstration,
or proof that every crate and feature is exercised.

## Demonstrated paths

- `/`: server-rendered posts using the `html!` macro and ORM.
- `/live-feed` and `/_live`: server-driven WebSocket example.
- `/editor`: Wasm island mounting example.
- `/pico-demo` and `/templates-demo`: Pico CSS and Tera presentations.
- `/posts/repository`: parameterized repository queries.
- `/pricing`: `Billable` quotas, payment-adapter mock fixtures, and an unsigned,
  offline DPS XML preview. It never issues or signs an NFS-e.
- `/security-demo`: bounded security-helper demonstrations.
- `/ai-assistant`: local deterministic vector-search and guardrail example.
- `http://127.0.0.1:5555`: debug-only local Studio server.
- `/nexus`: one-click, loopback-only admin access in debug builds; validated
  Basic Auth credentials are mandatory in release builds.

The provider catalogue is descriptive. Adapter capabilities differ, fees and
provider terms can change, and this example makes no live provider request.

## Local setup

From the workspace root, create `examples/blog/.env` with an application key
and database URL:

```dotenv
APP_ENV=development
APP_KEY=replace-with-at-least-32-random-bytes
DATABASE_URL=sqlite://blog.db
```

Then run:

```bash
touch examples/blog/blog.db
cargo run -p rullst-blog-example
```

Open `http://127.0.0.1:3000`, then use the Studio and Nexus buttons. Studio is
served on `http://127.0.0.1:5555`; Nexus accepts only a verified loopback peer in
this debug build.

A release build does not start Studio. It also replaces the local Nexus policy
with `NexusAuthPolicy::basic_from_env()`, so production startup requires unique
`NEXUS_ADMIN_USERNAME` and `NEXUS_ADMIN_PASSWORD` values plus a verified TLS
boundary. Never expose Studio or a credential-free Nexus policy on a public
interface.

## Tenant fixture

The showcase inserts a fixed `TenantMembership` extension containing only
`community`, `tenant-enterprise`, and `tenant-startup`. `X-Tenant-ID` is therefore
only a selector within that trusted demo membership; an arbitrary tenant is
rejected.

```bash
curl -s -H "X-Tenant-ID: tenant-enterprise" http://127.0.0.1:3000/
curl -s -H "X-Tenant-ID: tenant-startup" http://127.0.0.1:3000/
curl -s http://127.0.0.1:3000/
```

Production applications must derive `TenantMembership` from authenticated claims
or a cryptographically trusted internal gateway, never directly from a client
header.

## Security boundary

The showcase loads third-party development assets and consequently uses a relaxed
demo CSP for those pages. It is not the production header baseline. A deployed
application should self-host or explicitly trust assets, use per-response nonces,
and test the final CSP.

The `/wp-admin` handler is a visible demo route. Real honeypot and ban behavior is
provided by the security middleware with trusted-peer identity and TTL limits.

## Verification

The workspace test suite compiles this package. Dedicated E2E and DAST workflows
exercise a bounded set of HTTP behavior; consult their exact commands and logs
instead of treating the existence of a workflow as a security guarantee.
