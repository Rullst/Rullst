# 🗺️ rullst-connect Roadmap

> **Status policy (2026-08-26):** this detailed design backlog is preserved.
> Read legacy checkboxes through the audited
> [`rullst-connect` row](../ROADMAP.md#audit-of-the-detailed-crate-roadmaps) and
> the [capability ledger](../docs/src/capability-ledger.md); messaging remains a
> separate future boundary, not a shipped OAuth feature.

Welcome to the official roadmap for `rullst-connect`! The library currently supports 11 robust core providers, dynamic token parsing, `get_user_from_token` extraction, CSRF protection (via `state`), and `scopes` customization.

The crate is currently version 12. `[x]` means a bounded implementation exists;
`[~]` means only part of the historical sentence is implemented. Unchecked
messaging and enterprise identity sections remain roadmap work.

## ⏳ In Progress (Phases 3 & 4)

- [x] **Boilerplate Reduction Macros:** Use `define_provider!` to drastically cut down repetitive code in provider structs, making it easier for the community to contribute new providers.
- [x] **Native Framework Integration:** Create the optional `axum` and `actix` features, providing Extractors (like `AuthCallback`) so URL parsing of codes, states, and errors works magically.
- [~] **Token Revocation (Logout):** The trait fails closed by default; Google
  has a reviewed adapter and the mock implements a deterministic fixture. Other
  providers remain unsupported until individually implemented and tested.
- [x] **Mocking Tools (TDD):** A `MockProvider` to facilitate writing unit tests for end users of the library.
- [x] **OIDC Support:** Cryptographic `id_token` validation for Google, Apple, and custom OIDC providers using refreshed JWKS material.
- [~] **Security Audit & PKCE:** Strict URL parsing, PKCE builders and negative
  tests exist. Security review is continuous and no crate can promise the
  absence of every panic or integration mistake.
- [x] **Fail-Closed Provider Configuration:** Fallible constructors, exact loopback URL validation, and typed network-free credential modes.
- [x] **Rotation-Aware JWKS Cache:** Per-provider TTL, refresh on an unknown `kid`, and bounded stale-if-error restricted to already-known keys.

## 🔮 Not So Distant Future

- [x] **HTTP Client Agnostic:** Generic client support (via `HttpClient` trait) allowing the use of `surf`, `reqwest-middleware`, or others instead of forcing `reqwest`. (Done in v5.2.0)
- [~] **Database Integration (rullst-orm, SQLx, Diesel):**
  `IntoDatabaseUser` defines an application adapter contract; Connect does not
  persist identities or ship ORM/Diesel implementations.
- [~] **HTTP Proxy Support:** A custom `HttpClient` can install a reviewed proxy
  transport. There is no first-class proxy builder on the default client.
- [~] **Refresh Token Module:** All named provider adapters expose explicit
  refresh operations where supported; automatic expiry detection, storage and
  rotation remain application-owned.
- [~] **Universal Avatar Standardization:** Provider parsers normalize known
  avatar fields, but availability and optimal resolution cannot be guaranteed.
- [~] **Leptos & Dioxus Integration:** `AuthCallback` is framework-neutral and
  can be deserialized by either ecosystem; only Axum and Actix have native
  extractor implementations.
- [x] **Integration Tests with Mock Servers (`wiremock`):** Cover the real HTTP flow to guarantee that the parser correctly handles incomplete responses, expired tokens, or network failures. (Done in v5.2.0)
- [x] **Rate Limiting & Advanced Retry Policies:** Offer integrated wrappers (e.g., via `reqwest-middleware` and `reqwest-retry`) to perform native exponential backoff when providers reject requests due to rate limits (HTTP 429).
- [x] **Unified Provider Error Extraction:** Map error responses from providers (like "invalid_grant") into structured enums within `ConnectError` to drastically improve debugging experience.

## 🚀 Phase 5: High-Value & Developer Experience (Immediate Value)

- [x] **Strict Profile Normalization (`UniversalProfile`):** A typed,
  credential-free projection (`id`, `name`, `email`, `email_verified`,
  `avatar_url`) is returned from `ConnectUser::universal_profile()`.
- [~] **Secure State/Nonce Handling (`AuthSession`):** The Axum/tower-sessions
  extractor consumes and validates a stored one-time CSRF state. Generating and
  storing state plus OIDC nonce lifecycle are not yet automatic.
- [x] **Native Apple Secret Generation:** Handle "Sign In with Apple" painlessly by accepting a `.p8` key and Key ID to generate the required JWT `client_secret` on-the-fly.
- [~] **Embedded Local Mock IdP:** An explicitly mounted Axum
  authorization/access-token/userinfo fixture. It does not issue a signed ID
  token, expose JWKS or provide OIDC conformance and must remain local-only.
- [x] **Enterprise-Grade Observability:** Native integration with the `tracing` crate. Emit detailed spans during token exchanges and profile fetching to simplify debugging in production and distributed systems.
- [x] **OIDC Auto-Discovery (`.well-known`):** Create a generic `OidcProvider::discover("url")` that automatically downloads the OpenID configuration and sets up endpoints internally in a single line of code.
- [x] **Device Authorization Flow (RFC 8628):** Support for CLI and Smart TV logins where users enter a code on a secondary device, a critical feature for headless Rust applications.
- [x] **Cryptographic OIDC Signature Validation (JWKS):** Fetch provider public keys and verify supported asymmetric `id_token` signatures, issuer, audience, expiry, and optional nonce. This is one control, not a guarantee against every identity attack.

## 🏢 Phase 6: Enterprise Identity & B2B SaaS (Scale & Compliance)

*While other libraries charge thousands of dollars for these B2B and compliance-heavy features, `rullst-connect` aims to democratize enterprise-grade security by providing them completely free and open source.*

- **Dynamic Multi-Tenancy (B2B SaaS Ready):** Allow providers to be instantiated or hydrated dynamically per request, fetching credentials (Client ID/Secret) at runtime for multi-tenant architectures.
- **SAML 2.0 Support:** The absolute gold standard for massive corporations (Banks, Government, Fortune 500). Supporting secure XML-based SAML integration with Microsoft Entra ID (Azure AD), Okta, and PingIdentity.
- **SSO Domain Routing (Home Realm Discovery):** If a user types `user@acme-corp.com`, automatically route them to Acme Corp's specific Okta/Azure AD login screen without requiring them to click a specific provider button.
- **Advanced RBAC/ABAC Group Mapping:** Automatically mapping OAuth/SAML scopes and corporate groups directly into database roles and permissions upon login.
- **Compliance & Audit Logging (SOC2/GDPR Ready):** Built-in middleware to generate structured, legally compliant audit trails (who logged in, IP, device footprint, failed attempts) ready to be ingested by SIEM systems like Splunk or Datadog.
- **OIDC Back-Channel Logout:** Support for receiving secure webhooks from the Identity Provider. If an employee logs out centrally, `rullst-connect` automatically terminates their local application session.

## 🛡️ Phase 7: Bleeding-Edge Security & IoT (High Security)

- **FIDO2 / WebAuthn (Passkeys):** Passwordless biometric authentication (Windows Hello, FaceID, YubiKey) natively integrated with the user sessions.
- **SCIM 2.0 Provisioning (User Sync):** Allows corporate identity providers to automatically create, update, and deactivate users in your database in real-time (System for Cross-domain Identity Management).
- **DPoP (Demonstrating Proof-of-Possession - RFC 9449):** The bleeding-edge OAuth security standard. Binds the access token to the specific client using cryptography, making stolen tokens completely useless to attackers.
- **JWE (JSON Web Encryption):** Full support for deeply encrypted tokens (not just signed), ensuring zero data leakage in transit for highly classified environments.
- **Mutual TLS (mTLS) Authentication:** Support for presenting client certificates during token exchange, a strict requirement for Open Banking and military-grade network security.

## 🧠 Phase 8: Sci-Fi & Future-Proofing (AI & Quantum)

- **AI-Powered Risk-Based Authentication (RBA):** Built-in support for evaluating login context (IP, velocity, time, device fingerprint) using local ML models (via ONNX runtime in Rust) to generate a real-time risk score, dynamically triggering 2FA/MFA or blocking suspicious OAuth token exchanges.
- **Semantic Profile Normalization (NLP):** Using lightweight embedded natural language processing to automatically understand and map bizarre, undocumented JSON fields from obscure OAuth providers into the strictly typed `UniversalProfile` without needing explicit hardcoded mapping rules.
- **Post-Quantum Cryptography (PQC) Readiness:** Future-proofing JWT verification and token exchange against quantum computer attacks by supporting NIST-approved post-quantum signature algorithms (e.g., CRYSTALS-Dilithium, SPHINCS+).

## 📬 Phase 9: Cloud Message Queues & Distributed Event Streaming (`rullst_connect::queues`)

- **NATS & JetStream Driver (`rullst_connect::nats`):** Ultra-lightweight, sub-millisecond cloud-native pub/sub and distributed event persistence engine.
- **AWS SQS & SNS Native Client (`rullst_connect::sqs`):** Native AWS managed message queues and multi-subscriber topic fan-out.
- **Google Cloud Pub/Sub Client (`rullst_connect::gcp_pubsub`):** Distributed event streaming across GCP microservices.
- **Apache Pulsar Driver:** High-performance, multi-tenant distributed pub-sub messaging with geo-replication.

---

Want to help implement any of these features? Feel free to open a PR!
