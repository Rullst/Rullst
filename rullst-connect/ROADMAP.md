# 🗺️ rullst-connect Roadmap

Welcome to the official roadmap for `rullst-connect`! The library currently supports 11 robust core providers, dynamic token parsing, `get_user_from_token` extraction, CSRF protection (via `state`), and `scopes` customization.

For our journey towards the **`v1.0.0`** release (Enterprise Level), we have planned the following major additions which are actively being worked on:

## ⏳ In Progress (Phases 3 & 4)

- [x] **Boilerplate Reduction Macros:** Use `define_provider!` to drastically cut down repetitive code in provider structs, making it easier for the community to contribute new providers.
- [x] **Native Framework Integration:** Create the optional `axum` and `actix` features, providing Extractors (like `AuthCallback`) so URL parsing of codes, states, and errors works magically.
- [x] **Token Revocation (Logout):** Add the `revoke_token` method to the trait to allow direct logout on the supported providers' servers.
- [x] **Mocking Tools (TDD):** A `MockProvider` to facilitate writing unit tests for end users of the library.
- [x] **OIDC Support:** Automatic and ultra-fast `id_token` validation for Google and Apple using JWT (without extra HTTP calls).
- [x] **Security Audit & PKCE:** Implement strict URL parsing, eliminate panics, and provide native `.with_pkce()` support across all providers.

## 🔮 Not So Distant Future

- [x] **HTTP Client Agnostic:** Generic client support (via `HttpClient` trait) allowing the use of `surf`, `reqwest-middleware`, or others instead of forcing `reqwest`. (Done in v5.2.0)
- [x] **Database Integration (rullst-orm, SQLx, Diesel):** Helper traits (e.g., `IntoDatabaseUser`) to seamlessly save the user into the database, with special focus on `rullst-orm` to keep the Laravel ecosystem vibe in Rust!
- [x] **HTTP Proxy Support:** Allow the configuration of corporate proxies for locked-down environments. (Done in v5.2.0)
- [x] **Refresh Token Module:** Automated `refresh_token` support in case the primary token expires. (Done in v5.1.0)
- [x] **Universal Avatar Standardization:** Advanced parsing to guarantee optimal resolutions for returned profile pictures.
- [x] **Leptos & Dioxus Integration:** Extractors for Fullstack / WebAssembly Rust frameworks (Leptos added in v5.2.0).
- [x] **Integration Tests with Mock Servers (`wiremock`):** Cover the real HTTP flow to guarantee that the parser correctly handles incomplete responses, expired tokens, or network failures. (Done in v5.2.0)
- [x] **Rate Limiting & Advanced Retry Policies:** Offer integrated wrappers (e.g., via `reqwest-middleware` and `reqwest-retry`) to perform native exponential backoff when providers reject requests due to rate limits (HTTP 429).
- [x] **Unified Provider Error Extraction:** Map error responses from providers (like "invalid_grant") into structured enums within `ConnectError` to drastically improve debugging experience.

## 🚀 Phase 5: High-Value & Developer Experience (Immediate Value)

- [x] **Strict Profile Normalization (`UniversalProfile`):** Expand on avatar standardization by guaranteeing a strictly typed and identical struct (`id`, `name`, `email`, `email_verified`, `avatar_url`) regardless of the underlying provider's payload quirks.
- [x] **Secure State/Nonce Handling (`AuthSession`):** Native integrations (via `tower-sessions`) to automatically save and validate CSRF `state` and `nonce` securely, removing the burden from the developer.
- [x] **Native Apple Secret Generation:** Handle "Sign In with Apple" painlessly by accepting a `.p8` key and Key ID to generate the required JWT `client_secret` on-the-fly.
- [x] **Embedded Local Mock IdP:** An embedded, ultra-lightweight Axum router (`mock_idp`) that perfectly simulates OAuth endpoints, allowing development teams to run full E2E local tests completely offline.
- [x] **Enterprise-Grade Observability:** Native integration with the `tracing` crate. Emit detailed spans during token exchanges and profile fetching to simplify debugging in production and distributed systems.
- [x] **OIDC Auto-Discovery (`.well-known`):** Create a generic `OidcProvider::discover("url")` that automatically downloads the OpenID configuration and sets up endpoints internally in a single line of code.
- [x] **Device Authorization Flow (RFC 8628):** Support for CLI and Smart TV logins where users enter a code on a secondary device, a critical feature for headless Rust applications.
- [x] **Cryptographic OIDC Signature Validation (JWKS):** Automatically fetch provider Public Keys and cryptographically verify the RSA signature of the `id_token` JWT to guarantee zero spoofing and meet enterprise security standards.

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

---

Want to help implement any of these features? Feel free to open a PR!
