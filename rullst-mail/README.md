# Rullst Mail 📬

`rullst-mail` is Rullst's transactional email and mailables engine. Every official dispatch path now passes through one pre-flight pipeline for CRLF protection, recipient deliverability checks, content security scanning, and DLP sanitization before queueing or transport delivery.

Resend, SendGrid, Postmark, SendPulse, Mailjet, Mailtrap, ACS and the explicit
SES bearer-proxy adapter share a
pooled REST client with redirects and ambient proxy settings disabled, a
five-second connection deadline and a 30-second request deadline, including
error-body reads. Error-body reads remain capped at four KiB; errors retain only status and
retry metadata, omitting provider bodies that may echo personal data.
Configure the final trusted endpoint directly. Native SES uses its separate
AWS SDK transport and SMTP uses Lettre; the REST deadline claim does not
describe those transports or prove provider acceptance.

The optional SQLite suppression store uses SQLx transaction guards: dropping
an uncommitted writer schedules rollback before the connection is reused.
Cancellation racing an already dispatched commit still needs outcome
reconciliation. This protects local atomicity; provider authentication and the
application's recipient/tenant policy remain separate responsibilities.

---

## Shared suppression candidate (v13)

Optional `postgres` adds `PostgresSuppressionStore` to the existing guard and
event contracts. Independent instances share authoritative recipient suppression
and replay state. Addresses/event IDs are stored as keyed identifiers, quota and
configuration drift fail closed, and ordinary startup needs no DDL privileges.
Install the correctly scoped guard in every sending process and worker; it is
not enabled automatically. See the [shared suppression contract](https://github.com/Rullst/Rullst/blob/main/docs/src/shared-mail-suppression.md)
for setup, retention, provider-authentication boundaries, restart/worker evidence
and pending hosted/package admission. The facade feature is `mail-postgres`.
Hosts sharing a namespace may differ by up to 300 seconds of clock skew; a
larger backwards clock step fails closed as `SuppressionUnavailable`.

## ✨ Features

- **🛡️ Typed failures:** production delivery paths return `MailError`; malformed messages and provider configuration fail closed.
- **⚡ Delivery and Test Drivers:**
  - **Resend** (`ResendDriver`) — Native REST API with scheduled delivery & RFC 8058.
  - **SendGrid** (`SendGridDriver`) — Native v3 REST API with personalization & attachments.
  - **SendPulse**, **Mailjet**, **Mailtrap** — Native transactional REST adapters; see the configuration and bounded contracts below.
  - **Azure Communication Services** — Native Email REST with Container Apps Managed Identity.
  - **Postmark** (`PostmarkDriver`) — High-deliverability transactional REST API with Message Streams.
  - **AWS SES v2** (`AwsSesDriver`, `aws-ses`) — official AWS SDK/SigV4 native transport with temporary/rotating credential support, plus deterministic offline fixture and an explicit legacy proxy boundary.
  - **Native SMTP** (`SmtpDriver`) — Pure async Lettre transport with implicit TLS on port 465 and mandatory STARTTLS on every other port.
  - **Memory & MailTrap** (`MemoryDriver`, `MailTrap`) — Local zero-I/O in-memory harness, distinct from the hosted Mailtrap service with fluent assertions.
  - **Log** (`LogDriver`) — Terminal and disk file logging (`storage/logs/mail.log`).
- **🔀 Typed Circuit Breaker & Automatic Failover (`FailoverDriver`):** Fails over only for transport, HTTP 5xx, provider rate-limit, or transient SMTP failures; permanent message/configuration/provider rejection stays on the original error path. `SuppressionUnavailable` and `AttachmentInspectionUnavailable` are `Transient` (retry later) but never failover-eligible. A fallback that returns such an error ends the chain with it; when every driver fails transiently, the result is a `Transient` error, or `RateLimited` with the bounded `Retry-After` when the last driver was rate limited. Every attempt keeps the caller's tenant context or delivery ID, so a wrapped `TenantMailResolver` selects the tenant's driver. Structured tracing exposes bounded decision fields without provider bodies.
- **🏢 Auth-bound Multi-Tenancy Resolver (`TenantMailResolver`):** Select isolated in-process drivers directly from a trusted Core `TenantContext`; registry failures and invalid IDs fail closed.
- **📎 Bounded Attachments & Inline CID Assets:** The shared pre-flight contract caps count and byte size, validates safe basenames/MIME/CID metadata and requires every unique inline CID to be referenced by HTML. Resend, SendGrid, Postmark, native SES, the SES bearer proxy and SMTP serialize the same owned-byte model; transports copy or Base64-encode as required.
- **🔬 Opt-in Attachment Inspection (`AttachmentInspectionGuard`):** A strict bounded local policy rejects executable magic, spoofed known types, active PDF/SVG, secrets and unsafe text links before transport. Checks follow the case-insensitive declared type, the filename extension and the content signature together, never the declared type alone. A static `AttachmentInspector` adapter boundary supports an independently operated production scanner.
- **🚫 Durable Recipient Suppression (`sqlite`):** `SuppressionGuard` checks manual, hard-bounce and spam-complaint state before transport. The SQLite store binds verified provider/event identities, detects conflicting replay, enforces immutable quotas transactionally and survives restart or multiple local processes.
- **📊 Secret-Minimized Delivery Observability:** `ObservedMailDriver` records only a bounded provider label, terminal outcome, latency, attachment count and scheduling/tenant booleans through a non-failing static observer.
- **⏰ Durable Scheduling (`.send_at()`, `.send_in()`):** SQLite and Redis queues persist schedules for up to 366 days and never claim early; direct Resend/SendGrid delivery uses provider scheduling, and direct SendGrid rejects a schedule more than 72 hours ahead (its provider limit) with `ConfigError` before any request. Real SMTP, Postmark, Log and SES paths reject future direct delivery and must use a durable queue; offline fixtures may retain the timestamp for assertions.
- **🕵️ Outbound Phishing & Homograph URL Interceptor (`.validate_security()`):** Pre-flight detection of mixed-script Unicode IDN spoofed domains (`pаypal.com` with Cyrillic characters), checked per DNS label of the link host and user-info only, so single-script IDNs such as `παράδειγμα.gr` or `пример.com` and non-Latin query text are allowed while all-lookalike Cyrillic labels under a non-Cyrillic TLD are rejected, and dangerous URI schemes (`javascript:`, `data:text/html`).
- **📜 RFC 8058 One-Click List-Unsubscribe:** Automatic compliant header injection (`List-Unsubscribe`, plus `List-Unsubscribe-Post: List-Unsubscribe=One-Click` only for an HTTPS unsubscribe URL, as RFC 8058 requires). The unsubscribe email must be one bare address and the URL may not contain whitespace, `<`, `>` or `"`, so neither value can add another header entry.
- **🔤 Automatic Plain-Text Fallback:** Automatic HTML-to-plain-text conversion without manual duplication.
- **🔒 Outbound DLP Secret Scanner:** Proactive credential masking (whole-token `AKIA`/`ASIA` AWS access key IDs, passwords, API tokens, bearer tokens and PEM private-key blocks of any `<label>PRIVATE KEY` type, including OpenSSH, EC and encrypted keys) before emails leave your server.
- **📦 Async Background Worker Queues:** Native non-blocking dispatch via `rullst-core::queue`.
- **🧪 Explicit offline provider mode:** empty or `mock_*` credentials select `DeliveryMode::OfflineMock`, never perform network I/O, and are inspectable through `OfflineMailMock`.
- **🛠️ Safe CLI Scaffolding:** Generates registered, facade-based mailables for Welcome, Password Reset, OTP, Invoice, custom, evidence-aware NFS-e/international receipts, and explicit D+1/D+3/D+7 dunning; validates names, refuses collisions and escapes dynamic HTML.
- **🧾 Payment-Bound PDF Delivery:** The opt-in `capital-invoice` bridge accepts
  only Capital's final evidence-bound `PaidInvoice`, attaches bounded HTML/PDF,
  applies pre-flight and preserves a stable key for the application outbox.

---

## 🚀 Quickstart

### 1. Composing and Sending an Email

```rust
use rullst_mail::{Mail, Message};
use chrono::{Utc, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let logo_bytes = include_bytes!("../assets/logo.png");

    let message = Message::new()
        .to("alice@example.com")
        .from("noreply@rullst.dev")
        .subject("Welcome to Rullst!")
        .html(r#"
            <h1>Welcome, Alice!</h1>
            <p>Thanks for joining our platform.</p>
            <img src="cid:app_logo" alt="Logo" />
        "#)
        .attach_cid("app_logo", "logo.png", logo_bytes.to_vec(), "image/png")
        .attach_bytes("welcome_guide.pdf", b"%PDF-1.4...".to_vec(), "application/pdf")
        .send_in(std::time::Duration::from_secs(60)) // Deliver in 1 minute
        .unsubscribe_url("https://rullst.dev/unsub/alice");

    // The mandatory pipeline validates and sanitizes before queueing or delivery.
Mail::send(message).await?;

    Ok(())
}
```

For a schedule that survives process restarts, operate a built-in queue and keep
its worker handle alive:

```rust,no_run
use rullst_core::queue::{Queue, Worker};
use rullst_mail::{register_mail_handler, Mail, Message};

# async fn schedule() -> Result<(), Box<dyn std::error::Error>> {
let queue = Queue::sqlite("sqlite://storage/jobs.db").await?;
let mut worker = Worker::new(&queue).poll_interval(100);
register_mail_handler(&mut worker);
let worker_handle = worker.run()?;

let message = Message::new()
    .to("alice@example.com")
    .subject("Scheduled update")
    .text("Delivered after the durable due time")
    .send_in(std::time::Duration::from_secs(60));
Mail::enqueue(&queue, message).await?;

// Keep `worker_handle` in application state; shut it down during graceful exit.
worker_handle.shutdown().await?;
# Ok(())
# }
```

Execution begins on the first worker poll after the UTC timestamp and remains
at-least-once. Queue scheduling does not promise exact wall-clock execution,
exactly-once provider delivery, or provider acceptance. Redis promotes
scheduled jobs by its server clock, so the worker accepts a claimed job whose
timestamp is at most 300 seconds ahead of the worker's own clock and fails a
claim that is earlier than that.

The queue has no handler-requested retry: any delivery error, including a
`Transient` or `RateLimited` provider failure and its `Retry-After`, marks the
mail job failed with the error text. Nothing is lost silently, but the job is
sent again only after `Queue::retry_failed_job`. Automate that for transient
failures, or deliver through an outbox with its own retry policy (as account
mail does), when provider blips must be retried without an operator.

Queued jobs store attachment bytes as one base64 string per attachment. Workers
still accept jobs written with the earlier integer-array encoding, but an older
worker cannot read the base64 form, so upgrade workers before producers during
a rolling deployment.

---

### 2. Resilient Multi-Driver Failover (Circuit Breaker)

```rust
use rullst_mail::drivers::{FailoverDriver, PostmarkDriver, ResendDriver};
use std::time::Duration;

let primary = ResendDriver::try_new("re_...")?;
let fallback_1 = PostmarkDriver::try_new("pm_token_...")?;

let failover_driver = FailoverDriver::new(primary)
    .with_fallback(fallback_1)
    .with_threshold(3) // Trip circuit after 3 consecutive failures
    .with_cooldown(Duration::from_secs(60)); // Cooldown for 60s
```

---

### 3. Dynamic B2B Multi-Tenancy Routing

```rust
use rullst_core::security::TenantMembership;
use rullst_mail::{ResendDriver, TenantMailResolver};

let resolver = TenantMailResolver::new();
let membership = TenantMembership::try_new(["tenant_globex"])?;
let context = membership.select("tenant_globex")?;

// Register tenant-specific API credentials during application configuration.
resolver.register_for_context(
    &context,
    ResendDriver::try_new("re_globex...")?,
)?;

// The context must be derived from trusted authentication/membership state.
resolver.send_for_context(&context, &message).await?;
```

The registry is intentionally process-local. Durable encrypted credential storage,
rotation, and distribution between instances remain application/deployment concerns.

---

### 4. Fast Unit & Integration Testing with `MailTrap`

```rust
use rullst_mail::{Mail, MailTrap, Message};

#[tokio::test]
async fn test_user_registration_email() {
    Mail::set_driver(Box::new(MailTrap::driver()));
    MailTrap::clear();

    let msg = Message::new()
        .to("alice@example.com")
        .subject("Welcome to Rullst!")
        .html("<p>Please verify your email address.</p>")
        .attach_bytes("terms.pdf", b"%PDF...".to_vec(), "application/pdf")
        .unsubscribe_url("https://example.com/unsub/alice");

    Mail::send_now(msg).await.unwrap();

    // Fluent assertions
    MailTrap::assert_sent_to("alice@example.com")
        .with_subject("Welcome to Rullst!")
        .with_body_contains("Please verify your email")
        .with_attachment_count(1)
        .with_attachment_named("terms.pdf")
        .with_unsubscribe_url("https://example.com/unsub/alice");
}
```

---

### 5. Scaffolding Mailables with CLI

```bash
# Generate Welcome & Onboarding email
cargo rullst make:mail WelcomeEmail --welcome

# Generate Time-limited Password Reset email
cargo rullst make:mail PasswordReset --reset

# Generate Two-Factor OTP code email
cargo rullst make:mail OtpVerification --otp

# Generate SaaS Invoice receipt email
cargo rullst make:mail InvoiceReceipt --invoice

# Generate an NFS-e/international receipt whose mock provenance stays visible
cargo rullst make:mail-invoice

# Generate the explicit D+1/D+3/D+7 payment-recovery sequence
cargo rullst make:mail-dunning
```

`make:mail-invoice` enables `mailer` and `capital`. Its generated
`from_nfse_response` constructor accepts the typed Capital response and renders
`OfflineMock` only as `[PREVIEW — NOT AUTHORIZED]`; it never converts local DPS
or XMLDSig validity into tax authorization. `make:mail-dunning` exposes three
explicit stages, while due-date calculation, scheduling, entitlement changes,
and account state remain application responsibilities. Both templates execute
the mandatory pre-flight while building and fail on unsafe links.

Generated mailables set no `from`; configure the default sender with
`MAIL_FROM` (or `from` under `[mail]` in `Rullst.toml`), which new projects
list in `.env.example`. Staging and production must also select a driver.

For a payment-bound native PDF rather than the scaffolded fiscal template,
enable `rullst-mail/capital-invoice` (or umbrella `rullst/capital-mail`) and use
`PaidInvoiceDelivery::prepare`, then set the verified sender with
`.from(sender)?` (13.0), which re-runs pre-flight. It rejects non-final/mock
payment evidence and recipient/amount/currency substitution before sending.
The host must atomically claim its stable delivery key in durable state;
webhook orchestration, provider acceptance and exactly-once delivery are not
implied.

---

### 6. Pre-Flight Deliverability & Disposable Email Filtering

Prevent sender quota waste and fake user signups with built-in deliverability checks and blocked temporary domains:

```rust
use rullst_mail::{is_disposable_email, validate_email_deliverability, Message};

// 1. Direct email address validation
assert!(validate_email_deliverability("user@company.com").is_ok());
assert!(is_disposable_email("spammer@mailinator.com"));

// 2. Pre-flight check before dispatching
let msg = Message::new()
    .to("user@mailinator.com")
    .subject("Welcome!");

if msg.is_disposable() {
    eprintln!("Blocked disposable email address!");
}
```

---

### 7. Composing Inspection, Suppression, and Observability

The wrappers use static dispatch and may be composed around any `MailDriver`.
Enable `rullst-mail/sqlite` (or umbrella `rullst/mail-sqlite`) for the durable
local suppression store:

```rust,no_run
use rullst_mail::{
    AttachmentInspectionGuard, BoundedMailObserver, LocalAttachmentInspector,
    MailDriver, MemoryDriver, Message, MutableSuppressionStore,
    ObservedMailDriver, SqliteSuppressionStore, SuppressionEvent,
    SuppressionGuard, SuppressionReason,
};
use std::time::{SystemTime, UNIX_EPOCH};

# async fn deliver() -> Result<(), Box<dyn std::error::Error>> {
let store = SqliteSuppressionStore::connect(
    "sqlite://storage/mail-suppressions.sqlite",
    100_000,
    500_000,
).await?;

// Only record events after the provider-specific signature was authenticated.
let observed_at = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
store.record(SuppressionEvent::try_new(
    "postmark",
    "verified-event-id",
    "blocked@example.com",
    SuppressionReason::HardBounce,
    observed_at,
)?).await?;

let (transport, _) = MemoryDriver::isolated();
let inspected = AttachmentInspectionGuard::new(
    transport,
    LocalAttachmentInspector::strict(),
);
let suppressed = SuppressionGuard::new(inspected, store);
let observer = BoundedMailObserver::new(10_000)?;
let driver = ObservedMailDriver::try_new("memory", suppressed, observer)?;
driver.send(&Message::new()
    .to("recipient@example.com")
    .subject("Bounded delivery")
    .text("Hello"))
    .await?;
# Ok(())
# }
```

`SuppressionEvent` does not verify a webhook signature: a provider-specific
adapter must authenticate the exact event first. SQLite is shared durable local
state, not multi-host replication or encrypted storage. Keep replay IDs at least
as long as every provider's redelivery window. The local attachment inspector is
a bounded heuristic, not antivirus, sandbox execution, recursive archive
inspection or content disarm. The default observer is bounded and process-local;
the host owns any external metrics/tracing sink, retention and alerts.

---

### 8. Authenticated Open/Click Tracking Primitives

Generate versioned, purpose-bound HMAC-SHA256 tracking tokens with a mandatory
32-byte secret and bounded validity. HMAC authenticates but does not encrypt:
the current token payload contains the recipient address and destination URL in
base64-readable form. Applications must decide whether to use tracking at all
and own consent, minimization, retention, redirects and applicable law.

Click tracking rewrites only the double-quoted `href` of `<a>` elements, so a
`<link>` stylesheet or `<base>` fetched when a message is opened never registers
as a click. The token signs the destination with HTML character references
decoded (`?a=1&amp;b=2` is redirected as `?a=1&b=2`), and the tracker base is
escaped for the attribute it enters. A destination that the mandatory pipeline
would reject (homograph host) or redact (credentials in the URL) is not wrapped,
so that pipeline still rejects or redacts it.

```rust
use rullst_mail::{TrackingEngine, TrackingVerifier, PIXEL_1X1_GIF, Message};
use std::time::Duration;

let secret = b"replace-with-32-or-more-random-key-bytes";

// Fluent open & click tracking injection
let tracked_msg = Message::new()
    .to("user@example.com")
    .subject("Monthly Newsletter")
    .html("<p>Check out our <a href=\"https://rullst.dev/pricing\">pricing</a>.</p>")
    .try_with_open_tracking("https://app.com", secret, "campaign_2026")?
    .try_with_click_tracking("https://app.com", secret)?;

// Default verification enforces a 30-day TTL.
let event = TrackingEngine::verify_open_token(secret, &token)?;
println!("Email opened by {} for campaign {}", event.email, event.campaign_id);

// Endpoints needing single-consumption semantics can reject replay explicitly.
let verifier = TrackingVerifier::new(Duration::from_hours(24), 100_000)?;
let event = verifier.verify_open_once(secret, &token, now_unix_seconds)?;
```

---

### 9. Transactional Test Fixtures with `MailFactory`

Quickly generate standard transactional emails for local preview and testing:

```rust
use rullst_mail::MailFactory;

let welcome_msg = MailFactory::fake_welcome("alice@example.com", "Alice", "My SaaS App");
let reset_msg = MailFactory::fake_password_reset("bob@example.com", "https://app.com/reset?token=xyz", 15);
let otp_msg = MailFactory::fake_otp("carol@example.com", "492015", 5);
let invoice_msg = MailFactory::fake_invoice("david@example.com", "INV-2026-001", 9900, "USD");
let alert_msg = MailFactory::fake_security_alert("eve@example.com", "Unrecognized Login", "198.51.100.1", "Chrome / macOS");
```

---

### 10. Native AWS SES v2 with SigV4

Enable the opt-in official SDK transport:

```toml
[dependencies]
rullst-mail = { version = "12.1.0", features = ["aws-ses"] }
aws-config = "1.11"
```

`MAIL_DRIVER=ses` selects native mode when both `AWS_ACCESS_KEY_ID` and
`AWS_SECRET_ACCESS_KEY` exist. `AWS_SESSION_TOKEN` is accepted for temporary
credentials. Without those variables, the existing empty/`mock_*` token rule
selects the offline fixture; a real `AWS_SES_BEARER_TOKEN` is usable only with
an explicit trusted proxy URL. The proxy receives the SES v2 `SendEmail` JSON
shape, including every attachment and inline CID asset as
`Content.Simple.Attachments` (Base64 `RawContent`, `FileName`, `ContentType`,
`ContentDisposition` and `ContentId`). It is checked against the same SES field
limits and 40 MiB encoded estimate as native mode before any request, and
failures return `MailError::ValidationError`. The proxy must forward
attachments or reject the request; Rullst never drops them.

Long-running services should inject a refreshing credential provider or a
caller-built SDK config instead of freezing credentials:

```rust,no_run
use rullst_mail::{AwsSesDriver, MailDriver, Message, aws_ses_sdk};

# async fn deliver() -> Result<(), Box<dyn std::error::Error>> {
let shared = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
let config = aws_ses_sdk::Config::new(&shared);
let driver = AwsSesDriver::from_native_config(config)?;
driver.send(&Message::new()
    .to("recipient@example.com")
    .from("verified@example.com")
    .subject("Signed by AWS SigV4")
    .text("Hello from Rullst"))
    .await?;
# Ok(())
# }
```

The application still owns AWS identity/domain verification, sandbox exit, IAM
least privilege, quotas, reputation, bounce/complaint handling and monitoring.
A successful `MessageId` is provider acceptance, not proof of inbox delivery.
The native adapter rejects SES field limits and an encoded message estimate
over 40 MiB before network I/O; provider 429 responses preserve a bounded
delta-seconds `Retry-After` for failover/retry policy.

---

## Native providers added in 12.1

| Provider | Constructor | `MAIL_DRIVER` | Credentials |
|---|---|---|---|
| SendPulse | `SendPulseDriver::try_new(api_key)` | `sendpulse` | `SENDPULSE_API_KEY` |
| Mailjet | `MailjetDriver::try_new(api_key, secret_key)` | `mailjet` | `MAILJET_API_KEY`, `MAILJET_SECRET_KEY` |
| Mailjet remote validation | Same constructor plus `.with_sandbox()` | `mailjet-sandbox` | Same credential pair |
| Mailtrap sending | `MailtrapDriver::try_new(token)` | `mailtrap` | `MAILTRAP_API_TOKEN` |
| Mailtrap hosted capture | `MailtrapDriver::sandbox(token, id)` | `mailtrap-sandbox` | Token plus positive `MAILTRAP_SANDBOX_ID` |

No additional feature is required beyond Mail. All three reuse the mandatory
pipeline, bounded HTTPS client, typed failure/retry classification and empty or
`mock_*` offline fallback. Mailjet rejects a mixed real/offline credential pair.
Real delivery requires an explicit verified sender (`Message::from`). Direct
future delivery is rejected; use a durable queue for scheduling. Provider
acceptance is not inbox delivery and retries remain at-least-once.

SendPulse uses its current static Bearer API-key authentication; an OAuth client
ID/secret is not an API key. The SMTP service must be activated in the account.
HTML and binary attachments use the provider's Base64 contract. Inline CID and
unsubscribe headers are rejected by this bounded REST adapter rather than
silently omitted; use SMTP for those message shapes. Disable security-mail
tracking in the provider account. See [authentication](https://sendpulse.com/integrations/api)
and the [transactional API](https://sendpulse.com/integrations/api/smtp).

Mailjet uses [Send API v3.1](https://dev.mailjet.com/docs/email-api/send-api-v31/send-basic-email),
HTTP Basic credentials, per-message success validation, attachments and inline
CID. It sets open/click tracking to `disabled`. Remote sandbox mode validates
with the provider without sending; it is distinct from the no-network mock.

Mailtrap supports the [Sending API](https://docs.mailtrap.io/developers/email-sending)
and a separately selected [Sandbox API](https://docs.mailtrap.io/developers/email-sandbox/send-test-emails).
It includes attachments, inline CID and unsubscribe headers. Sending never
silently falls back to sandbox. Disable tracking at the domain/account level
for security notices. The older `MailTrap` type remains only Rullst's local test
harness; `MailtrapDriver` is the hosted provider integration.

Protocol/offline tests cover these contracts. Actual account/domain activation,
deliverability, limits and live acceptance remain deployment work. Free-plan
quotas can change; consult the providers rather than relying on SDK constants.

## ⚙️ Configuration (`Rullst.toml` or Environment Variables)

```toml
[mail]
driver = "resend" # "log" | "memory" | "smtp" | "resend" | "sendgrid" | "postmark" | "ses"
from = "Acme <no-reply@acme.example>" # default sender for messages without `from`
```

Environment variables:
- `MAIL_DRIVER`: Select active driver (`log`, `memory`, `smtp`, `resend`, `sendgrid`, `postmark`, `ses`, `azure-acs`, `sendpulse`, `mailjet`, `mailjet-sandbox`,
  `mailtrap`, `mailtrap-sandbox`). When neither it nor `[mail] driver` is set,
  development and test fall back to `log`, while staging and production
  (`RULLST_ENV`, `APP_ENV` or `[app] env`) return `MailError::ConfigError`
  instead of logging mail that is never delivered. Select `log` explicitly to
  keep metadata-only logging there.
- `MAIL_FROM`: Default sender (v13) for `Mail` facade messages that set no
  `from`, such as generated mailables. It takes precedence over `[mail] from`;
  an explicit `from` on the message always wins. Use one address or
  `Name <address>` that your provider account has verified. An invalid value
  fails every facade send with `MailError::ConfigError`; call
  `Mail::default_sender()` at startup to fail fast. Drivers used directly do
  not read it.
- `RESEND_API_KEY`: API key for Resend.
- `SENDGRID_API_KEY`: API key for SendGrid.
- `POSTMARK_SERVER_TOKEN`: Server API token for Postmark.
- `AWS_REGION`: Region used by native SigV4 signing or SES proxy/mock metadata.
- `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`: Select native SES when the
  `aws-ses` feature is enabled; both must be present.
- `AWS_SESSION_TOKEN`: Optional temporary-credential session token.
- `AWS_SES_BEARER_TOKEN`: Token for an explicitly configured trusted proxy;
  it is never sent to AWS as a substitute for SigV4.
- `AWS_SES_ENDPOINT`: Native SDK base endpoint or complete proxy send URL;
  HTTPS is required except for loopback integration tests.
- `MAIL_HOST`, `MAIL_PORT`, `MAIL_USERNAME`, `MAIL_PASSWORD`: SMTP credentials.
- `MAIL_LOG_PATH`: Path for log file (default: `storage/logs/mail.log`).

For Resend, SendGrid, Postmark, SendPulse, Mailjet, Mailtrap, ACS and the SES
fixture/proxy,
an empty credential or one beginning with `mock_` selects the deterministic
offline fallback. Use `driver.delivery_mode()` and
`OfflineMailMock::deliveries()` to assert this explicitly in tests. The
process-wide capture keeps only the newest 1,000 deliveries and at most 64 MiB
of their subject, body and attachment bytes, and the first capture in a process
logs a `mail.offline_mock.active` warning, because an empty production secret
also selects this fallback.

Every real transport needs a sender that the provider account has verified:
the message's `from`, or, for `Mail` facade sends, the `MAIL_FROM` /
`[mail] from` default. Without either, delivery fails with
`MailError::ConfigError` before any request, and the error names both
settings. Transports never invent a sender.

SMTP selects the offline fallback only explicitly: an empty or `mock_*`
`MAIL_HOST`, or a `mock_*` username or password. A real host without
credentials is an unauthenticated relay and receives real delivery, so
`MAIL_DRIVER=smtp` without `MAIL_HOST` sends to `127.0.0.1:25`. A username
without a password, or the reverse (blank values count as missing), returns
`MailError::ConfigError` instead of falling back to the mock.

Port 465 uses implicit TLS (SMTPS); every other port, including the facade's
default 25 and the usual submission port 587, requires STARTTLS. Plaintext is
never used: a server that does not offer STARTTLS, or presents a certificate
that is not valid for `MAIL_HOST`, fails with a typed transport error, so a
local relay needs a certificate valid for the configured host name.

---

## Scope boundary

`rullst-mail` is a transactional composition, dispatch and testing library. It
does not replace delivery providers, marketing CRMs, domain reputation,
bounce/complaint ingestion, consent management or an operational inbox. A
provider accepting a request is not proof that a message reached the inbox.

The security and deliverability checks are bounded heuristics: they help reject
known disposable domains, CRLF injection, selected dangerous schemes,
mixed-script host labels and recognized secret patterns. They do not parse every
valid/hostile HTML or MIME document and cannot guarantee delivery, absence of
phishing, absence of data leakage or legal compliance. The link checks read
`href` attributes however they are cased, spaced or quoted, decode HTML
character references first (so `javascript&colon;` and `&#x430;` hosts are
seen as a browser sees them) and report only the violated rule, never the link.
Other attributes such as `src` or `action` are not inspected.

Recipients are parsed once by the pre-flight pipeline. It accepts one bare
address, `<address>` or `Name <address>` (the name may be quoted), and hands the
bare address to suppression, the disposable-domain check and every transport,
so a display name is not delivered. Lists, groups, comments, quoted local parts,
domain literals and malformed brackets are rejected with
`MailError::ValidationError`. Suppression events and lookups use the same
parser; anything it rejects fails closed. Suppression keys compare the domain
case-insensitively and key an internationalized domain by its IDNA A-label
(`bücher.de` and `xn--bcher-kva.de` share one entry); a non-ASCII local part
cannot be keyed, so `SuppressionGuard` rejects it with `ValidationError` rather
than reporting `SuppressionUnavailable`. The optional `from` sender is parsed
with it as well and keeps its display name: Resend, Postmark, SES and SMTP send
it as written, SendGrid, Mailjet, Mailtrap and SendPulse receive the address
and name as separate fields, and ACS receives only the bare address.

The pre-flight pipeline rejects a subject over 2 KiB, or an HTML or plain-text
body over 2 MiB each, with `MailError::ValidationError` before any content scan.
Oversized content is rejected, never truncated. The link, homograph and
secret-redaction scans are single forward passes, so their cost grows linearly
with the bounded body. They still run on the calling task; bound user-supplied
text at the request edge as well.

The local inspector chooses its checks from the declared MIME type (compared
case-insensitively), the filename extension and the content signature
together. Both policies reject executable magic, executable or script-host
extensions (`.exe`, `.bat`, `.cmd`, `.ps1`, `.vbs`, `.js`, `.hta`, `.lnk`,
`.msc`, `.appref-ms`, `.settingcontent-ms`, `.jnlp`, `.vhd` and the other
executable/script types on Outlook's Level 1 blocked list), SVG by type,
extension or content, active PDF content (JavaScript, launch, embedded-file,
XFA form, rich-media, embedded go-to and data-import names) wherever a
`%PDF-` header appears in the first KiB, and a declared type that disagrees
with a known extension or signature. `strict()` also rejects HTML extensions,
HTML/script markup or `javascript:`/`vbscript:` URIs, unknown extensions, any
declared type it does not inspect other than `application/octet-stream` (so a
`text/html` attachment named `invoice.txt` is rejected), and opaque formats;
`allowing_opaque()` still accepts HTML and other opaque content. PDF names written with `#xx` escapes or inside compressed streams are
not decoded.

Attachment limits are 32 items, 20 MiB per item and 25 MiB of raw bytes in
aggregate before transport encoding. Provider/account limits can be lower. The
base pipeline validates metadata but treats bytes as opaque. The opt-in local
inspector recognizes only its documented bounded formats and heuristics; use a
production scanner adapter when malware, archive, sandbox or CDR policy is
required.

---

## 📚 Documentation & Roadmap

- Architecture & Master Roadmap: [`rullst-mail/ROADMAP.md`](https://github.com/Rullst/Rullst/blob/main/rullst-mail/ROADMAP.md)
- Official Documentation Book: [`docs/src/crates/mail.md`](https://github.com/Rullst/Rullst/blob/main/docs/src/crates/mail.md)
