# Rullst v12 threat models

> **Model version:** TM-12.1  
> **Applies to:** the v12 release candidate source and generated applications  
> **Last source review:** 2026-08-26  
> **Status:** maintainer baseline; application owners must extend it for their
> data, topology and providers. It is not a pentest or certification.

These models turn security ambitions into named abuse cases. A control is not
effective merely because its type exists: it must be mounted in the deployed
application and its negative case must pass. The
[security architecture](security-architecture.md) defines the canonical HTTP
boundary and the [hardening status](hardening-status.md) records repository
evidence.

## Method and common boundaries

The method is a lightweight STRIDE review: spoofing, tampering, repudiation,
information disclosure, denial of service and elevation of privilege. Every
model distinguishes untrusted network input, authenticated application
identity, process-local state, shared durable state and third-party/hardware
trust.

Process-local replay caches, counters, rate limits and audit buffers are useful
single-instance controls. They are not distributed guarantees. Forwarded
addresses are untrusted unless a separately reviewed proxy policy establishes
the direct peer as trusted.

## TM-AUTH-1 — sessions, passwords, OAuth/OIDC and passkeys

**Assets:** password verifiers, session keys/tokens, OAuth state and nonce,
passkey challenges/credentials, recovery paths and account identity.

**Trust boundaries:** browser ↔ application; application ↔ OAuth/OIDC provider;
application ↔ session/challenge store; operator ↔ key store.

| Abuse case | Required disposition | Repository evidence or remaining work |
| --- | --- | --- |
| `AUTH-01` forged, truncated, expired or legacy session | Reject before constructing identity; authenticate encryption version, expiry and payload. | Negative session parsing/encryption tests in `rullst-auth`. |
| `AUTH-02` weak/missing application key | Fail startup/configuration closed outside explicit deterministic mocks. | Weak-key and malformed-key tests. |
| `AUTH-03` password timing/CPU starvation | Use Argon2id off the async executor, bound input and normalize failure behavior. | `spawn_blocking` implementation and password tests; deployment capacity remains application work. |
| `AUTH-04` OAuth login CSRF/code substitution | Bind state, nonce, redirect URI, issuer, audience and one-time callback context. | Provider parsing foundations exist; live provider conformance and recovery flows remain RC application tests. |
| `AUTH-05` passkey origin/RP/challenge confusion | Require exact RP/origin, ceremony type, single-use bounded challenge, UV/UP flags and supported attestation/key formats. | Negative tests cover the bounded ES256/none scope; normative WebAuthn conformance remains open. |
| `AUTH-06` session fixation/replay/revocation gap | Rotate on privilege change, expire server-side and provide device/session revocation. | Versioned expiring cookie sessions exist; shared revocation/device management remains roadmap. |
| `AUTH-07` account enumeration/recovery takeover | Normalize public responses, rate limit attempts, protect recovery factors and audit changes. | Login jail/timing helpers exist; a complete recovery product is application-owned. |

**Release-negative minimum:** malformed/expired session, wrong key, cross-origin
passkey, replayed challenge, wrong RP, invalid callback state and repeated login
failure.

## TM-NEXUS-1 — administrative CMS

**Assets:** administrative session, model records, bulk actions, AI assistant
tools, audit evidence and security telemetry.

| Abuse case | Required disposition | Repository evidence or remaining work |
| --- | --- | --- |
| `NEXUS-01` anonymous/default access | Fail closed without explicit production policy. Local shortcut requires debug build and verified loopback peer. | Router policy and loopback tests. |
| `NEXUS-02` IDOR/BOLA on CRUD/batch routes | Resolve subject/tenant, then authorize object ownership or role for every ID and bulk member. | RBAC ownership helpers exist; route-by-route independent review remains open. |
| `NEXUS-03` stored/reflected XSS | Escape dynamic HTML; make raw HTML explicit; enforce nonce CSP. | HTML/header tests; browser validation remains open. |
| `NEXUS-04` AI assistant privilege escalation | Treat model output as untrusted; allowlist typed tools and authorize each invocation as the human subject. | Prompt filtering exists; tool approval/audit policy remains open. |
| `NEXUS-05` destructive CSRF | Require CSRF on cookie-authenticated mutations and exact signed-webhook exemptions only. | Core CSRF tests; full browser flow remains open. |
| `NEXUS-06` audit repudiation | Record actor, tenant, object, operation, outcome and correlation ID in durable separate storage. | Tamper-evident local chain exists; durable append-only sink remains open. |

Trust boundaries are browser ↔ Nexus, Nexus ↔ application policy/database and
Nexus ↔ LLM/provider.

## TM-STUDIO-1 — local developer control room

**Assets:** environment/configuration, logs, traces, database browsing, job
controls, prompts and source-error tooling.

| Abuse case | Required disposition | Repository evidence or remaining work |
| --- | --- | --- |
| `STUDIO-01` remote exposure | Compile/mount shortcuts only in debug development and bind loopback. Production exposure needs application-owned auth and TLS. | Generated startup and Studio boundary tests/docs. |
| `STUDIO-02` secret leakage | Mask environment/log fields by key and value patterns; never render raw credentials. | Environment viewer/redactor tests; novel formats remain residual risk. |
| `STUDIO-03` SQL/table injection | Parameterize values and strictly validate dynamic identifiers. | Data-browser identifier/query-builder tests. |
| `STUDIO-04` arbitrary source file access | Require debug loopback, canonical allowlisted paths/extensions and bounded files. | Traversal, sensitive-file, non-loopback and extension tests. |
| `STUDIO-05` forged telemetry | Label local/unverified events accurately; never invent source IP, HMAC verification or provider status. | Telemetry integrity tests. |
| `STUDIO-06` destructive job/database action | Require explicit policy, CSRF and audit; remote production controls remain disabled until reviewed. | Local handlers exist; production authorization review remains open. |

Trust boundaries are local browser ↔ loopback listener, Studio ↔
telemetry/database and error console ↔ source filesystem.

## TM-TENANT-1 — multi-tenant data access

**Assets:** membership, tenant-scoped records, billing/workspace IDs, cache
keys, jobs, files, logs and exports.

| Abuse case | Required disposition | Repository evidence or remaining work |
| --- | --- | --- |
| `TENANT-01` trusted client header | Never accept tenant/role solely from an arbitrary header; bind to authenticated membership or a trusted gateway. | Strict tenant guard rejects absent context; gateway policy is application-owned. |
| `TENANT-02` object ID crosses tenant | Query with tenant/owner predicate and authorize the returned object server-side. | `RbacGuard::authorize_owner_or_role` and scanner heuristics; route coverage remains open. |
| `TENANT-03` bulk/list/export leakage | Apply tenant predicate before pagination/count/export and validate every bulk member. | Must be proven per generated/application route. |
| `TENANT-04` cache/queue/storage collision | Include canonical tenant identity in keys, jobs and storage roots; validate again in workers. | Storage traversal protections exist; cross-subsystem tenancy tests remain open. |
| `TENANT-05` confused-deputy background work | Persist actor/tenant/authorization intent and revalidate sensitive execution. | Shared durable job authorization contract remains open. |

Trust boundaries are identity ↔ membership, route ID ↔ object and application ↔
database/cache/queue/storage.

## TM-PAY-1 — webhooks, billing, payouts and fiscal boundaries

**Assets:** provider secrets, event IDs, subscription/payout state,
amount/currency, invoice identity, replay state and fiscal documents.

| Abuse case | Required disposition | Repository evidence or remaining work |
| --- | --- | --- |
| `PAY-01` forged webhook | Verify the provider's exact signed bytes and algorithm cryptographically before parsing side effects. | Provider-specific signature negative tests. |
| `PAY-02` replay/stale event | Enforce timestamp window and unique event ID in durable shared state before effects. | Freshness/local replay exists; distributed durable idempotency remains open. |
| `PAY-03` amount/owner substitution | Derive product/currency/owner from authenticated server state, never arbitrary client values. | Generated billing identity tests; live route review remains open. |
| `PAY-04` duplicate/partial transition | Use a transactional idempotent state machine and reconcile with the provider. | Cross-instance atomic workflow remains open. |
| `PAY-05` payout destination takeover | Require step-up authentication, allowlist/change delay and independent audit. | Application workflow remains open. |
| `PAY-06` false fiscal authorization | Mock only explicitly; live/homologation fail `Unsupported` until XMLDSig, mTLS and SEFIN homologation exist. | Fail-closed fiscal tests. |

Trust boundaries are provider ↔ webhook, user ↔ checkout, application ↔
provider and application ↔ durable billing state.

## TM-AI-1 — prompts, RAG and tool execution

**Assets:** system prompts, tenant content, provider keys, retrieved data, tool
credentials, destructive operations and audit evidence.

| Abuse case | Required disposition | Repository evidence or remaining work |
| --- | --- | --- |
| `AI-01` direct/indirect prompt injection | Treat prompts/retrieval as untrusted and never equate heuristic pass with safety. | Cross-provider guardrail tests and deterministic mocks. |
| `AI-02` PII/secret exfiltration | Minimize/redact before dispatch and enforce tenant-aware retrieval. | High-level client redaction tests; raw provider calls are an explicit boundary. |
| `AI-03` unauthorized tool/arguments | Allowlist typed tools, validate schema and authorize as the initiating subject after selection. | Registry exists; authorization/approval envelope remains open. |
| `AI-04` destructive autonomous action | Require human approval and idempotency for finance, deploy, deletion or privilege change. | Not yet a general framework guarantee. |
| `AI-05` SSRF/egress exfiltration | Validate destinations, block private/link-local/metadata networks and cap redirects/content/time. | General fetcher egress policy remains open. |
| `AI-06` unbounded cost/availability | Enforce body/token/time/concurrency budgets, cancellation and circuit breaking. | Provider matrix remains open. |
| `AI-07` hallucinated structured result | Require schema and authoritative server-side validation; prose is never authorization. | Structured-output foundations exist. |

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
| `IOT-04` power loss/partial flash | Use a recoverable A/B boot flow and commit counter only after verified boot. | Hardware/bootloader integration remains open. |
| `IOT-05` signing-key compromise | Use offline protected signing plus rotation/revocation. | Operational/HSM program remains open; simulators are not HSMs. |
| `IOT-06` telemetry spoof/replay | Authenticate channel/device and bind identity plus sequence/time. | Transport identity/MQTT remains open. |

Trust boundaries are signer ↔ distribution, distribution ↔ device, verified
manifest ↔ flasher/bootloader and device ↔ backend.

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
| `DEPLOY-06` generated insecure default | Compile materialized matrices and assert production fails closed while local shortcuts are loopback debug-only. | Structural matrix/two compiled projects exist; full matrix remains open. |
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
- Before stable v12, maintainers must review TM-12.1 against the exact RC,
  applications must add topology/provider threats and an independent reviewer
  must cover the highest-impact paths.
