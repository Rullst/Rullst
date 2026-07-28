# Rullst Mail

`rullst-mail` is the core mail delivery system for the Rullst Framework. It provides a clean, async, and type-safe API for sending transactional emails, managing templates, and handling SMTP configuration.

## Features
- **Async by Default:** Powered by Tokio.
- **Template Rendering:** First-class support for HTML templates.
- **SMTP Pooling:** Connection pooling for high-throughput dispatch.
- **Queueing:** Out-of-the-box integration with `rullst-core` jobs for background mailing.

## Quick Start

```rust
use rullst_mail::{Mailer, Message};

async fn send_welcome(email: &str) {
    let mailer = Mailer::new_from_env();
    let msg = Message::builder()
        .to(email)
        .subject("Welcome to Rullst!")
        .html("<h1>Hello!</h1><p>Welcome aboard!</p>")
        .build();

    mailer.send(msg).await.unwrap();
}
```
