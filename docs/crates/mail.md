# Rullst Mail ✉️

`rullst-mail` is the transactional email delivery module for the Rullst Framework. It provides a robust, zero-panic abstraction over popular email delivery providers, ensuring your transactional emails reach their destination securely and reliably.

## ✨ Features

- **Zero-Panic Guarantees:** 100% safe Rust. No unexpected crashes when building or sending emails.
- **Provider Agnostic:** Swap between AWS SES, Resend, SendGrid, Mailgun, and SMTP without changing your core application logic.
- **Template Rendering:** Native integration with `tinytemplate` for lightning-fast HTML email compilation.
- **Background Delivery:** Built-in integration with Rullst's background worker queues (Redis/Postgres) to prevent blocking your HTTP handlers.
- **Dry Run Mode:** Safe testing environment that logs emails instead of sending them.

## 🚀 Quickstart

Add `rullst-mail` to your project:

```bash
cargo add rullst-mail
```

### Sending an Email

Initialize the mailer with your preferred driver (e.g., Resend), render an HTML template, and dispatch it to the background queue:

```rust
use rullst_mail::{Mailer, driver::ResendDriver, Email};
use serde::Serialize;

#[derive(Serialize)]
struct WelcomeContext {
    name: String,
    activation_link: String,
}

#[tokio::main]
async fn main() {
    // 1. Initialize Driver
    let driver = ResendDriver::new("re_123456789");
    let mailer = Mailer::new(driver);

    // 2. Prepare Context
    let context = WelcomeContext {
        name: "Alice".to_string(),
        activation_link: "https://myapp.com/activate/123".to_string(),
    };

    // 3. Compose Email
    let email = Email::builder()
        .from("noreply@myapp.com")
        .to("alice@example.com")
        .subject("Welcome to Rullst!")
        .template("welcome_email.html")
        .context(context)
        .build()
        .expect("Failed to build email");

    // 4. Send (Async)
    mailer.send_async(email).await.expect("Failed to enqueue email");
}
```

## 🔐 Security Audit

`rullst-mail` strictly validates email addresses and template variables to prevent injection attacks (e.g., SMTP Header Injection). Network calls to providers are resilient, wrapped in timeout bounds, and properly propagate typed errors (`MailError`) upwards.

## 📚 Documentation

For advanced usage, configuring AWS SES, and setting up the background worker queue for heavy dispatching, please visit the **[Rullst Book](https://rullst.github.io/Rullst/book/index.html)**.
