# Nexus application integration plan

Status: proposed scope, reviewed on 25 September 2026. These are planned
improvements, not APIs available in a published release. The current contract
remains in the [specification](spec.md#102-rullst-nexus-nexus) and
[Nexus guide](4-rullst-nexus.md).

The goal is to embed Nexus into an existing application without duplicating
its login, tenant policy or administration shell. Keep this work in
`rullst-nexus`; a new crate, a universal admin platform and automatic domain
authorization are outside this scope. Applications continue to own membership,
parental relationships, consent, business transactions and deployment policy.

This refines M11, with dependencies on M9/M12/M14 and a separate M32 macro
ergonomics item. It does not add milestones to the 41-theme inventory in
[v13 priorities](v13-priorities.md), or override the
[maintenance scope](v13-maintenance-scope.md).

## Existing foundation and remaining gaps

Preserve the existing authenticated policies, administrator role checks,
parameterized CRUD, bounded inputs, server-side field restrictions, opt-in
tenant scoping and transactional mutation audit. Their limits remain explicit:
host membership and domain authorization are not inferred from registered
models, and the mutation audit does not provide an immutable external log.

`FieldMeta.hidden` already excludes a field from list and form rendering, and
the input validator rejects writes to hidden fields. Its field documentation
incorrectly describes form visibility and should be corrected. It is not a
promise that the database column is never fetched: the edit view currently
loads the full row. Field visibility, database projection and authorization
need distinct documented contracts.

Existing mobile/navigation improvements and pagination links are useful;
they do not establish an end-to-end administration journey without JavaScript.
Likewise, built-in Basic/loopback policies do not provide a public integration
contract for an application's existing sessions.

## Delivery order

| Stage | Bounded result | Required behavior |
| :--- | :--- | :--- |
| 1 — identity and operations | An opt-in host-authenticated, read-only integration mode | Validate the current session and administrative permission separately on each request, before data access or rendering. Nexus creates the principal after successful validation. Reject direct mutation requests as well as hiding unavailable controls. |
| 1 — self-contained shell | Versioned local assets and a shell compatible with strict CSP | No required external scripts/fonts/CDNs, inline handlers or `unsafe-inline`. Test actual browser behavior and outgoing asset requests. Keep application security middleware active. |
| 2 — paths and language | A validated mount path and explicit translation catalog | Use the configured path in every link, form, redirect and endpoint; encode path segments and query values correctly. Support explicit locale selection and deterministic fallback, beginning with en/pt-BR/es. |
| 2 — usable SSR | Keyboard and no-JavaScript read journeys | Search uses GET forms, pagination uses links, and labels, focus, skip navigation and table captions remain usable at narrow widths. HTMX enhances an already working read journey. |
| 3 — data ownership | An explicit per-instance pool or bounded typed collection provider | Keep independent instances isolated. Support application-owned projections without requiring registration of sensitive base tables, arbitrary SQL input or a second ORM. |
| 3 — field and domain policy | Positive read/write projections and operation capabilities | Define never-read secrets, masked values and write-only updates separately. Recheck operation and tenant authorization on the server. Sensitive mutations invoke application domain services and preserve transactional audit requirements. |
| Throughout — optional modules | Explicit configuration of chat, security and telemetry surfaces | In the new integration mode, unconfigured modules have neither mounted routes nor menu entries. Preserve the existing supported mode until an explicit migration is designed. |

Stages describe ordering, not permission to ship an incomplete security
boundary. Stage 1 may use an application-owned read renderer/provider, with a
documented trusted-code boundary and mandatory escaping of interpolated data.
It must not silently activate generic writes or trust client-supplied identity,
roles or tenant IDs. Exact public type and method names require design review;
an application's private extension is evidence of demand, not a stable API.

Prefer generic/static-dispatch integration contracts and typed failures. Keep
the existing Basic credential validation, transport requirements and verified
development-loopback restrictions. A public unrestricted principal constructor
is not the session integration contract. Revocation must take effect on the
next request, including HTMX fragments and direct resource URLs.

Before adding sensitive write operations, define domain-service delegation,
CSRF handling, authorization, transaction boundaries and any required
reauthentication. Do not let generic CRUD bypass consent, family relationships
or account-deletion workflows. Additional read-access auditing and assurance
levels remain separately scoped work; the first read-only mode does not claim
to provide them.

## Stable v12 maintenance boundary

Correct inaccurate field documentation and add a tested recipe for conditional
HTML boolean attributes in the stable documentation. Preserve current APIs,
MSRV and serialized contracts. Narrow defect fixes follow stable maintenance
and the [security policy](../../SECURITY.md); new public integration features
require a versioning decision and are not automatically patch backports.

For HTML boolean attributes, presence means true: `selected="false"` does not
disable selection. The current macro escapes and serializes a dynamic value;
the safe existing recipe emits the complete selected or unselected element,
including a quoted `selected="selected"` only in the selected branch. See
[MDN's boolean attribute semantics](https://developer.mozilla.org/en-US/docs/Glossary/Boolean/HTML).

An explicit conditional-attribute API is a separate v13 ergonomics proposal.
It must preserve escaping and distinguish actual HTML boolean attributes from
string-valued `aria-*` and `data-*` attributes. Do not reinterpret all false
values or silently change the stable macro's serialization behavior.

## Acceptance and evidence

Use synthetic downstream fixtures and offline browser tests; no application
production data or provider accounts are needed. Test the following observable
properties alongside the unchanged workspace and release requirements:

- Missing, expired and revoked sessions fail before the provider is called;
  ordinary authenticated users still cannot administer records. Cover direct
  URLs, fragments, unsupported methods and attempts to replace tenant context.
- A read-only configuration cannot mutate through individual, batch or custom
  routes. Existing tenant predicates and required-audit rollback remain intact
  in any mode that enables writes.
- Two instances with different pools, policies and prefixes cannot share data
  or identity. A never-read field is absent from the query projection, response,
  logs and audit payload, not merely hidden with CSS.
- The mounted shell passes strict CSP and resource-request assertions in a
  real browser, on success and denial paths. All locales and mount paths keep
  working with JavaScript disabled; verify actual form submission and results.
- Keyboard navigation, focus and table semantics work at 320/390/1440 px.
  Conditional boolean-attribute fixtures assert the browser's selected value
  and submitted query, rather than only searching generated HTML strings.
- New APIs compile on Rust 1.96.0 and the declared supported platforms. Retain
  the full tests, Clippy, formatting, packaging and applicable admission gates;
  passing a downstream application's tests is not framework release admission.

No part of this document claims that these APIs, browser journeys or release
checks have already been implemented or passed. Scope implementation after
stable maintenance priorities, starting with one complete read-only journey.
