# Rullst v12 threat models

> **Model version:** TM-13.0
> **Applies to:** the v13 development source and generated applications; the
> stable v12 branch keeps TM-12.10
> **Last source review:** 2026-09-03
> **Status:** maintainer baseline; application owners must extend it for their
> data, topology and providers. It is not a pentest or certification.

These models turn security ambitions into named abuse cases. A control is not
effective merely because its type exists: it must be mounted in the deployed
application and its negative case must pass. The
[security architecture](security-architecture.md) defines the canonical HTTP
boundary and the [v12 release audit](v12-release-audit.md) records repository
evidence.

## Method and common boundaries

The method is a lightweight STRIDE review: spoofing, tampering, repudiation,
information disclosure, denial of service and elevation of privilege. Every
model distinguishes untrusted network input, authenticated application
identity, process-local state, shared durable state and third-party/hardware
trust.

Process-local replay caches, counters, rate limits and audit buffers are useful
single-instance controls. They are not distributed guarantees. Forwarded
addresses are untrusted unless an explicit proxy policy (Core's
`TrustedProxyLayer`, see `CORE-03`) establishes the direct peer as trusted.

The machine-readable release minimum in
`.github/threat-model-release-minimum.json` binds 55 distinct abuse-case IDs to
67 evidence rows and 59 exact test executions across thirteen crates. The gate rejects missing markers,
missing tests and zero-test filters before executing Core, ORM, Auth, Nexus,
Studio, tenant ownership, Capital, AI, Mail, IoT, generated-default and Academy
negatives. Passing that bounded minimum does not imply that every case below is
closed.

## TM-CORE-1 — readiness and graceful request drain

**Assets:** service availability, readiness state, accepted requests and
dependency-health observations.

**Trust boundaries:** orchestrator ↔ health endpoint; application dependency
checks ↔ process-local readiness bits; new requests ↔ draining server.

| Abuse case | Required disposition | Repository evidence or remaining work |
| --- | --- | --- |
| `CORE-01` forged readiness or diagnostic disclosure | Readiness must fail during startup, any required-component failure, corrupted component state and drain; keep liveness process-only and return counts rather than component labels or error details. | The lifecycle-aware HTTP regression crosses startup → ready → draining, proves `503`/`200` transitions and verifies that the private component label is absent. Applications still own bounded dependency probes and determine which failures should gate traffic. |
| `CORE-02` shutdown race admits new work or abandons accepted work | Change admission monotonically to draining before the server's graceful wait, reject later requests and bound the operator's wait for already accepted requests. | The concurrency regression holds one admitted request, begins drain, rejects a second request with `503`, observes a typed timeout, releases the first and reaches zero. A real ephemeral-listener test proves the Server ready → shutdown → stopped lifecycle. The v13 candidate additionally retains admission through response data/trailers and proves streaming drain with two real processes and an existing proxy; hosted admission is pending. Load-balancer propagation, client retry/idempotency, supervisor kill deadlines and multi-replica coordination remain deployment/application work. |

**Release-negative minimum:** unready/draining response without private labels;
request accepted before drain completes while a later request is denied.

## TM-CORE-2 — client identity behind reverse proxies

**Assets:** rate-limit, lockout, ban and audit identity; the TLS evidence used
by Nexus Basic Auth; availability of clients that share a proxy.

**Trust boundaries:** arbitrary client ↔ forwarding headers; socket peer ↔
configured proxy networks; trusted proxy ↔ its appended entries.

| Abuse case | Required disposition | Repository evidence or remaining work |
| --- | --- | --- |
| `CORE-03` forged forwarding metadata chooses or collapses the client identity | Read `X-Forwarded-For` or RFC 7239 `Forwarded` (exactly one) only from a socket peer inside a bounded, validated list of trusted networks; never trust `/0` or an empty policy. Walk right to left, skip trusted hops, bound examined bytes/hops and keep the socket peer without failing the request when the trusted part is missing, malformed, unidentified or over-long. Report the forwarded scheme only after explicit opt-in. | `TrustedProxyLayer` unit tests cover direct-client spoofing, multi-hop chains, IPv6/brackets/ports/IPv4-mapped forms, `unknown`/obfuscated nodes, quoted-comma injection, byte/hop limits and client padding that is never parsed. Router tests prove Core `RateLimiter`, Security `rate_limit_middleware` and Nexus Basic Auth lockout bucket each client behind a trusted proxy while forged headers from a direct client are ignored, that only a trusted peer's `https` report satisfies Nexus TLS evidence, and that `Server` mounts the layer outside its rate limiter (static and hot-reload paths). A trusted proxy that forwards client-supplied headers unchanged, proxy address spoofing on a shared network, CDN/provider range maintenance and distributed limits remain deployment work. |

**Release-negative minimum:** a direct client forging forwarding headers keeps
its socket identity, and two clients behind one trusted proxy receive separate
rate-limit buckets.

## TM-AUTH-1 — sessions, passwords, OAuth/OIDC and passkeys

**Assets:** password verifiers, session keys/tokens, OAuth state and nonce,
passkey challenges/credentials, recovery paths and account identity.

**Trust boundaries:** browser ↔ application; application ↔ OAuth/OIDC provider;
application ↔ session/challenge store; operator ↔ key store.

| Abuse case | Required disposition | Repository evidence or remaining work |
| --- | --- | --- |
| `AUTH-01` forged, truncated, expired or legacy session | Reject before constructing identity; authenticate encryption version, expiry and payload. | Negative session parsing/encryption tests in `rullst-auth`. |
| `AUTH-02` weak/missing application key | Fail startup/configuration closed outside explicit deterministic mocks. | Weak-key and malformed-key tests. Malformed `.env` and `Rullst.toml` errors carry no file content (`.env` names an entry number, `Rullst.toml` a line and column), and cookie helpers skip `.env` when the process selects the environment; isolated-process regressions cover each. |
| `AUTH-03` password timing/CPU starvation | Use Argon2id off the async executor, bound input and normalize failure behavior. | `spawn_blocking` implementation and password tests. The recovery registry applies the Argon2 72-byte bound before account lookup, so overlong input yields one error for registered and unknown emails; a regression covers login, registration and reset. Deployment capacity remains application work. |
| `AUTH-04` OAuth login CSRF/code substitution | Bind state, nonce, redirect URI, issuer, audience and one-time callback context. | The optional Axum/tower-sessions path generates a ten-minute state/PKCE challenge plus OIDC nonce, stores verifier/nonce server-side, removes and immediately saves the one active challenge before validation, and tests sequential replay, mismatch, expiry, replacement and redaction. The unpublished v13 `AuthSessionForm` applies the same consumption to a bounded form POST body for `response_mode=form_post` providers such as Apple and tests mismatch, replay, method, media-type and size rejection. The store trait does not provide distributed compare-and-delete across already-loaded requests. `Provider::get_user_from_token` is documented as unsuitable for signing in a client-supplied token, because a provider userinfo response does not bind the token to this `client_id`; the unpublished v13 `verify_id_token` on Google and `OidcProvider` binds a client-supplied ID token to this client ID and a server-issued nonce, with wrong-audience and wrong-nonce negatives against the local signed IdP. Redirect registration, durable session/cookie/TLS policy, idempotent account linking/recovery and live-provider conformance remain RC application/deployment tests. |
| `AUTH-05` passkey origin/RP/challenge confusion | Require exact RP/origin, ceremony type, single-use bounded challenge, UV/UP flags and supported attestation/key formats. | Negative tests cover the bounded ES256/none scope. Challenge lifetimes are capped at one day and expiry arithmetic is checked, so configuration cannot panic and poison the challenge store. Normative WebAuthn conformance remains open. |
| `AUTH-06` session fixation/replay/revocation gap | Rotate on privilege change, expire server-side and provide device/session revocation. | Versioned expiring cookie sessions exist. The optional JWT policy uses bounded expiry, `kid` rotation, token IDs and subject session versions; production rejects its process-local store. The SQLite profile adds shared-local JTI/session-version revocation and bounded passkey inventory/revocation with counter CAS. The optional v13 shared PostgreSQL ceremony candidate adds account/session/RP-bound single use with local database/browser evidence; its monotonic shared clock lets a host up to five seconds behind adopt the recorded time and fails closed beyond that. Hosted admission remains pending. Cookie-session inventory, refresh flow, credential-repository replication/failover and application device ownership remain open. |
| `AUTH-07` account enumeration/recovery takeover | Normalize public responses, rate limit attempts, protect recovery factors and audit changes. | Login jail/timing helpers and subject-bound 80-bit recovery-code verifiers exist. Plaintext is returned only at enrollment and zeroized on drop; comparison is constant-time and consumption removes the verifier. Durable transactional consume, enrollment UX, step-up policy and full recovery workflow remain application-owned. |

**Release-negative minimum:** malformed/expired session, wrong key, cross-origin
passkey, replayed challenge, wrong RP, invalid callback state and repeated login
failure.

**TOTP replay (`AUTH-07`):** `verify_totp_code` is stateless and accepts the
previous, current and next 30-second step, so an observed code verifies again
for about 90 seconds. `verify_totp_step_after` accepts only a step newer than
the supplied last accepted step (RFC 6238 section 5.2); persisting the returned
step per secret atomically before admitting the login is application-owned.

## TM-CONNECT-1 — durable local OAuth token generations

**Assets:** access and refresh tokens, provider identity, application-account
binding, encryption keys and the latest accepted token generation.

**Trust boundaries:** application authorization ↔ account binding; provider
response ↔ refresh coordinator; local processes ↔ shared SQLite state; and
operator secret manager ↔ encrypted snapshot key.

| Abuse case | Required disposition | Repository evidence or remaining work |
| --- | --- | --- |
| `CONNECT-24` stale or competing local writer rolls a rotated token generation back | Store only authenticated ciphertext plus non-secret key/generation metadata, serialize local writes and replace/delete only after an exact generation compare-and-swap. Reject configuration drift, quota exhaustion, malformed rows and unsafe existing file targets without exposing the database path or token material. | The opt-in SQLite store uses `BEGIN IMMEDIATE`, an immutable persisted row ceiling and exact successor CAS. Two independent pools race generation one; exactly one succeeds, the loser fails with `GenerationConflict`, restart recovers the winner, and a stale delete is rejected. Separate negatives cover key mismatch, ciphertext authentication, metadata corruption, quota/configuration, plaintext absence and symlink targets. Provider-call leases, recovery after a losing remote refresh, key-manager operation, trusted directory permissions, backup, multi-host replication and live-provider conformance remain application/deployment work. |

**Release-negative minimum:** two local writers cannot both replace one observed
generation, and restart cannot turn a stale generation into the winner.

## TM-NEXUS-1 — administrative CMS

**Assets:** administrative session, model records, bulk actions, AI assistant
tools, audit evidence and security telemetry.

| Abuse case | Required disposition | Repository evidence or remaining work |
| --- | --- | --- |
| `NEXUS-01` anonymous/default access | Fail closed without explicit production policy. Local shortcut requires debug build and verified loopback peer. | Router policy and loopback tests. Basic Auth counts only failed presented credentials per IPv4 address or IPv6 /64; credential-less challenges never lock. A locked bucket is not evaluated for unknown clients, while browsers holding the per-process known-client cookie keep access, and pruning is amortized. Behind a shared proxy a first login can still be locked out unless Core's `TrustedProxyLayer` (`CORE-03`) is configured with the proxy's network; a router test then proves per-client lockout isolation. |
| `NEXUS-02` IDOR/BOLA on CRUD/batch routes | Resolve subject/tenant, then authorize object ownership or role for every ID and bulk member. | Models that explicitly register a text tenant column now scope every built-in read/mutation/batch predicate to a trusted `TenantContext`, inject it on create and fail closed without context. A real SQLite HTTP regression proves cross-tenant list/update/delete/batch denial and protected create input; pure SQL tests keep the all-feature release minimum portable. `#[derive(Nexus)]` keeps ORM-encrypted, `SecretString` and `#[orm(hidden)]` fields hidden and read-only, shows `#[orm(masked)]` fields only as write-only `Password` widgets by default and omits skipped fields, so the panel never lists, searches or overwrites them with plaintext; derive unit and facade regressions cover this. Global models, custom routes, identity/membership resolution, within-tenant object ownership and independent review remain host work. |
| `NEXUS-03` stored/reflected XSS | Escape dynamic HTML; make raw HTML explicit; enforce nonce CSP. | Core proves renderer/header nonce identity. Generated LMS auth/catalog/course/player style elements consume the request nonce, remove remote shell dependencies/inline style attributes and the materialized catalog escapes script-shaped search text. Nexus serves its stylesheet, script and vendored htmx same-origin with no inline code, handler or style attributes; an HTTP regression checks every Nexus page and fragment against the default production CSP, and the Linux browser check loads the shell under that CSP and fails on any violation. A route-by-route review of application routes mounted next to Nexus remains open. |
| `NEXUS-04` AI assistant privilege escalation | Treat model output as untrusted; allowlist typed tools and authorize each invocation as the human subject. | Prompt filtering exists; tool approval/audit policy remains open. |
| `NEXUS-05` destructive CSRF | Require CSRF on cookie-authenticated mutations and exact signed-webhook exemptions only. | The exact Core baseline regression proves a production cookie write is denied without the matching double-submit value, accepted with it and retains the outer header/CORS policy on denial. A nested-layer regression proves an explicitly protected application router wrapped by the `Server` baseline emits one matching token/cookie pair and accepts the valid POST instead of applying the protocol twice. Exact signed-webhook exemptions have separate unit negatives; a full Nexus browser/proxy flow remains open. |
| `NEXUS-06` audit repudiation | Record actor, tenant, object, operation, outcome and correlation ID in durable separate storage. | The opt-in required policy records the built-in authenticated actor, optional tenant, table/action, optional known key, count, committed outcome, bounded correlation ID, timestamp and format version in the same transaction; unavailable storage rolls the mutation back. A real SQLite regression covers commit and rollback, while schema tests cover all SQL dialects. It is same-database mutable evidence, not separate append-only or tamper-evident storage; denied attempts and automatically assigned create keys are not uniformly persisted. Host retention, backup, replication, immutable export and review remain open. |

Trust boundaries are browser ↔ Nexus, Nexus ↔ application policy/database and
Nexus ↔ LLM/provider.

## TM-STUDIO-1 — local developer control room

**Assets:** environment/configuration, logs, traces, database browsing, job
controls, prompts and source-error tooling.

| Abuse case | Required disposition | Repository evidence or remaining work |
| --- | --- | --- |
| `STUDIO-01` remote exposure | Compile/mount shortcuts only in debug development and bind loopback. `Server` mounts the panic console and `/_rullst/explain`/`/_rullst/autofix` only when the build has debug assertions and the environment is Development, so a release binary with an unset environment does not expose them; the panic console renders details only to loopback peers. Production exposure needs application-owned auth and TLS. | Generated startup and Studio boundary tests/docs; builder console-gate and panic-console peer tests. |
| `STUDIO-02` secret leakage | Mask environment/log fields by key and value patterns; never render raw credentials. | Environment viewer/redactor tests cover key markers plus URL user information, bearer tokens and secret-named assignments in allowlisted values; novel formats remain residual risk. |
| `STUDIO-03` SQL/table injection | Parameterize values and strictly validate dynamic identifiers. | Data-browser identifier/query-builder tests. |
| `STUDIO-04` arbitrary source file access | Require debug loopback, canonical allowlisted paths/extensions and bounded files. | Traversal, sensitive-file, non-loopback and extension tests. |
| `STUDIO-05` forged telemetry | Label local/unverified events accurately; never invent source IP, HMAC verification or provider status. | Telemetry integrity tests. |
| `STUDIO-06` destructive job/database action | Require verified local policy and same-origin checks; remote production controls remain disabled until separately authenticated, authorized and reviewed. | Studio serves `Referrer-Policy: same-origin`, so browser form posts carry the real origin; `Origin: null` passes only with `Sec-Fetch-Site: same-origin`. A browser-modelled regression covers Studio's served policy, an imposed `no-referrer`, cross-site and same-site (other local port) documents and a bare `null` origin; a local Chrome run confirmed the header values. Data-browser writes additionally require an unforgeable middleware marker, inspected complete PK, typed binds, exactly one affected row and exact delete confirmation. A key column outside the identifier boundary or column cap makes the table read-only, and the write transaction is rolled back unless exactly one row changed; a four-backend composite-key regression and a multi-row rollback test cover this. Importing the raw router is denied by an exact release-negative test. Queue retry/purge and feature-flag toggles require the same marker; raw-router negatives prove that they return `403` and leave queue records and flag values unchanged. Because safe methods pass without an `Origin`, `GET` views perform no schema or data writes; a four-backend regression proves that the feature-flag page no longer creates `rullst_feature_flags`. Application tenant/RBAC, durable audit and rollback remain open. |
| `STUDIO-07` forged or replayed remote span batch | Keep ingestion push-only; bind each endpoint to one producer name/key, authenticate the exact body and bounded source/timestamp/nonce fields, consume valid nonces atomically, reject stale/future requests and validate the complete batch before storage. | Wrong-source/key, HMAC tamper, stale timestamp, concurrent replay, schema/cardinality, deduplication and capacity regressions. TLS, key custody/rotation, clock synchronization and producer admission remain deployment work. |
| `STUDIO-08` cache secret exposure or broad deletion | Return metadata only, replace logical keys with keyed opaque browser tokens, omit values, omit bulk flush and require the verified-local mutation marker for one-entry invalidation. | Memory plus live Redis metadata contracts, HTML non-disclosure tests and forged/missing-marker mutation negatives. Application cache-key classification and operator policy remain external. |
| `STUDIO-09` database selection drift | Reuse the application's ORM pool; otherwise resolve the database only through the shared `Server`/Artisan resolver, never invent a fallback database and never echo configuration content in errors. | Resolver unit tests pin the process-environment, `.env` and `[database].url` precedence, full query strings and content-free errors; an isolated-process regression proves that an unconfigured `GET /studio` creates no pool and no `db.sqlite`. A Studio request that arrives before an application's own explicit `Orm::init` can still initialize the pool first; applications that choose their URL in code should initialize it before serving Studio. |
| `STUDIO-10` resource exhaustion through table views | Bound rows, cell text and search terms before data leaves the database or is bound per column. | Pages load 25 rows; the SELECT cuts cell text to 256 characters (key columns to 16 KiB plus one character, making longer keys read-only) and search terms above 256 bytes return `422`. A four-backend regression proves that an 8 KiB cell is cut and the search bound is enforced. The per-column `LIKE` scan remains proportional to table size. |

Trust boundaries are local browser ↔ loopback listener, authenticated trace
producer ↔ push-only application endpoint, Studio ↔ telemetry/database/cache,
and error console ↔ source filesystem.

## TM-TENANT-1 — multi-tenant data access

**Assets:** membership, tenant-scoped records, billing/workspace IDs, cache
keys, jobs, files, logs and exports.

| Abuse case | Required disposition | Repository evidence or remaining work |
| --- | --- | --- |
| `TENANT-01` trusted client header | Never accept tenant/role solely from an arbitrary header; bind to authenticated membership or a trusted gateway. | Strict tenant guard rejects absent context; gateway policy is application-owned. |
| `TENANT-02` object ID crosses tenant | Query with tenant/owner predicate and authorize the returned object server-side. | `UserContext` carries a validated optional tenant and `RbacGuard::authorize_tenant[_owner_or_role]` requires an exact match that even admin cannot bypass. The materialized LMS test exercises bounded school-scoped IDs; general route coverage remains open. |
| `TENANT-03` bulk/list/export leakage | Apply tenant predicate before pagination/count/export and validate every bulk member. | Must be proven per generated/application route. |
| `TENANT-04` cache/queue/storage collision | Include canonical tenant identity in keys, jobs and storage roots; validate again in workers. | Core's `TenantStorage`, `TenantCache`, `TenantRealtime` and `TenantPresence` can only be constructed from a validated `TenantContext`, apply immutable tenant namespaces and have exact same-key/channel/presence-room non-interference tests. Cache keys, realtime channels and presence rooms share one `tenants:<tenant>:<logical>` builder whose tenant segment encodes `%` as `%25` and `:` as `%3A`, so a logical name containing `:` cannot alias another tenant (identifiers without those characters keep their previous names); `("a", "b:c")` versus `("a:b", "c")` is tested for cache, broadcast and presence. Tenant identifiers made only of dots are rejected, and `TenantStorage` also refuses any tenant that is not exactly one normal path segment, with an exact tenant-`.` negative. On local storage it also refuses a tenant whose `tenants/<tenant>` directory the filesystem resolves under another name, so case-variant identifiers (`Acme`/`acme` on APFS or NTFS) and Windows trailing-dot stripping (`acme.`) cannot share a directory; the first tenant to create it keeps it. The cache wrapper exposes no cross-tenant flush; the realtime wrappers validate names and bound payloads. Academy's database outbox/worker persists and validates `school_id`; its leaderboard cache is tenant-scoped, validates decoded scope and is invalidated after score, quiz and correction. Generated ORM Redis model hashes bind the application namespace plus opaque tenant and table digests, require a typed tenant context for tenant models and reject another tenant's handle or decoded hash; an opt-in live Redis regression proves tenant/namespace separation, while the `orm:events:*` channels stay un-namespaced. Application room authorization, distributed realtime/liveness, search, metrics, exports, distributed cache/failover, remote bucket policy and broader Academy integration remain open. |
| `TENANT-05` confused-deputy background work | Persist actor/tenant/authorization intent and revalidate sensitive execution. | Shared durable job authorization contract remains open. |

Trust boundaries are identity ↔ membership, route ID ↔ object and application ↔
database/cache/queue/storage.

## TM-ORM-1 — portable document recovery

**Assets:** document contents, portable identifiers, application/collection
scope, snapshot encryption keys and the destination collection.

**Trust boundaries:** application writer ↔ repository inventory; source store ↔
application-owned snapshot storage; operator key custody ↔ snapshot opener; and
snapshot contents ↔ destination repository.

| Abuse case | Required disposition | Repository evidence or remaining work |
| --- | --- | --- |
| `ORM-01` copied, tampered or cross-scope snapshot discloses or substitutes documents | Encrypt and authenticate the complete versioned payload with a fresh nonce, explicit rotation key ID, length-delimited application/collection binding and fixed decode limits; authenticate before JSON decoding. | The exact negative changes ciphertext, key material and application binding and proves all three fail closed. Key bytes are consumed through a zeroizing temporary, opaque snapshot/key `Debug` output is redacted and plaintext labels are absent from the envelope. Key generation/custody/rotation, external snapshot permissions and deletion remain operator responsibilities. |
| `ORM-02` partial retry overwrites conflicting destination data or silently accepts extras | Accept only an empty destination or an exact matching subset, insert without replacement, treat only an exact raced duplicate as replay and verify the final complete inventory. Never delete an extra row or claim transactionality across stores. | The exact negative proves a differing row and an extra row fail before mutation. Deterministic tests cover partial resume and idempotent replay; the live matrix exports MongoDB→SurrealDB→MongoDB and checks ordered IDs. Export performs two equal observations, but formal consistency still requires the application to quiesce writers; a failed restore can retain prior successful inserts for the documented retry path. |

**Release-negative minimum:** ciphertext/key/scope substitution and conflicting
or extra destination state.

## TM-SEC-1 — declared HTTP payload contracts

**Assets:** route semantics, validated request data, schema configuration and
security telemetry.

| Abuse case | Required disposition | Repository evidence or remaining work |
| --- | --- | --- |
| `SEC-16` malformed, ambiguous or schema-confused JSON body | Require a JSON media type on unsafe schema-bound requests, classified as the route's JSON extractor does (any ASCII case, parameters, `json` subtype or `+json` suffix) so an accepted spelling cannot skip the checks; reject malformed, duplicate-key, oversized or deeply nested input before applying a precompiled closed schema. Never fetch attacker-selected schema references. | The bounded route policy compiles JSON Schema 2020-12 or one OpenAPI 3.1 component with local references, no network/filesystem resolver and linear-time regexes. The exact negative covers shape/additional-property confusion and external references; middleware tests cover 415/400/422 and exact body preservation. Auth, ownership, domain rules and non-JSON parameters remain separate. |

Trust boundaries are application schema configuration ↔ compiled validator and
untrusted HTTP body ↔ route handler.

## TM-SEC-2 — anomaly assessment and proof-of-work admission

**Assets:** service availability, canonical client subjects, challenge key,
challenge capacity and replay state.

| Abuse case | Required disposition | Repository evidence or remaining work |
| --- | --- | --- |
| `SEC-07` forged, replayed, cross-subject or resource-exhausting challenge | Authenticate the complete challenge, bind it to one canonical subject, cap difficulty/TTL/cardinality, reject invalid work, expire it and atomically consume one successful proof. Classification must remain explainable and must not become authorization by itself. | The exact negative tampers with the token, changes the subject, submits invalid work, verifies concurrently and replays/expires the challenge; only one local verifier succeeds. Aggregate collection, proxy/device identity, accessible alternatives, distributed replay state, adaptive evaluation and enforcement remain application/deployment work. |

Trust boundaries are host-supplied aggregates ↔ deterministic classifier,
application subject ↔ challenge and process-local replay state ↔ distributed
deployment.

## TM-SEC-3 — authenticated local security-event journal

**Assets:** normalized security-event evidence, HMAC keys, key identifiers,
record order and durable local file state.

| Abuse case | Required disposition | Repository evidence or remaining work |
| --- | --- | --- |
| `SEC-33` forged, substituted or rotation-confused local event | Authenticate a domain-separated canonical frame containing sequence, key identifier, predecessor tag, payload length and exact normalized event bytes; reject absent/wrong keys, forgery, reordering and removed interior records before returning an event as HMAC-verified. | The exact negative proves payload tampering, same-identifier wrong-key substitution and absent historical-key rotation fail closed. Additional deterministic tests cover restart across active-key rotation, predecessor/sequence attacks, quotas, symlink targets, external length changes and redacted/zeroizing key storage. This is one local writer and a maximum of eight keys/4,096 records/16 MiB; trusted key/path custody, whole-tail rollback checkpoints, multi-writer coordination, compaction, retention, remote delivery and acknowledgement remain operator/application work. |

Trust boundaries are local event producer ↔ authenticated journal, key manager ↔
rotation ring and filesystem state ↔ restart verifier.

## TM-PAY-1 — webhooks, billing, payouts and fiscal boundaries

**Assets:** provider secrets, event IDs, subscription/payout state,
amount/currency, invoice identity, replay state and fiscal documents.

| Abuse case | Required disposition | Repository evidence or remaining work |
| --- | --- | --- |
| `PAY-01` forged webhook | Verify the provider's exact signed bytes and algorithm cryptographically before parsing side effects. | Axum and Actix call the same verifier; provider-specific signature negatives plus Actix body/event preservation prove the bounded adapters. Wise's unauthenticated legacy payout parser fails closed for empty and live tokens and remains an explicit `mock_*` fixture; the v13 Wise transfer verifier checks RSA-SHA256 over the exact body against configured keys, including Wise's published sandbox example, before parsing. Signed Paddle transaction/adjustment events cannot pass the legacy subscription normalizer. |
| `PAY-02` replay/stale event | Enforce timestamp window and unique event ID in durable shared state before effects. | Freshness and bounded local replay are enforced before dispatch. The opt-in SQL ledger persists provider-scoped payload/event claims across SQLite/PostgreSQL/MySQL/MariaDB processes, fails closed on capacity/storage/profile errors and has restart/contention/four-protocol live evidence. Middleware claims before dispatch and is not exactly-once delivery; stable event IDs can instead share the caller's relational domain transaction, and a live MySQL/MariaDB/PostgreSQL case proves a caller transaction whose snapshot predates a concurrently committed claim is rejected. Wise signs no timestamp or delivery identity, so Wise transfer transitions need idempotent host processing. |
| `PAY-03` amount/owner substitution | Derive product/currency/owner from authenticated server state, never arbitrary client values. | Materialized SQLx/Turso billing scaffolds bind checkout to authenticated identity and deny cross-owner subscription reuse before customer binding; route/plan/provider review remains open. |
| `PAY-04` duplicate/partial transition | Use a transactional idempotent state machine and reconcile with the provider. | `check_and_record_event_key_with_transaction` can bind one stable provider event ID to one mutation in the same supported relational database. Cross-system atomicity, provider reconciliation, automatic state-machine policy, and middleware crash recovery remain open. |
| `PAY-05` payout destination takeover | Require step-up authentication, allowlist/change delay and independent audit. | Application workflow remains open. |
| `PAY-06` false fiscal authorization | Mock only explicitly; live/homologation fail `Unsupported` until XMLDSig, mTLS and SEFIN homologation exist. | Fail-closed fiscal tests. |
| `PAY-07` forged or confused SEFIN issuance response | Bound body size, status, environment, submitted DPS ID, 50-digit access key and signed `infNFSe/@Id`; malformed, mismatched, unsigned, tampered or decompression-amplified material must never become authorization. | The feature-gated offline protocol codec emits deterministic GZip/Base64 request JSON and distinguishes HTTP 201 authorization from bounded 400/403/500 rejection. It verifies both embedded XMLDSig values locally. Certificate trust, durable idempotency, live restricted-environment evidence and homologation remain open. |
| `PAY-08` replayed or substituted direct charge | Require integer minor units, currency, authoritative provider customer/payment-method identity and a durable order idempotency key; bind the accepted response before side effects. | `ChargeRequest` validates and redacts the bounded inputs. Stripe forwards `Idempotency-Key`, confirms off-session and rejects responses with mismatched amount/currency or non-accepted status. Offline receipts carry the distinct non-success `Mock` status and other adapters fail unsupported. The host still owns mandate/SCA setup, identity authorization, durable key uniqueness, webhook reconciliation and entitlement state. |
| `PAY-09` substituted or replayed paid invoice delivery | Bind the recipient, exact minor-unit total and currency to final non-mock payment evidence; expose a stable delivery identity and require durable claiming before retryable effects. | `PaidInvoice` rejects `Processing`/`Mock` and any e-mail, amount or currency mismatch. The opt-in Mail bridge renders bounded HTML/PDF, runs mandatory pre-flight and retains a stable delivery key. The host still owns authenticated order construction, webhook reconciliation, atomic outbox claiming, retry policy and provider acceptance; delivery is not exactly once. |
| `PAY-10` duplicated or response-confused metered usage | Bind every accepted response to provider-specific customer/item, metric identity, quantity, timestamp/action and retry evidence; never infer missing provider fields from a uniform subscription ID. | Stripe Meter Events forward a bounded identifier and recheck customer/event/value/timestamp/identifier. Lemon Squeezy Usage Records recheck item/quantity/action and mark the application event key as requiring durable outbox claiming. Both cap response bytes, redact identities and keep mocks visibly non-live. Provider-account acceptance, configured aggregation, durable storage/retry and invoice reconciliation remain application/operational work. |

Trust boundaries are provider ↔ webhook, user ↔ checkout, application ↔
provider and application ↔ durable billing state.

## TM-MAIL-1 — outbound content, suppressions and delivery telemetry

**Assets:** recipients, message content and attachments, provider event
identities, suppression state, tenant identity and operational observations.

| Abuse case | Required disposition | Repository evidence or remaining work |
| --- | --- | --- |
| `MAIL-01` executable, active or type-confused attachment leaves the process | Validate bounded metadata and recognizable signatures, reject active content and fail closed when authoritative inspection is unavailable. | The opt-in static `AttachmentInspectionGuard` completes inspection before the wrapped transport. Its strict local heuristic rejects executable magic, spoofed known types, active PDF/SVG, secrets and unsafe text links. The declared MIME type alone never selects the checks: it is compared case-insensitively and combined with the filename extension and content signature, so an active PDF, SVG, HTML or script file labelled `text/plain` or `application/xml` is rejected, and the strict policy rejects a declared type it does not inspect (such as `text/html` on `invoice.txt`) unless it is `application/octet-stream`; executable/script-host extensions are rejected under both policies. PDF `#xx` name escapes and compressed streams are not decoded. It is not antivirus, sandboxing, recursive archive inspection or CDR; production risk policy may require an independently operated scanner adapter. |
| `MAIL-02` forged, replayed or lost suppression event permits unwanted delivery | Authenticate the provider event before mutation, bind provider/event/payload exactly, persist suppression durably and check it before transport. | The opt-in SQLite store provides exact replay conflict detection, monotonic manual/bounce/complaint state, immutable quotas and shared-local restart/two-instance evidence; `SuppressionGuard` fails closed on suppressed recipients or unavailable state. One recipient parser reduces `Name <address>` forms to the bare address for the pipeline, disposable check, verified feedback and store lookup, so a display name cannot miss a suppression entry; unparsable recipients are rejected. Rullst does not authenticate provider webhooks in this API, and only already-verified events may be recorded. Replay-ID retention must cover the provider window. The optional v13 PostgreSQL store shares state across hosts behind a monotonic namespace clock that tolerates at most 300 s of backwards host skew and fails closed beyond it. Multi-host replication, webhook adapters and provider-account acceptance remain open. |
| `MAIL-03` message or recipient leaks through delivery telemetry | Emit only bounded low-cardinality outcomes and never recipient, subject, body, filename or provider response content. | `ObservedMailDriver` records provider label, outcome, elapsed time, attachment count and two booleans through a non-failing static sink. The bounded default sink is process-local; externally operated metrics/tracing export, retention and alerting remain deployment work. |
| `MAIL-04` oversized or adversarial message content exhausts the sending executor | Bound scanned text before inspection and keep every content scan linear in its input. | The mandatory pipeline rejects a subject over 2 KiB or an HTML/text body over 2 MiB before scanning; link extraction deduplicates through a hash set and the redactors and URL search are single forward passes. A regression test runs marker-, link- and PEM-dense bodies at the limit under a deadline. Scans still run on the calling task, so hosts should also cap request bodies. |
| `MAIL-05` a production misconfiguration silently selects the offline mock and loses mail | Select offline delivery only from explicit mock configuration and reject partial credentials. | SMTP selects its offline fixture only for an empty/`mock_*` host or a `mock_*` username/password; a real host without credentials is a real relay and a lone username or password is a `ConfigError`. REST providers keep the documented empty/`mock_*` credential convention, so deployments should assert `delivery_mode()` at startup. Without `MAIL_DRIVER` (process environment, then `.env`) or `[mail] driver`, the facade falls back to the metadata-only `log` driver only in development/test, resolved with the `Server` environment precedence; staging and production fail with `ConfigError`. |

Trust boundaries are authenticated application state ↔ mail message,
application ↔ inspection/suppression adapters, verified provider event ↔ local
suppression state and transport result ↔ observation sink.

## TM-MESSAGING-1 — durable messages, wire frames and correlation

**Assets:** message payloads and headers, idempotency state, ACK leases,
consumer progress, storage keys, broker routing metadata and trace correlation.

| Abuse case | Required disposition | Repository evidence or remaining work |
| --- | --- | --- |
| `MESSAGING-01` protected message content is recovered from a copied SQLite file | Offer an explicit authenticated-encryption profile, encrypt header values and payload before persistence, and fail startup on profile/key mismatch. | `connect_encrypted` uses AES-256-GCM with random nonces and a bounded keyring. A raw-database/restart regression proves the selected header and payload are absent while delivery round-trips. Topic, event/content type, IDs, timestamps, idempotency key, fingerprint and delivery state remain visible metadata; host key custody, permissions, backup and erasure remain required. |
| `MESSAGING-02` ciphertext or metadata is copied, reordered or altered | Bind namespace, topic, sequence, message ID, event/content type, timestamp and rotation key ID into AAD; reject authentication failure before claiming the message. | The exact negative swaps two valid ciphertext rows and receives `StorageAuthenticationFailed`; probe tamper and wrong-key tests cover startup. AES-GCM does not prevent deletion, rollback of the complete database or availability loss, so protected backup/rollback detection remains deployment work. |
| `MESSAGING-03` unsafe rotation silently makes retained records unreadable | Track every retained record's non-secret key ID, require every referenced prior key at startup, use only the primary key for new writes and permit removal only after old records are purged. | The rotation regression opens old+new, writes with the new primary, rejects new-only while an old record remains, then permits it after terminal purge. External secret-manager rotation, escrow, recovery rehearsal and multi-host rollout remain operator work. |
| `MESSAGING-04` malformed, oversized, cross-namespace or future-version wire frame enters broker state | Bound the frame before payload allocation, validate every field and canonical ordering, require exact namespace and reject unknown versions/trailing bytes. | A fixed byte digest freezes v1; exact negatives cover every truncation, wrong magic, unknown version, trailing data and namespace mismatch. The codec is only an envelope primitive, not a remote transport or provider protocol. |
| `MESSAGING-05` attacker-controlled trace metadata leaks baggage or creates ambiguous correlation | Propagate only strictly validated W3C version-00 `traceparent` and a conservative `tracestate` subset; exclude baggage and redact diagnostics. | Grammar, duplicate-key, zero-ID, uppercase and malformed-member negatives are exact; an in-memory delivery proves only the two allowlisted headers. Sampling, tenant authorization, exporter security and retention remain host work. |
| `MESSAGING-06` process stops after broker publication but before acknowledging the relational outbox | Commit domain state plus outbox row together, claim with a lease, publish the exact outbox event key as broker idempotency, then ACK the exact claim. Never call the two systems one atomic transaction. | The opt-in static `OrmOutboxRelay` maps one configured stream/topic, validates the claimed JSON and keeps payload/event/claim keys out of `Debug`. An exact crash-window regression publishes, lets the claim expire, republishes through a new claim, observes the broker's duplicate receipt and only then ACKs; one message exists. Worker supervision, retention, tenant authorization and destination idempotency remain application work. |

Trust boundaries are publisher/consumer ↔ broker contract, process ↔ SQLite
file/key manager, local envelope ↔ future remote adapter and untrusted
correlation headers ↔ tracing infrastructure.

## TM-AI-1 — prompts, RAG and tool execution

**Assets:** system prompts, tenant content, provider keys, retrieved data, tool
credentials, destructive operations and audit evidence.

| Abuse case | Required disposition | Repository evidence or remaining work |
| --- | --- | --- |
| `AI-01` direct/indirect prompt injection | Treat prompts/retrieval as untrusted and never equate heuristic pass with safety. | The versioned `rullst-ai-guardrails-v1` offline corpus runs injection/jailbreak cases across all built-in transports; adaptive and live-model evals remain open. |
| `AI-02` PII/secret exfiltration | Minimize/redact before dispatch and enforce tenant-aware retrieval. | The versioned offline corpus fixes exact implemented email/card/CPF/CNPJ redactions across built-in transports; CPF/CNPJ masking requires valid check digits and does not recognize alphanumeric CNPJs. `RagPipeline` guards/masks every selected passage, requires trusted tenant context and rejects mismatched tags; the application retriever must still apply authoritative datastore tenant/ownership predicates, and raw provider calls remain an explicit bypass boundary. |
| `AI-03` unauthorized tool/arguments | Allowlist typed tools, validate schema and authorize as the initiating subject after selection. | Local registry dispatch requires exact allowlist, principal authorization, closed bounded JSON, call budget and audit; destructive/financial approvals are one-use and payload-bound. Provider-native selection loops, domain authorization, approver authentication and durable audit remain open. |
| `AI-04` destructive autonomous action | Require human approval and idempotency for finance, deploy, deletion or privilege change. | Not yet a general framework guarantee. |
| `AI-05` SSRF/egress exfiltration | Validate destinations, block private/link-local/metadata networks and cap redirects/content/time. | `EgressPolicy::strict()` denies all hosts until an exact allowlist is configured. The opt-in `EgressFetcher` resolves under deadline, validates every answer, pins them into a proxy-free client, verifies the peer, revalidates manual redirects and bounds declared/streamed bytes; deterministic negatives stop private/mixed DNS before transport. It does not automatically wrap provider/application clients, and tenant-aware destination authorization, response validation plus a successful live-origin redirect/stream contract remain open. |
| `AI-06` unbounded cost/availability | Enforce body/token/time/concurrency budgets, cancellation and circuit breaking. | Built-in transports have configurable request deadlines and local tools have call/payload budgets; provider-neutral cancellation, concurrency limits and circuit breaking remain open. |
| `AI-07` hallucinated structured result | Require schema and authoritative server-side validation; prose is never authorization. | Structured-output foundations exist. |
| `AI-08` cross-tenant, reordered or incompletely erased chat memory | Select storage only from trusted tenant/conversation context, commit complete exchanges atomically, reject stale writers and erase the exact key. | The reusable stores bind tenant plus validated conversation ID and enforce bounded consecutive user/assistant pairs. The SQL adapter uses transactional revision CAS; its exact SQLite negative proves isolation, a single winner, deletion with foreign keys disabled and no orphaned messages, while live matrices cover PostgreSQL/MySQL/MariaDB protocols. MySQL/MariaDB key columns use `ascii_bin` and every tenant-scoped statement compares the key byte-exactly; the MySQL 8.0 and MariaDB matrices prove that tenant and conversation IDs differing only by case stay separate, that a legacy case-insensitive table fails closed and that the documented migration restores separation. Authentication/ownership inside a tenant, encryption, retention deadlines and backup erasure remain host policy. |

Trust boundaries are content ↔ model context, application ↔ provider, model
output ↔ tool dispatcher and retriever ↔ network/data sources.

## TM-IOT-1 — OTA manifest and device lifecycle

**Assets:** provisioned public key, firmware image/hash, target, monotonic
version, boot state and telemetry.

| Abuse case | Required disposition | Repository evidence or remaining work |
| --- | --- | --- |
| `IOT-01` forged/tampered manifest | Verify Ed25519 over canonical bounded bytes with the provisioned key. | Signature/tamper tests. |
| `IOT-02` rollback | Require version above a persisted monotonic counter before boot. | In-process anti-rollback exists; persistent integration remains open. |
| `IOT-03` wrong target/image | Verify target, declared length and cryptographic hash before flashing. | Manifest checks exist; downloader/flasher is roadmap. |
| `IOT-04` power loss/partial flash | Use a recoverable A/B boot flow and commit counter only after verified boot. | The target bank is always opposite the running bank that platform code reports; a commit does not move the running bank, and a regression proves a second update before reboot never targets it. Hardware/bootloader integration remains open. |
| `IOT-05` signing-key compromise | Use offline protected signing plus rotation/revocation. | Operational/HSM program remains open; simulators are not HSMs. |
| `IOT-06` telemetry spoof/replay | Authenticate channel/device and bind identity plus sequence/time. | Transport identity/MQTT remains open. |

Trust boundaries are signer ↔ distribution, distribution ↔ device, verified
manifest ↔ flasher/bootloader and device ↔ backend.

## TM-ACADEMY-1 — education, assessment and games

**Assets:** school and tenant membership, learner identity, protected content,
enrollment/entitlement, lesson progress, submissions and grades, score events,
leaderboards, rewards, automations, payment state, minors' data and
administrative evidence.

**Actors and trust boundaries:** learner, guardian, instructor, evaluator,
moderator, support operator, school owner and platform administrator; browser or
game client ↔ application; application ↔ school membership/database/cache/
queue/storage; application ↔ billing/media/notification provider; automation
or AI output ↔ authorized domain command. The client is never authoritative
for identity, entitlement, grade, score, reward or payment state.

| Abuse case | Required disposition | Repository evidence or remaining work |
| --- | --- | --- |
| `ACADEMY-01` forged learner, role or school context | Derive the subject and memberships from an authenticated server-side session or reviewed gateway; never trust form/header identity. | The LMS starter derives its numeric learner ID from the encrypted session, loads active persisted school memberships, accepts `X-School-ID` only as a selector within that set, rejects invalid/absent/ambiguous selection and binds the chosen tenant plus school-scoped active roles into server-created contexts. Invite/provisioning and the separate production identity lifecycle remain application work. |
| `ACADEMY-02` cross-user lesson or progress access | Resolve the lesson and active enrollment, authorize the enrollment owner before returning media or writing progress, and deny before side effects. | The LMS starter's learning service authorizes the enrollment owner from `UserContext` before any database access, and its routes sit behind the authenticated middleware. The materialized starter executes that owner-boundary test (`materialized_lms_executes_security_contracts`). |
| `ACADEMY-03` unenrolled or expired content access | Check a current server-side entitlement on every protected lesson, download and media request; signed URLs must be short-lived and subject-bound. | **Retired in v13 (TM-13.0).** The full Academy blueprint that implemented this was removed; the LMS starter has no enrollment release rules, expiry or signed media URLs. An application that adds them must implement and test this disposition. |
| `ACADEMY-04` cross-school data leak | Scope reads, counts, exports, cache keys, jobs, files, search and telemetry by authenticated school membership and test non-interference. | **Retired in v13 (TM-13.0).** The full Academy blueprint that implemented this was removed; the LMS starter has no schools, memberships or cohorts. An application that adds them must implement and test this disposition. |
| `ACADEMY-05` assessment or grade tampering | Version authoritative questions/rubrics, enforce attempt/time limits server-side, grade trusted inputs and audit every override. | **Retired in v13 (TM-13.0).** The full Academy blueprint that implemented this was removed; the LMS starter has no quizzes, assignments or grades. An application that adds them must implement and test this disposition. |
| `ACADEMY-06` replayed, impossible or reordered score | Authenticate a versioned `ScoreEvent`, recompute or validate results, enforce unique attempt/deduplication keys and deterministic leaderboard ordering. | **Retired in v13 (TM-13.0).** The full Academy blueprint that implemented this was removed; the LMS starter has no score events or leaderboards. An application that adds them must implement and test this disposition. |
| `ACADEMY-07` duplicate or confused-deputy automation | Commit a transactional outbox with domain state, deliver at least once, make handlers idempotent and reauthorize sensitive actions as the recorded actor/tenant. | **Retired in v13 (TM-13.0).** The full Academy blueprint that implemented this was removed; the LMS starter has no outbox-driven automation. An application that adds them must implement and test this disposition. |
| `ACADEMY-08` admin, support or AI privilege escalation | Apply least privilege and separation of duties, require step-up/approval for sensitive mutations and treat AI output only as untrusted input to guarded tools. | **Retired in v13 (TM-13.0).** The full Academy blueprint that implemented this was removed; the LMS starter has no publication review, role grants or an AI admin. An application that adds them must implement and test this disposition. |
| `ACADEMY-09` malicious upload or active content | Enforce real type/size/name limits, isolate parsing, quarantine/scanning and distinct SVG/HTML/archive/document/media policies. | Core now exposes a bounded in-memory admission contract with a hard size ceiling and application allowlist, canonical tenant/name checks, exact declared MIME/extension versus recognized signature, active-text rejection (text whose first significant character after whitespace, byte-order marks or zero-width characters opens markup, including XML prologs, comments and doctypes, or that contains `javascript:`), tenant-prefixed randomized quarantine keys, SHA-256 binding, a static-dispatch scanner contract and fail-closed release. The deterministic scanner is explicitly mock-only. The exact negatives reject active SVG/HTML text, including BOM-, prolog- and comment-prefixed markup, traversal, invalid tenant and MIME/extension spoofing. Multipart streaming, remote S3/R2 persistence, sandboxed parsers/transcoding, archives and a production malware adapter remain open. |
| `ACADEMY-10` minors' privacy or unsafe retention | Minimize collection, separate guardian consent when required, implement export/deletion/retention/anonymization and keep PII out of logs/telemetry. | **Retired in v13 (TM-13.0).** The full Academy blueprint that implemented this was removed; the LMS starter has no age bands, guardian consent or retention workers. An application that adds them must implement and test this disposition. |
| `ACADEMY-11` payment-to-entitlement substitution | Derive product, school, learner and amount from server state; apply verified idempotent provider events before granting or revoking entitlement. | Capital webhook foundations exist, but the LMS starter has no billing-to-entitlement integration and makes no paid-course claim. |
| `ACADEMY-12` availability, farming or multi-account abuse | Apply identity and origin budgets, replay controls, anomaly review and accessible rules without using a manipulable cache as source of truth. | The optional `redis-rate-limit` adapter provides atomic shared counters, bounded windows and hashed client keys; empty/`mock_*` configuration is visibly process-local and fails `require_distributed()`. The exact negative gate proves that fail-closed boundary, while a separate CI/release contract proves two independent clients consume one budget against digest-pinned Redis. Academy HTTP composition, cluster/failover and domain-specific farming/multi-account tests remain open. |

**Academy negative minimum before a comparative security claim:** anonymous,
cross-user, cross-course and cross-school content/progress requests; expired
entitlement; assessment replay/time manipulation; duplicate/impossible score;
automation redelivery; unauthorized grade/admin mutation; hostile upload;
payment replay; and multiple-account abuse. The exact release-negative gate now
exercises owner/cross-user access, bounded cross-school HTTP/database mutation
denials, versioned/idempotent score handling and the explicit distributed-rate-
limit startup boundary. Cross-subsystem tenant isolation, activity-specific
score recomputation, Redis failover and domain-specific multi-account behavior
remain outside that bounded evidence.

## TM-DEPLOY-1 — CLI, artifacts and release/deployment

**Assets:** source, generated projects, registry token, release tag, `.crate`
archives, evidence, deployment credentials and production target.

| Abuse case | Required disposition | Repository evidence or remaining work |
| --- | --- | --- |
| `DEPLOY-01` tag/version/DAG mismatch | Match every package/internal requirement to the tag and publish topologically. | Machine-readable order and metadata preflight. |
| `DEPLOY-02` artifact/source substitution | Package from tag, inspect/checksum/attest, reproduce in publish job and compare bytes. | Workflow gates exist; real RC run remains open. |
| `DEPLOY-03` secret/local artifact leakage | Deny `.env*`, key/certificate patterns, unsafe paths and unexpected archives. | `.crate` audit gate. |
| `DEPLOY-04` partial irreversible publish | Wait for index/checksum after each crate and resume only if the existing checksum matches. Never reuse a version. | Workflow exists; recovery runbook remains open. |
| `DEPLOY-05` CLI command/path injection | Validate identifiers, paths and image names; avoid shell interpolation and propagate failures. | Generator injection/path tests. |
| `DEPLOY-06` generated insecure default | Compile materialized matrices and assert production fails closed while local shortcuts are loopback debug-only. | Structural/representative blueprint matrices plus materialized auth, mail, chat and SQLx/Turso billing contracts exist; final RC/multi-OS evidence remains open. |
| `DEPLOY-07` compromised dependency/action | Pin actions, lock dependencies, audit advisories/licenses/sources and govern expiring exceptions. | Policy exists; tag-bound result remains open. |

Trust boundaries are contributor/CI ↔ repository, tag ↔ artifact, artifact ↔
registry and CLI ↔ filesystem/process/cloud.

## Review and change control

- Changes to authentication, authorization, tenant resolution, webhooks, AI
  tools, OTA or release flow must reference at least one abuse-case ID in their
  test or review description.
- New boundaries receive new IDs; IDs are never silently reused.
- Closing residual risk requires code/configuration, a negative test and
  commit-bound evidence. Documentation alone cannot claim a control is deployed.
- For every stable v12 maintenance release, maintainers must review TM-12.10
  against the exact candidate,
  applications must add topology/provider threats and an independent reviewer
  must cover the highest-impact paths.
