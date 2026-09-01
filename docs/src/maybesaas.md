# Maybe SaaS: Incubating Products Built with Rullst

> **Status: strategy proposal, not an implementation claim.** None of the
> products named in this document exists merely because it is described here.
> The [capability ledger](capability-ledger.md) remains the evidence source for
> what the framework implements today.

Some of Rullst's most ambitious ideas need more than another module in a web
framework. They require a continuously operated service, official homologation,
named hardware, vendor interoperability, incident response, or a dedicated
security program. Those ideas can become separate products **built with
Rullst**, while the framework remains a reusable and honest foundation.

This is not automatically an open-core strategy, and it does not require every
advanced capability to become paid software. It is an incubation boundary: use
the smallest delivery model that can provide credible evidence and operations.

## The four possible delivery models

| Model | Use it when | What stays in the Rullst framework repository |
| :--- | :--- | :--- |
| **Framework crate** | The capability is a reusable local library and does not require a vendor-operated control plane. | Public traits, types, adapters, deterministic mocks, documentation, and contract tests. |
| **Open-source reference application** | Users need a deployable example or can reasonably self-host the whole capability. | The stable client contract and an example; the application can have its own repository and release cycle. |
| **Managed SaaS or private control plane** | The value depends on durable shared state, continuously updated provider rules, tenant operations, monitoring, or 24/7 availability. | A provider-neutral client/SDK, an explicit remote adapter, offline mocks, and a self-hosted escape hatch where practical. |
| **Conformance or hardware program** | Correctness depends on official test environments, physical devices, audited cryptography, interoperability matrices, or certification. | Interfaces and test vectors. Passing named external suites is required before a production claim. |

A capability can use more than one model. For example, an IoT platform can have
an open device SDK, a managed control plane, and a physical hardware
conformance lab.

## Recommended boundary

```text
Application built with Rullst
        |
        | typed client contract
        v
Framework adapter + deterministic offline mock
        |
        | explicit opt-in; never silent fallback
        v
Separately deployed product or customer-managed service
        |
        +-- durable state and tenant isolation
        +-- provider, fiscal, or device interoperability
        +-- monitoring, audit, support, and incident response
        +-- independent conformance evidence where required
```

The framework must remain useful without the managed product. Remote services
must not become hidden requirements for routing, ORM, authentication, or local
development. Live configuration must also fail closed instead of silently
falling back to a mock.

## Candidate incubation programs

The names below are working descriptions, not announced product names.

### 1. Fiscal Cloud for NFS-e

**Best initial form:** a dedicated fiscal program, followed by a managed SaaS
and a self-hostable/private deployment if the operating model proves viable.

The current Rullst capability now includes a bounded local DPS 1.01 builder,
checksum-pinned official XSD validation, PKCS#12 XMLDSig and mTLS client
preparation, while live transmission remains disabled. A live product would be
responsible for substantially more:

- official schemas, municipality/national variations, rejection codes, and
  protocol updates;
- PKCS#12 or delegated certificate custody, rotation, access control, and audit;
- official request/response envelopes, retries, reconciliation, cancellation,
  substitution, and immutable evidence around the local crypto/schema core;
- durable idempotency and a complete issuance state machine;
- official homologation environments, operational monitoring, and specialized
  support.

`rullst-capital` should retain the typed fiscal contract, request/response
models, offline preview, and an explicit remote adapter. The live fiscal engine
should have its own lifecycle because protocol and legal maintenance must not
be coupled to releases of the web framework. It must not advertise legal or tax
compliance without qualified review and current official evidence.

**Why it is attractive:** it solves a difficult Brazilian SaaS problem and
would exercise queues, cryptography, observability, billing, multi-tenancy, and
failure recovery in a real Rullst application.

**Why it is risky:** certification, certificate custody, protocol drift, and
financially consequential failures make this much more than an HTTP adapter.

### 2. IoT Device and OTA Control Plane

**Best initial form:** open device SDK plus a hardware conformance program;
managed SaaS only after selecting and testing named device families.

Rullst currently provides `no_std` telemetry/frame foundations and an
Ed25519-signed OTA manifest verification gate. A credible platform would add:

- device registry, provisioning, fleet inventory, and tenant isolation;
- MQTT/CoAP/LoRaWAN interoperability against named brokers and devices;
- durable anti-rollback counters, staged rollouts, download resumption, boot
  slots, health confirmation, and rollback orchestration;
- signed firmware provenance, software bills of materials, revocation, and
  incident response;
- physical test rigs covering power loss, partial writes, clock faults, poor
  networks, and recovery paths.

The device-side verifier should remain small, auditable, and independent of the
SaaS. A customer must not brick a fleet merely because the control plane is
temporarily unavailable.

### 3. Key Management, HSM, and Post-Quantum Interoperability

**Best initial form:** adapter crates and a conformance program, **not a new
cryptographic SaaS first**.

HSM and post-quantum work is worthwhile only for named protocols, devices, and
threat models. The safe path is to integrate audited libraries and standards,
then validate them against PKCS#11 services, cloud KMS/HSM providers, secure
elements, and published test vectors. Rullst must not invent cryptographic
primitives or use a generic “quantum-safe” badge.

A managed orchestration service could eventually handle key policy, rotation,
attestation inventory, and audit workflows. Actual private-key operations
should remain inside the selected HSM/KMS boundary whenever possible. On-premise
and bring-your-own-key modes are likely requirements, not optional extras.

### 4. Enterprise Identity Gateway

**Best initial form:** a separate identity service or private control plane,
with a dedicated crate boundary if reusable protocols are added.

`rullst-connect` is currently focused on OAuth2/OIDC and social identity.
Enterprise SAML, SCIM provisioning, organization/domain discovery, directory
synchronization, delegated administration, policy evaluation, and tenant audit
are a coherent product of their own. They should not be represented as a few
extra provider flags in the OAuth crate.

The framework can expose authentication/session integration and typed identity
events. The gateway can operate federation metadata, provisioning jobs,
enterprise connectors, and tenant-specific policy. A private deployment option
is important for organizations that cannot send identity metadata to a shared
SaaS.

### 5. Security Operations Control Plane

**Best initial form:** a self-hostable reference service before any managed
SaaS claim.

Rullst Security, Radar, Studio, and Nexus already provide useful local security
and telemetry foundations. A separate control plane could aggregate signed
events from multiple applications, manage distributed rate-limit policy,
deliver alerts to SIEM destinations, retain audit evidence, and coordinate
incident response.

This product would need explicit data-retention controls, regional placement,
tenant isolation, end-to-end authentication, redaction, bounded ingestion, and
an unavailable state that never weakens an application's local defenses. Claims
such as autonomous blocking, zero leakage, or complete OWASP coverage would
still require narrow definitions and independent evidence.

### 6. Messaging and Remote Storage

**Best initial form:** provider-neutral crates and conformance suites before a
managed service.

Kafka, RabbitMQ, Redis Streams, S3, and R2 are integrations with mature
services. Rullst now has a coherent bounded `rullst-messaging` contract for
idempotency, groups, leases, retry and dead letters, with a deterministic
process-local broker and opt-in durable local SQLite state. Remote adapters and
the remote-storage boundary still need provider-specific
conformance, backpressure, multipart, path/key and deterministic mock evidence.
Building another broker or object store as a SaaS would add little value until
Rullst applications reveal a concrete unmet need.

## What should remain inside the framework

Even when a separate product exists, Rullst should own:

- stable, provider-neutral traits and serializable contract types;
- feature-gated client adapters with bounded timeouts and typed errors;
- deterministic offline mocks selected only by explicit mock configuration;
- local development and self-hosted paths where they are practical;
- contract tests that every official or third-party provider must pass;
- telemetry hooks that expose availability without fabricating success;
- migration and escape-hatch documentation that prevents vendor lock-in.

The separate product should own production tenancy, billing, durable global
state, operational dashboards, on-call response, external-provider drift,
certification evidence, and service-specific data governance.

## Promotion gates

An incubated idea must not be promoted from roadmap to implemented because a
demo or one happy-path adapter exists. Before a public production claim, require
the applicable gates:

1. a named owner and a documented support/release policy;
2. a threat model, abuse cases, tenant-isolation tests, and data classification;
3. a versioned API contract, deterministic mock, and self-host/exit strategy;
4. end-to-end tests against named providers, devices, or official environments;
5. migrations, backup/restore, disaster recovery, observability, and SLOs;
6. bounded retries, idempotency, reconciliation, and failure-injection tests;
7. security review and, for cryptographic/fiscal work, independent specialist
   validation;
8. a private beta with real operators before general-availability language;
9. evidence linked from the capability ledger and the relevant roadmap.

Passing framework unit tests is necessary, but it cannot substitute for these
external and operational gates.

## Suggested order of investment

1. **Shared foundations:** distributed idempotency/rate limiting, canonical
   Security contracts, complete scaffold validation, and reproducible releases.
2. **One reference product:** build and operate a narrowly scoped Rullst service
   to validate deployment, multi-tenancy, telemetry, upgrades, and support.
3. **Fiscal discovery or enterprise identity:** choose one based on access to
   qualified domain partners and real design customers; do not start both as
   production programs simultaneously.
4. **IoT control plane:** proceed only with named hardware, a physical lab, and
   a partner willing to test real update failures.
5. **HSM/PQC:** integrate audited standards for concrete use cases after the
   threat model exists; keep speculative branding out of production claims.

The first reference product does not need to be the most spectacular one. Its
purpose is to prove that applications built with Rullst can be upgraded,
observed, secured, and operated for long periods. That operational evidence is
more valuable to the framework than another unchecked feature list.

## Strategic conclusion

Yes, a separate SaaS built entirely with Rullst can be an excellent direction.
It creates a demanding real customer of the framework and can finance deeper
engineering. The separation is successful only if it protects both sides:

- the framework stays open, portable, provider-neutral, and truthful;
- the product can evolve at the cadence required by its domain;
- neither side claims external certification or production readiness without
  evidence;
- every managed convenience has an explicit contract and a credible exit path.

The goal is not to move unfinished features behind a hosted API. It is to give
the ideas that require operations, certification, or hardware the independent
engineering program they need to become real.
