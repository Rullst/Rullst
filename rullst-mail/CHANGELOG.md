# Changelog — `rullst-mail`

All notable changes to the `rullst-mail` crate will be documented in this file.

## [12.0.0] - 2026-08-19

### 🚀 Added
- **RFC 8058 One-Click List-Unsubscribe Support**:
  - Added `.unsubscribe_url(...)` and `.unsubscribe_email(...)` to `Message`.
  - Added `.list_unsubscribe_header()` method generating compliant `<mailto:...>, <https://...>` headers.
  - Automatically injected RFC 8058 headers (`List-Unsubscribe` and `List-Unsubscribe-Post: List-Unsubscribe=One-Click`) across all delivery drivers (`SmtpDriver`, `ResendDriver`, `SendGridDriver`, and `LogDriver`).
- **Automatic Plain-Text Fallback Derivation**:
  - Built-in `strip_html_to_plain_text()` converter that parses HTML elements (`<h1>`-`<h6>`, `<p>`, `<div>`, `<li>`, `<br>`) and decodes HTML entities.
  - Automatically creates `body_text` fallback when `.html(...)` is called without requiring manual duplication.
- **Outbound DLP (Data Loss Prevention) Secret Scanner**:
  - Built-in `redact_email_secrets()` and `.sanitize_secrets()` to proactively mask AWS credentials (`AKIA...`), database passwords, API keys, bearer tokens, and private RSA keys before leaving the server.
- **In-Memory `MailTrap`, `MemoryDriver` & Fluent Assertions**:
  - Zero-I/O in-memory mail driver for lightning-fast testing.
  - `MailTrap` facade with fluent testing helpers (`assert_nothing_sent()`, `assert_sent_to("user@...").with_subject(...).with_body_contains(...).with_unsubscribe_url(...)`).
  - Isolated instance creation via `MemoryDriver::isolated()`.
- **Scaffolding CLI Command (`cargo rullst make:mail`)**:
  - Generate strongly-typed Mailables in `src/mail/` with responsive dark-mode HTML email layout, plain-text fallback, and secret sanitization.
  - Supported blueprint flags: `--welcome`, `--reset`, `--otp`, and `--invoice`.
- **Modular Sub-Module Architecture (< 300 lines per file)**:
  - Decomposed monolithic `mail.rs` into clean, decoupled modules:
    - `message.rs`
    - `drivers/mod.rs`, `drivers/log.rs`, `drivers/smtp.rs`, `drivers/resend.rs`, `drivers/sendgrid.rs`, `drivers/memory.rs`
    - `facade.rs`
    - `worker.rs`
    - `lib.rs`

### 🛡️ Security & Reliability
- Formal verification with Kani proofs on builder methods.
- Zero-panic guarantees in all production dispatch paths with structured `MailError`.
