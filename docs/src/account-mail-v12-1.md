# Account mail in 12.1

This is an opt-in 12.1 source contract. Updating a dependency does not migrate an
application's account table, replace its session middleware, configure a sender
or deploy a worker. See the [release record](v12.md) for publication status.

## Boundaries

`rullst-auth::recovery` owns the authoritative account registry, password reset,
revocable opaque sessions and encrypted transactional outbox. `rullst-mail` owns
message construction, mandatory preparation and transport. The small
`rullst::account_mail` bridge composes them without coupling Auth to a provider
or introducing another publishable crate.

Enable `account-mail-postgres` or `account-mail-sqlite` on `rullst`, or the
corresponding `recovery-*` feature on Auth when supplying a different worker.
These features do not enable the general ORM. Features remain additive; disable
umbrella defaults when a backend-exclusive dependency graph is required.

## Adoption sequence

1. Obtain two independent persistent random 32-byte application secrets from a
   secret manager. `RecoverySecrets::new` separates keyed token/email digests
   from AES-256-GCM outbox/address encryption. Never use the fixture keys from
   tests. There is no automatic key rotation: plan a reviewed migration and
   backup restoration with the same secrets.
2. Connect `SqlRecoveryStore`, then explicitly run `migrate()` during deployment.
   PostgreSQL and SQLite share the fixed authoritative schema. Namespace stores
   by application/tenant with separate databases and secrets; a public request
   cannot select a tenant or supply a database URL.
3. Register new accounts with `register_account_with_locale`. Account creation
   and the encrypted welcome notice commit together. The supported recorded
   locales are English, Brazilian Portuguese and Spanish; the worker supplies a
   deterministic fallback. Existing accounts require an application migration;
   there is no automatic import of arbitrary password tables.
4. Use `authenticate`, `create_session` and `verify_session` for this registry.
   Verify the opaque session on every authenticated request. Set it in a Secure,
   HttpOnly, appropriately SameSite cookie. Use `revoke_session` for logout.
   Legacy encrypted-only session cookies do **not** gain revocation implicitly.
5. A protected, independently rate-limited request endpoint calls
   `request_password_reset(email, trusted_now)`. Return the same generic
   acknowledgement for unknown, throttled and accepted requests. No mail provider
   is contacted inline. Database failures require a generic service response.
6. A separate bounded worker calls `deliver_next_account_mail` with an
   `AccountMailConfig` containing the server-owned HTTPS origin, reset endpoint,
   sender, application name and fallback locale. The endpoint cannot contain a
   query or fragment. The provider must have a verified sending domain.
7. Submit the new password and opaque code to a CSRF-protected consume endpoint
   using `complete_password_reset`. Never include the password in a URL. On
   success, direct the member to login; no automatic login is performed.

The reset page must set `Cache-Control: no-store` and `Referrer-Policy:
no-referrer`, remove the action query from browser history, avoid third-party
page resources, and ensure proxy/access logs omit the query string. The SDK
cannot configure a deployed proxy. Apply separate ingress limits to login,
request and consume routes, and test response timing in the deployed system.
The store enforces a 250 ms minimum request duration, but this alone cannot hide
arbitrary database delays or establish indistinguishability under outages.

## Reset and delivery guarantees

- Reset codes use 256 bits from the operating system CSPRNG and expire after
  20 minutes. The credential table retains only a purpose-separated keyed
  digest. A replacement request invalidates previous unused codes.
- Password replacement, consumption, sibling invalidation, session revocation
  and creation of a password-changed notice share one database transaction.
  Failure to persist the outbox rolls the password change back.
- Recovery requests have a three-per-account/15-minute limit and a global
  120-per-minute ceiling. Consumption has an independent global 60-per-minute
  ceiling before expensive password hashing. These are bounded defaults, not a
  substitute for per-client/distributed ingress controls.
- The outbox encrypts recipient and reset code, with per-record authenticated
  encryption. The recovery credential remains digest-only; the delivery copy is
  decryptable using the application encryption key until expiry/terminal cleanup.
  Retention is bounded at 10,000 records, with six attempts, 60-second leases and
  exponential retry delay. Old workers cannot acknowledge a newer lease.
- Reset emails render an absolute UTC expiry, keeping the body identical across
  retries. Stable delivery IDs reach Resend through observation, inspection,
  suppression, resolver and failover wrappers. Other transports remain
  at-least-once. Switching providers or crashing after acceptance can duplicate
  mail. Resend's [idempotency contract](https://resend.com/blog/engineering-idempotency-keys)
  has its own retention window; this is not an exactly-once inbox guarantee.

## Message content and feedback

`ActionLink` validates the application's exact configured origin. `AccountMail`
provides deterministic localized welcome, reset, password-changed, verification,
new-device, security-alert, address-change, closure and export notices. These
additional templates do not implement the corresponding identity transactions.
`MailPurpose` identifies their purpose; a template is not a marketing consent
record or a legal-compliance certificate.

MAIL-001/RULLST-005 is fixed in the mandatory pipeline: bounded opaque `token=`
query values inside safe body URLs survive preparation, including HTML-escaped
query separators. Other credentials and non-URL token text remain redacted.
`Message`, action links and recovery notices omit sensitive content from Debug;
`LogDriver` emits delivery metadata only. Do not log values explicitly exposed
for delivery by `expose_url`, `expose` or `into_message`.

`ResendFeedbackVerifier` authenticates the exact raw body with Svix v1 HMAC and
five-minute freshness. Reject duplicate signature headers at the HTTP boundary.
Use its `suppression_event()` with `MutableSuppressionStore::record`, then place
`SuppressionGuard` around the worker transport. Permanent bounces and complaints
suppress delivery; transient failures do not. A durable suppression store is
required across process restarts. The SDK provides a SQLite suppression adapter;
other production deployments supply their own `SuppressionStore`. Verified
feedback does not automatically mutate an unrelated account registry.

Security templates add no tracking or marketing unsubscribe dependency. Disable
tracking at the provider/domain/account level as well. In particular,
[Postmark server settings can override the message flag](https://postmarkapp.com/developer/user-guide/tracking-opens/tracking-opens-per-email).
Marketing consent, legal basis, preference withdrawal and retention are
application responsibilities; enabling Mail cannot establish GDPR/LGPD compliance.

## Azure Communication Services

`AzureCommunicationDriver` supports the ACS Email 2023-03-31 REST
contract. Set `MAIL_DRIVER=azure-acs` and
`AZURE_COMMUNICATION_EMAIL_ENDPOINT=https://RESOURCE.communication.azure.com`.
In Azure Container Apps, `AzureManagedIdentity` uses the platform-injected
loopback `IDENTITY_ENDPOINT`/`IDENTITY_HEADER` and optional user-assigned
`AZURE_CLIENT_ID` for the Communication Services resource. The host must grant
the identity email-sending permission and verify its domain/sender.

The driver disables engagement tracking, bounds payload/response sizes,
validates the operation-polling origin, and accepts only a terminal successful
operation as success. A pending operation after the bounded wait needs
reconciliation; ACS retries have at-least-once semantics. An empty or `mock_*`
endpoint/credential is deterministic offline behavior, not Azure acceptance.
Live Managed Identity/ACS sending still requires deployment acceptance.
See the official [REST contract](https://learn.microsoft.com/en-us/rest/api/communication/email/email/send?view=rest-communication-email-2023-03-31)
and [Container Apps identity contract](https://learn.microsoft.com/en-us/azure/container-apps/managed-identity?tabs=portal,http).

## Remaining proposal work

The examples proposal is broader than welcome/password recovery. A Redis
identity registry, durable email-verification/address-change transactions,
marketing consent policy, and an authorized/audited reset-resend administration
workflow are separate work. Existing Mail observers and `outbox_snapshot()`
provide minimized application diagnostics; this change does not expose a global
mailbox browser, reset codes or administrative resend endpoint.

Local contracts cover mandatory preparation, localized mock delivery, SQLite
restart/encryption/fencing, concurrent single use, rollback, session revocation,
signed feedback and PostgreSQL recovery. CI runs the PostgreSQL contract against
a pinned disposable image. Provider fixtures do not establish actual delivery,
reputation, merchant readiness or regulatory compliance.
