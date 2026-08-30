# Rullst Mail 📬

> **Vision preserved:** additional providers and air-gapped/zero-leak ambitions
> were not silently removed; see their status and recommendation in the
> [capability ledger](../capability-ledger.md#ai-and-mail).

`rullst-mail` is Rullst's transactional email and mailables engine. Official dispatch paths pass through a pre-flight pipeline for CRLF protection, recipient checks, content security scanning, and DLP sanitization before queueing or transport delivery.

---

## ✨ Features

- **🛡️ Typed failures:** production delivery paths return `MailError`; malformed messages and provider configuration fail closed. CI and formal checks remain scoped evidence, not an absolute guarantee.
- **⚡ Delivery and Test Drivers:**
  - **Resend** (`ResendDriver`) — Native REST API with scheduled delivery & RFC 8058.
  - **SendGrid** (`SendGridDriver`) — Native v3 REST API with personalization & attachments.
  - **Postmark** (`PostmarkDriver`) — High-deliverability transactional REST API with Message Streams.
  - **AWS SES boundary** (`AwsSesDriver`) — offline fixture or explicit trusted bearer proxy; direct SES fails closed until SigV4 exists.
  - **Native SMTP** (`SmtpDriver`) — Pure async Lettre transport with TLS.
  - **Memory & MailTrap** (`MemoryDriver`, `MailTrap`) — Zero-I/O in-memory harness with fluent assertions.
  - **Log** (`LogDriver`) — Terminal and disk file logging (`storage/logs/mail.log`).
- **🔀 Multi-Driver Circuit Breaker & Automatic Failover (`FailoverDriver`):** Primary driver dispatch with automatic fallback across secondary drivers, atomic failure threshold triggering, cooldown circuit breakers, and structured tracing warnings.
- **🏢 Auth-bound Multi-Tenancy Resolver (`TenantMailResolver`):** Select isolated in-process drivers directly from a trusted Core `TenantContext`; registry failures and invalid IDs fail closed.
- **📎 Attachments & Inline CID Assets:** Fluent API for raw bytes, files, and Content-ID (`CID`) inline images; transports may copy and Base64-encode payloads.
- **⏰ Provider-specific Scheduling (`.send_at()`, `.send_in()`):** Resend and SendGrid receive scheduling fields; generic queue/SMTP/Postmark due-time dispatch is not implied.
- **🕵️ Outbound Phishing & Homograph URL Interceptor (`.validate_security()`):** Pre-flight detection of mixed-script Unicode IDN spoofed domains (`pаypal.com` with Cyrillic characters) and dangerous URI schemes (`javascript:`, `data:text/html`).
- **📜 RFC 8058 One-Click List-Unsubscribe:** Automatic compliant header injection (`List-Unsubscribe` and `List-Unsubscribe-Post: List-Unsubscribe=One-Click`).
- **🔤 Automatic Plain-Text Fallback:** Automatic HTML-to-plain-text conversion without manual duplication.
- **🔒 Outbound DLP Secret Scanner:** Proactive credential masking (AWS keys, passwords, API tokens, bearer tokens) before emails leave your server.
- **📦 Async Background Worker Queues:** Native non-blocking dispatch via `rullst-core::queue`.
- **🧪 Explicit offline provider mode:** empty or `mock_*` credentials select `DeliveryMode::OfflineMock`, never perform network I/O, and are inspectable through `OfflineMailMock`.
- **🛠️ Safe CLI Scaffolding:** Generates registered facade-based Welcome, Password Reset, OTP, Invoice, custom, evidence-aware NFS-e/international receipt, and explicit D+1/D+3/D+7 dunning mailables, refusing unsafe names/collisions and escaping dynamic HTML.

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

# Generate an evidence-aware NFS-e/international receipt
cargo rullst make:mail-invoice

# Generate explicit D+1/D+3/D+7 payment-recovery stages
cargo rullst make:mail-dunning
```

The fiscal template enables the umbrella `capital` feature and accepts a typed
`FiscalResponse`: an `OfflineMock` always renders as `[PREVIEW — NOT
AUTHORIZED]`. The dunning template does not infer due dates, schedule itself,
or mutate access. Both generated `build` paths run the mandatory mail pre-flight
and reject unsafe links; tax provenance, billing state, scheduling, and policy
remain application-owned.

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

### 7. Authenticated open/click tracking primitives

Generate versioned, purpose-bound HMAC-SHA256 tracking tokens with a mandatory
32-byte secret and bounded validity. HMAC authenticates but does not encrypt:
recipient and target URL remain base64-readable in the current token. The
application owns consent, minimization, retention, redirects and applicable
privacy-law decisions.

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
let verifier = TrackingVerifier::new(Duration::from_secs(24 * 60 * 60), 100_000)?;
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
- `AWS_SES_BEARER_TOKEN`: Bearer token for an explicit trusted proxy; this is
  not AWS SigV4.
- `AWS_SES_ENDPOINT`: Required HTTPS proxy endpoint for non-mock mode; HTTP is
  accepted only on loopback for local tests.
- `MAIL_HOST`, `MAIL_PORT`, `MAIL_USERNAME`, `MAIL_PASSWORD`: SMTP credentials.
- `MAIL_LOG_PATH`: Path for log file (default: `storage/logs/mail.log`).

For Resend, SendGrid, Postmark, the SES proxy fixture, and authenticated SMTP,
an empty credential or one beginning with `mock_` selects the deterministic
offline fallback. Use `driver.delivery_mode()` and
`OfflineMailMock::deliveries()` to assert this explicitly in tests.

---

## Scope and product boundaries

`rullst-mail` is a transactional-delivery library, not a marketing CRM or a
claim that third-party delivery infrastructure is unnecessary.

Implemented building blocks include:

- typed message construction and escaped generated templates;
- explicit SMTP, Resend, SendGrid, Postmark, log and memory drivers, plus the
  bounded SES proxy/mock adapter;
- deterministic offline mode for empty or `mock_*` provider credentials;
- an in-memory `MailTrap` and `MailFactory` fixtures;
- bounded retry/failover helpers, tenant-driver resolution, attachments, and
  provider-specific scheduling fields;
- HMAC-authenticated (not encrypted) tracking tokens with expiry/replay helpers,
  URL checks, and bounded secret-redaction heuristics.

These components do not provide deliverability, sender-domain reputation,
legal consent, unsubscribe policy, durable campaign orchestration, a visual
marketing editor, or a production inbox. Provider acceptance is not proof of
delivery. Tracking pixels/links have privacy and consent implications that the
application must evaluate for each jurisdiction and use case.

Choose a delivery provider and operational policy based on measured volume,
region, data processing terms, bounce/complaint handling, retention, cost, and
failover tests. Rullst publishes no universal latency, price, or feature
comparison against commercial platforms.
