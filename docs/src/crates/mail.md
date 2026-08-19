# Rullst Mail ✉️

`rullst-mail` is the high-performance transactional email and mailables engine for the Rullst Framework. It provides a robust, zero-panic abstraction over popular delivery providers with built-in RFC 8058 compliance, DLP secret sanitization, and in-memory test assertions.

## ✨ Features

- **🛡️ Zero-Panic Guarantees:** 100% safe Rust with typed `MailError` and formal verification via Kani proofs.
- **⚡ Multiple Delivery Drivers:** Built-in support for **Log**, **Memory**, **SMTP**, **Resend**, and **SendGrid**.
- **📜 RFC 8058 One-Click List-Unsubscribe:** Automatic compliant header injection (`List-Unsubscribe` and `List-Unsubscribe-Post: List-Unsubscribe=One-Click`).
- **🔤 Automatic Plain-Text Fallback:** Automatic HTML-to-plain-text conversion without manual duplication.
- **🔒 Outbound DLP Secret Scanner:** Proactive credential masking (AWS keys, passwords, API tokens, bearer tokens) before emails leave your server.
- **🧪 In-Memory `MailTrap` & Fluent Assertions:** Zero-I/O test harness for lightning-fast tests with rich assertion helpers.
- **📦 Async Background Worker Queues:** Native non-blocking dispatch via `rullst-core::queue`.
- **🛠️ CLI Scaffolding (`cargo rullst make:mail`):** Instant boilerplate generation for Welcome, Password Reset, OTP, and Invoice mailables.

## 🚀 Quickstart

### 1. Composing and Sending an Email

```rust
use rullst_mail::{Mail, Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let message = Message::new()
        .to("alice@example.com")
        .from("noreply@rullst.dev")
        .subject("Welcome to Rullst!")
        .html("<h1>Welcome, Alice!</h1><p>Thanks for joining our platform.</p>")
        .unsubscribe_url("https://rullst.dev/unsub/alice")
        .sanitize_secrets();

    // Sends asynchronously via background queue or active driver
    Mail::send(message).await?;

    Ok(())
}
```

### 2. Fast Unit & Integration Testing with `MailTrap`

```rust
use rullst_mail::{Mail, MailTrap, Message};

#[tokio::test]
async fn test_user_registration_email() {
    // Intercept all outgoing emails in memory
    Mail::set_driver(Box::new(MailTrap::driver()));
    MailTrap::clear();

    // Trigger application flow
    let msg = Message::new()
        .to("alice@example.com")
        .subject("Welcome to Rullst!")
        .html("<p>Please verify your email address.</p>")
        .unsubscribe_url("https://example.com/unsub/alice");

    Mail::send_now(msg).await.unwrap();

    // Fluent assertions
    MailTrap::assert_sent_to("alice@example.com")
        .with_subject("Welcome to Rullst!")
        .with_body_contains("Please verify your email")
        .with_unsubscribe_url("https://example.com/unsub/alice");
}
```

### 3. Scaffolding Mailables with CLI

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

## ⚙️ Configuration (`Rullst.toml` or Environment Variables)

```toml
[mail]
driver = "resend" # "log" | "memory" | "smtp" | "resend" | "sendgrid"
```

Environment variables:
- `MAIL_DRIVER`: Select active driver (`log`, `memory`, `smtp`, `resend`, `sendgrid`).
- `RESEND_API_KEY`: API key for Resend.
- `SENDGRID_API_KEY`: API key for SendGrid.
- `MAIL_HOST`, `MAIL_PORT`, `MAIL_USERNAME`, `MAIL_PASSWORD`: SMTP credentials.
- `MAIL_LOG_PATH`: Path for log file (default: `storage/logs/mail.log`).
