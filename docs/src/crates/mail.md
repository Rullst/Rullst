# Rullst Mail 📬

`rullst-mail` is the enterprise-grade transactional email and mailables engine for the Rullst Framework. It provides a robust, zero-panic abstraction over popular delivery providers with built-in multi-driver circuit breaker failover, dynamic multi-tenancy routing, zero-copy attachments, inline CID assets, scheduled delivery, RFC 8058 compliance, DLP secret sanitization, and in-memory test assertions.

---

## ✨ Features

- **🛡️ Zero-Panic Guarantees:** 100% safe Rust with typed `MailError` and formal verification via Kani proofs.
- **⚡ 7 Production Delivery Drivers:**
  - **Resend** (`ResendDriver`) — Native REST API with scheduled delivery & RFC 8058.
  - **SendGrid** (`SendGridDriver`) — Native v3 REST API with personalization & attachments.
  - **Postmark** (`PostmarkDriver`) — High-deliverability transactional REST API with Message Streams.
  - **AWS SES v2** (`AwsSesDriver`) — Native REST API v2 with custom endpoints & region selection.
  - **Native SMTP** (`SmtpDriver`) — Pure async Lettre transport with TLS.
  - **Memory & MailTrap** (`MemoryDriver`, `MailTrap`) — Zero-I/O in-memory harness with fluent assertions.
  - **Log** (`LogDriver`) — Terminal and disk file logging (`storage/logs/mail.log`).
- **🔀 Multi-Driver Circuit Breaker & Automatic Failover (`FailoverDriver`):** Primary driver dispatch with automatic fallback across secondary drivers, atomic failure threshold triggering, cooldown circuit breakers, and structured tracing warnings.
- **🏢 Dynamic Multi-Tenancy Resolver (`TenantMailResolver`):** Isolate credentials, custom domains, and dedicated API keys per tenant/organization in B2B SaaS applications.
- **📎 Zero-Copy Attachments & Inline CID Assets:** Fluent API for raw bytes, files, and Content-ID (`CID`) inline image embedding (`<img src="cid:logo">`) with Base64 encoding.
- **⏰ Precision Scheduled Delivery (`.send_at()`, `.send_in()`):** Deliver messages at exact UTC timestamps or relative durations.
- **🕵️ Outbound Phishing & Homograph URL Interceptor (`.validate_security()`):** Pre-flight detection of mixed-script Unicode IDN spoofed domains (`pаypal.com` with Cyrillic characters) and dangerous URI schemes (`javascript:`, `data:text/html`).
- **📜 RFC 8058 One-Click List-Unsubscribe:** Automatic compliant header injection (`List-Unsubscribe` and `List-Unsubscribe-Post: List-Unsubscribe=One-Click`).
- **🔤 Automatic Plain-Text Fallback:** Automatic HTML-to-plain-text conversion without manual duplication.
- **🔒 Outbound DLP Secret Scanner:** Proactive credential masking (AWS keys, passwords, API tokens, bearer tokens) before emails leave your server.
- **📦 Async Background Worker Queues:** Native non-blocking dispatch via `rullst-core::queue`.
- **🛠️ CLI Scaffolding (`cargo rullst make:mail`):** Instant boilerplate generation for Welcome, Password Reset, OTP, and Invoice mailables.

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
        .unsubscribe_url("https://rullst.dev/unsub/alice")
        .sanitize_secrets();

    // Validate links against homograph spoofing
    message.validate_security()?;

    // Sends asynchronously via background queue or active driver
    Mail::send(message).await?;

    Ok(())
}
```

---

### 2. Resilient Multi-Driver Failover (Circuit Breaker)

```rust
use rullst_mail::drivers::{AwsSesDriver, FailoverDriver, PostmarkDriver, ResendDriver};
use std::sync::Arc;
use std::time::Duration;

let primary = Arc::new(ResendDriver { api_key: "re_...".into() });
let fallback_1 = Arc::new(PostmarkDriver::new("pm_token_..."));
let fallback_2 = Arc::new(AwsSesDriver::new("us-east-1", "ses_token_..."));

let failover_driver = FailoverDriver::new(primary, vec![fallback_1, fallback_2])
    .with_threshold(3) // Trip circuit after 3 consecutive failures
    .with_cooldown(Duration::from_secs(60)); // Cooldown for 60s
```

---

### 3. Dynamic B2B Multi-Tenancy Routing

```rust
use rullst_mail::resolver::TenantMailResolver;
use rullst_mail::drivers::{ResendDriver, SmtpDriver};
use std::sync::Arc;

let resolver = TenantMailResolver::new()
    .with_default_driver(Arc::new(ResendDriver { api_key: "re_global...".into() }));

// Register tenant-specific dedicated SMTP or API keys
resolver.register("tenant_acme", Arc::new(SmtpDriver { /* Acme SMTP */ }));
resolver.register("tenant_globex", Arc::new(ResendDriver { api_key: "re_globex...".into() }));

// Dispatches using the tenant's isolated credentials
resolver.send_for_tenant("tenant_acme", &message).await?;
```

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
```

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

### 7. Zero-Cookie Privacy-Preserving Tracking Engine

Generate HMAC-SHA256 signed tracking tokens and transparent 1x1 GIF pixels without third-party cookies (GDPR & LGPD compliant):

```rust
use rullst_mail::{TrackingEngine, PIXEL_1X1_GIF, Message};

let secret = b"my-master-cryptographic-secret";

// Fluent open & click tracking injection
let tracked_msg = Message::new()
    .to("user@example.com")
    .subject("Monthly Newsletter")
    .html("<p>Check out our <a href=\"https://rullst.dev/pricing\">pricing</a>.</p>")
    .with_open_tracking("https://app.com", secret, "campaign_2026")
    .with_click_tracking("https://app.com", secret);

// Verifying tracking events in your web route handler
let event = TrackingEngine::verify_open_token(secret, &token)?;
println!("Email opened by {} for campaign {}", event.email, event.campaign_id);
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
- `AWS_REGION`: AWS region for SES (e.g. `us-east-1`).
- `AWS_SES_BEARER_TOKEN`: Auth token for AWS SES REST v2.
- `MAIL_HOST`, `MAIL_PORT`, `MAIL_USERNAME`, `MAIL_PASSWORD`: SMTP credentials.
- `MAIL_LOG_PATH`: Path for log file (default: `storage/logs/mail.log`).

---

## ⚖️ Rullst Mail vs Marketing Automation Platforms (RD Station / Mailchimp)

A common architectural question when building web applications and SaaS platforms is: **"Does `rullst-mail` replace services like RD Station or Mailchimp?"**

The answer depends on the use case. `rullst-mail` is designed as a **high-throughput, sovereign, transactional email & backend delivery engine**, replacing the need for expensive third-party transactional tiers while allowing seamless optional integration with marketing automation tools.

### ⚔️ What `rullst-mail` Replaces Directly:
1. **Transactional Email Services**: Replaces **Resend**, **SendGrid Transactional**, **Postmark**, **AWS SES SDKs**, **Mailgun**, and **Mailtrap**.
2. **Backend Notification & Dunning Workflows**:
   - Authentication ceremonies (account activation, password reset, 2FA/OTP tokens).
   - Real-time security alerts and system events.
   - SaaS invoices, payment receipts (Pix, Credit Card via `rullst-capital`), and Brazilian NFS-e DPS tax receipts with zero-copy PDF/XML attachments.
   - **AI Smart Dunning**: Empathetic sales recovery and automated dunning sequences powered by `rullst-ai` and `rullst-capital`.
3. **Local Testing Environments**: Eliminates paid email sandbox subscriptions by providing a zero-I/O in-memory `MailTrap` with visual inspection in Rullst Studio (`/studio/mail`).
4. **B2B SaaS Multi-Tenancy**: Lets every tenant organization configure their own isolated custom domains, SMTP servers, or API keys (`TenantMailResolver`).

---

### 📊 Comparative Matrix: `rullst-mail` vs RD Station / Mailchimp

| Feature / Capability | `rullst-mail` (Native Framework Engine) | RD Station / Mailchimp / ActiveCampaign |
| :--- | :---: | :---: |
| **Transactional Emails (Password reset, 2FA, receipts)** | ✅ **Native, sub-millisecond, zero-markup** | ❌ Expensive add-on or restricted |
| **Multi-Driver Delivery with Automatic Failover** | ✅ **Yes (`FailoverDriver` with Circuit Breaker)** | ❌ Vendor-locked to proprietary IP pools |
| **Zero-Copy Attachments & Inline CID Assets** | ✅ **Yes (High-throughput pure Rust)** | ⚠️ Heavily capped file sizes |
| **Zero-Cookie Privacy Tracking** | ✅ **Native (`TrackingEngine` HMAC)** | ⚠️ Third-party cookie dependency |
| **Disposable Email & Deliverability Filter** | ✅ **Native (`DisposableEmailFilter`)** | ⚠️ Expensive external addons |
| **Security: Anti-Phishing & Homograph URL Scanner** | ✅ **Native pre-flight IDN inspection** | ⚠️ Basic link scanning |
| **Outbound DLP Secret Scanner (AWS tokens, keys)** | ✅ **Native (`redact_email_secrets`)** | ❌ No credential leak prevention |
| **Dynamic B2B SaaS Multi-Tenancy** | ✅ **Yes (Dedicated credentials per tenant)** | ❌ Single-account flat tenancy |
| **Scheduled Delivery (`.send_at()`, `.send_in()`)** | ✅ **Yes (Native UTC & relative delays)** | ✅ Yes |
| **In-Memory MailTrap & Test Fixtures** | ✅ **Native (`MailTrap` & `MailFactory`)** | ⚠️ Manual sandbox configuration |
| **Code-Driven Automated Sequences & Dunning** | ✅ **Yes (Tokio background queue integration)** | ✅ Yes |
| **Live Studio Web Inspector (`/studio/mail`)** | ⏳ *(Not implemented yet - In Roadmap)* | ⚠️ Proprietary dashboard |
| **AI Smart Dunning Revenue Recovery** | ⏳ *(Not implemented yet - In Roadmap)* | ⚠️ Rule-based workflows only |
| **Drag-and-Drop No-Code Visual Builder** | ❌ *(Code/Template-first: HTML, Jinja2, Tailwind)* | ✅ **Yes (Visual WYSIWYG for marketers)** |
| **No-Code Landing Pages & Commercial Sales CRM** | ❌ *(Built with `rullst-core` / `rullst-nexus`)* | ✅ **Yes (Integrated lead scoring CRM)** |

---

### 💡 Architectural Recommendation

* **Use `rullst-mail` if:** You are building a SaaS, web app, or API that needs to send transactional emails, alerts, invoices, smart dunning sequences, or if you want to build your own internal email automation engine with zero third-party subscription markup.
* **Coexist with RD Station / Mailchimp if:** Non-technical marketing teams require a standalone drag-and-drop WYSIWYG newsletter builder, visual landing page creators, and a commercial lead-scoring CRM. In this setup, `rullst-mail` powers the core application while marketing platforms integrate via webhooks using `rullst-connect`.
