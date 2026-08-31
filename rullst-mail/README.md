# Rullst Mail 📬

`rullst-mail` is Rullst's transactional email and mailables engine. Every official dispatch path now passes through one pre-flight pipeline for CRLF protection, recipient deliverability checks, content security scanning, and DLP sanitization before queueing or transport delivery.

---

## ✨ Features

- **🛡️ Typed failures:** production delivery paths return `MailError`; malformed messages and provider configuration fail closed.
- **⚡ Delivery and Test Drivers:**
  - **Resend** (`ResendDriver`) — Native REST API with scheduled delivery & RFC 8058.
  - **SendGrid** (`SendGridDriver`) — Native v3 REST API with personalization & attachments.
  - **Postmark** (`PostmarkDriver`) — High-deliverability transactional REST API with Message Streams.
  - **AWS SES boundary** (`AwsSesDriver`) — deterministic offline fixture or an explicit trusted bearer-authenticated proxy. Direct SES v2 fails closed until SigV4 is implemented.
  - **Native SMTP** (`SmtpDriver`) — Pure async Lettre transport with TLS.
  - **Memory & MailTrap** (`MemoryDriver`, `MailTrap`) — Zero-I/O in-memory harness with fluent assertions.
  - **Log** (`LogDriver`) — Terminal and disk file logging (`storage/logs/mail.log`).
- **🔀 Typed Circuit Breaker & Automatic Failover (`FailoverDriver`):** Fails over only for transport, HTTP 5xx, provider rate-limit, or transient SMTP failures; permanent message/configuration/provider rejection stays on the original error path. Structured tracing exposes bounded decision fields without provider bodies.
- **🏢 Auth-bound Multi-Tenancy Resolver (`TenantMailResolver`):** Select isolated in-process drivers directly from a trusted Core `TenantContext`; registry failures and invalid IDs fail closed.
- **📎 Attachments & Inline CID Assets:** Fluent API for raw bytes, files, and Content-ID (`CID`) inline images; transports may copy and Base64-encode payloads.
- **⏰ Durable Scheduling (`.send_at()`, `.send_in()`):** SQLite and Redis queues persist schedules for up to 366 days and never claim early; direct Resend/SendGrid delivery uses provider scheduling. Real SMTP, Postmark, Log and SES-proxy paths reject future direct delivery and must use a durable queue; offline fixtures may retain the timestamp for assertions.
- **🕵️ Outbound Phishing & Homograph URL Interceptor (`.validate_security()`):** Pre-flight detection of mixed-script Unicode IDN spoofed domains (`pаypal.com` with Cyrillic characters) and dangerous URI schemes (`javascript:`, `data:text/html`).
- **📜 RFC 8058 One-Click List-Unsubscribe:** Automatic compliant header injection (`List-Unsubscribe` and `List-Unsubscribe-Post: List-Unsubscribe=One-Click`).
- **🔤 Automatic Plain-Text Fallback:** Automatic HTML-to-plain-text conversion without manual duplication.
- **🔒 Outbound DLP Secret Scanner:** Proactive credential masking (AWS keys, passwords, API tokens, bearer tokens) before emails leave your server.
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
exactly-once provider delivery, or provider acceptance.

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

For a payment-bound native PDF rather than the scaffolded fiscal template,
enable `rullst-mail/capital-invoice` (or umbrella `rullst/capital-mail`) and use
`PaidInvoiceDelivery::prepare`. It rejects non-final/mock payment evidence and
recipient/amount/currency substitution before sending. The host must atomically
claim its stable delivery key in durable state; webhook orchestration,
provider acceptance and exactly-once delivery are not implied.

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

### 7. Authenticated Open/Click Tracking Primitives

Generate versioned, purpose-bound HMAC-SHA256 tracking tokens with a mandatory
32-byte secret and bounded validity. HMAC authenticates but does not encrypt:
the current token payload contains the recipient address and destination URL in
base64-readable form. Applications must decide whether to use tracking at all
and own consent, minimization, retention, redirects and applicable law.

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

### 8. Transactional Test Fixtures with `MailFactory`

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

## ⚙️ Configuration (`Rullst.toml` or Environment Variables)

```toml
[mail]
driver = "resend" # "log" | "memory" | "smtp" | "resend" | "sendgrid" | "postmark" | "ses"
```

Environment variables:
- `MAIL_DRIVER`: Select active driver (`log`, `memory`, `smtp`, `resend`, `sendgrid`, `postmark`, `ses`).
- `RESEND_API_KEY`: API key for Resend.
- `SENDGRID_API_KEY`: API key for SendGrid.
- `POSTMARK_SERVER_TOKEN`: Server API token for Postmark.
- `AWS_REGION`: Region metadata used by the SES proxy/mock adapter.
- `AWS_SES_BEARER_TOKEN`: Token for an explicitly configured trusted proxy;
  it is not an AWS SigV4 credential implementation.
- `AWS_SES_ENDPOINT`: Required HTTPS proxy endpoint for non-mock SES mode;
  loopback HTTP is allowed for local integration tests.
- `MAIL_HOST`, `MAIL_PORT`, `MAIL_USERNAME`, `MAIL_PASSWORD`: SMTP credentials.
- `MAIL_LOG_PATH`: Path for log file (default: `storage/logs/mail.log`).

For Resend, SendGrid, Postmark, the SES proxy fixture, and authenticated SMTP,
an empty credential or one beginning with `mock_` selects the deterministic
offline fallback. Use `driver.delivery_mode()` and
`OfflineMailMock::deliveries()` to assert this explicitly in tests.

---

## Scope boundary

`rullst-mail` is a transactional composition, dispatch and testing library. It
does not replace delivery providers, marketing CRMs, domain reputation,
bounce/complaint ingestion, consent management or an operational inbox. A
provider accepting a request is not proof that a message reached the inbox.

The security and deliverability checks are bounded heuristics: they help reject
known disposable domains, CRLF injection, selected dangerous schemes,
mixed-script domains and recognized secret patterns. They do not parse every
valid/hostile HTML or MIME document and cannot guarantee delivery, absence of
phishing, absence of data leakage or legal compliance.

---

## 📚 Documentation & Roadmap

- Architecture & Master Roadmap: [`rullst-mail/ROADMAP.md`](https://github.com/Rullst/Rullst/blob/main/rullst-mail/ROADMAP.md)
- Official Documentation Book: [`docs/src/crates/mail.md`](https://github.com/Rullst/Rullst/blob/main/docs/src/crates/mail.md)
