# External audit kit

This page is for a third-party reviewer or penetration tester. It says what is
in scope, where the threat models are, how to build and run a sample
application with the production security baseline, which evidence the tooling
produces, what to test first and how to report findings. Every command below
runs from a checkout of the commit under review.

## Scope

**In scope**

| Area | What to review |
| :--- | :--- |
| `rullst-core` | The runtime baseline in `rullst-core/src/security/`: secure headers and CSP nonce, WAF, double-submit CSRF, exact machine endpoints, trusted-proxy client resolution, tenant context; the `Server` composition in `rullst-core/src/server/stack.rs`; rate limiting (`resilience_rate_limit.rs`); uploads (`uploads.rs`). |
| `rullst-security` | RASP, response DLP, honeypot, CSWSH guard, JSON schema guard, login jail, MFA, RBAC (`RbacGuard`, `UserContext`), audit chain and field encryption. |
| `rullst-auth`, `rullst-connect` | Password hashing, encrypted sessions, session registry, passkeys, OAuth2/OIDC flows. |
| `rullst-orm` | Parameterized queries, `sanitize_identifier`, fail-closed tenant columns. |
| `rullst-capital` | Payment webhook signature, freshness and replay verification. |
| `rullst-nexus`, `rullst-studio` | The Nexus administrator boundary (loopback in debug builds, Basic Auth over verified TLS in release) and that Studio stays a debug-only, loopback-only tool. |
| `rullst-macros` | Output escaping of `html!`. |
| `cargo-rullst` starters | The code `cargo rullst new` generates for the Blank (HTML and `--api`), Blog, Portfolio, SaaS, LMS and ERP blueprints, and the `audit` command. |

Endpoints worth a direct look in the starters: SaaS `/login`, `/register`,
`/logout`, `/dashboard`, `/billing/checkout`, `/billing/portal`,
`/billing/webhook` and `/nexus`; LMS `/courses/{id}/enroll`,
`/lessons/{id}/play` and `/lessons/{id}/progress`; ERP `/products`,
`/products/{id}/add-stock` and `/orders`; Blank JSON API `POST /api/messages`.
In the sample application: `POST /posts`, `/posts/repository`, `/checkout`,
`/security-demo`, `/_live` (WebSocket), `/nexus` and the `/wp-admin` trap.

**Out of scope**

- Live third-party services: payment, mail, LLM and OAuth providers. The
  framework ships deterministic offline mocks for them; test the adapter
  boundary, not the provider.
- Your deployment: reverse proxy, TLS termination, DNS, host and network
  hardening, and volumetric DDoS.
- The deliberate mock credentials and offline fixtures of `examples/`
  (see `examples/blog/README.md`).
- Unpublished v13 candidates (`rullst-privacy`, `rullst-labs`, `rullst-media`,
  `rullst-supervision`) unless the engagement names them.

## Threat models

- [Rullst threat models](threat-models.md): one model per boundary. Start with
  TM-CORE-2 (client identity behind proxies), TM-AUTH-1 (sessions, passwords,
  OAuth and passkeys), TM-NEXUS-1, TM-STUDIO-1, TM-TENANT-1 (multi-tenant data
  access), TM-SEC-1 (HTTP payload contracts), TM-PAY-1 (webhooks and billing)
  and TM-DEPLOY-1 (CLI and release).
- [Security architecture](security-architecture.md): contracts and the
  canonical production middleware order.
- [Which security layer to use](security-layers.md): what each layer does and
  what it does **not** protect against.
- [Session management](session-management.md),
  [private multipart uploads](private-multipart-uploads.md),
  [outgoing webhooks](outgoing-webhooks.md) and
  [guarded local AI tools](ai-tool-security.md) for those features.
- [v12 security claims](v12-security-claims.md) and
  `.github/threat-model-release-minimum.json`, the release gate's minimum
  threat-model evidence.

## Sample application

### Showcase app (`examples/blog`)

The blog showcase mounts CSRF and the header baseline itself and exercises
tenancy, forms, WebSockets, Nexus and the security primitives. Run it with
`RULLST_ENV=staging`, which applies the same baseline as production (WAF,
CSRF, secure cookies), and keep it on loopback:

```bash
cd examples/blog
printf 'RULLST_ENV=staging\nAPP_KEY=%s\n' "$(openssl rand -hex 32)" > .env
HOST=127.0.0.1 cargo run -p rullst-blog-example
```

It listens on `http://127.0.0.1:3000` and creates `blog.db` on first start.
Quick checks from another terminal:

```bash
# Security headers and the CSRF cookie
curl -si http://127.0.0.1:3000/ | grep -iE 'content-security-policy|x-frame-options|set-cookie'
# A write without the CSRF token is refused (403)
curl -si -X POST http://127.0.0.1:3000/posts --data 'title=t&body=b' | head -1
# The WAF refuses an injection probe (403)
curl -si 'http://127.0.0.1:3000/?q=1%27%20union%20select%20pw%20from%20users--' | head -1
```

A debug build keeps Nexus loopback-only. To review the release policy, build
with `--release` and provide unique `NEXUS_ADMIN_USERNAME` and
`NEXUS_ADMIN_PASSWORD` values behind TLS, as the example's README explains.

### Generated starters

Build the CLI from the checkout and generate a starter next to it. Run the
command from the repository root, so the project depends on this checkout's
crates through path dependencies:

```bash
cargo build -p cargo-rullst --bin rullst
./target/debug/rullst new ../audit-saas --default --blueprint saas --database sqlite --skip-initial-migration
cd ../audit-saas
cargo test
```

Use `--blueprint blank`, `blank --api`, `blog`, `portfolio`, `lms` or `erp`
for the other starters. `cargo test` runs the generated `src/security_tests.rs`
offline against the starter's own router behind the staging/production
baseline: security headers, CSRF, WAF, and where the starter has them, the
sign-in rate limit (SaaS, LMS), the owner check of the lesson routes (LMS) and
the bearer token of the JSON API (`blank --api`). Treat these as a floor, not
as your test plan.

## Tooling

Run the evidence report from the application directory. The CLI built above
works in any project:

```bash
cd examples/blog
../../target/debug/rullst audit --report json   # writes SECURITY_REPORT.json
```

In a generated project, run `../Rullst/target/debug/rullst audit --report json`
(adjust the path to your checkout), or `cargo rullst audit --report json` with
an installed CLI. The [security report guide](security-report.md) explains
each check, its OWASP ASVS 5.0.0 Level 1 mapping and the `NOT EVALUATED`
list. The command exits 1 when a check reports `FINDINGS` or `ERROR`.

The SBOM and supply-chain evidence come from the repository root, as the
release workflow (`.github/workflows/release.yml`) produces them:

```bash
cargo run --locked -p cargo-rullst --bin rullst -- audit --idor --compliance --sbom
cargo audit
```

The first command writes `sbom-cyclonedx.json` (CycloneDX 1.5, from
`Cargo.lock`) and `SECURITY_COMPLIANCE.md`; `cargo audit` needs `cargo-audit`
installed. Dependency policy is in `deny.toml`, which CI checks with
cargo-deny (`.github/workflows/cargo-deny.yml`); locally, `cargo deny check`.

## What to test first

1. **Authentication and sessions:** login, registration and logout in the SaaS
   and LMS starters; session cookie attributes and fixation; the credential
   rate limit and its keying behind a proxy (`[security] trusted_proxies`);
   passkeys and OAuth state in `rullst-auth` and `rullst-connect`.
2. **CSRF:** every write route in staging, including multipart bodies (the
   token must precede file fields), HTMX requests and the exact
   `csrf_signed_webhook_paths` and machine-endpoint exemptions.
3. **Tenant isolation:** `TenantContext`, fail-closed `tenant_column` models
   (the showcase's `X-Tenant-ID` selector and `/posts/repository`) and
   Nexus tenant columns.
4. **IDOR:** parameterized routes such as the LMS lesson and enrollment routes
   and ERP stock updates. `cargo rullst audit --idor` fails on a route without
   a declared access class (`// rullst-access: ...`) and a recognized guard;
   verify that the guard actually protects the route.
5. **Webhooks:** signature, freshness and replay checks on `/billing/webhook`
   and in `rullst-capital`; the bearer machine endpoint of the JSON API.
6. **File upload:** body limits, content-type handling, file-name sanitizing
   and the private multipart upload flow.
7. **WAF and RASP bypass:** encodings, payloads split across fields, JSON
   versus form bodies, and false positives on ordinary text. The WAF is a
   coarse baseline; parameterized SQL and output encoding are the defenses.

## Reporting

Report vulnerabilities privately; never in a public issue or pull request:

- GitHub private vulnerability reporting: the **Report a vulnerability**
  button on the repository's
  [Security tab](https://github.com/Rullst/Rullst/security/advisories/new); or
- e-mail to `officialrullst@gmail.com`, as the [security policy](https://github.com/Rullst/Rullst/blob/main/SECURITY.md)
  describes, with the affected crate and version, a proof of concept and the
  estimated impact.

Include the commit hash you reviewed and, when relevant, the
`SECURITY_REPORT.json` and the commands you ran.

## What a clean result means

A run with no findings means that the checks you ran saw nothing they
recognize on that commit and configuration. It does **not** mean:

- that the application is secure, or that a requirement such as an OWASP ASVS
  control is met: `audit --report` never reports `PASS` and lists most Level 1
  requirements as `NOT EVALUATED`;
- that a deployment is safe: proxy, TLS, secrets and host configuration are
  outside the source;
- that authorization is correct: the IDOR scan checks declarations and guards,
  not business rules;
- that a later commit, another feature set or another starter behaves the
  same.

Record the scope, commit, configuration and commands with every result, and
treat each `NOT CHECKED` or `NOT EVALUATED` item as work the review still has
to cover.
