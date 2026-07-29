# Rullst Mail - Roadmap

Rullst Mail aims to be the standard Mailable API for Rust, abstracting away the complexity of email rendering and delivery.

## Phase 1: Sending & Drivers
- [ ] **Multi-Drivers**: Native adapters for standard SMTP, Resend, SendGrid, and AWS SES using a unified interface.
- [ ] **Background Queues**: When calling `Mail::send()`, automatically push the email to a background worker queue (via Redis or database) instead of blocking the main web request thread.

## Phase 2: Beautiful Templating
- [ ] **HTML Templating**: Seamless integration with Rullst UI (`html!` macros) to build responsive, component-based email templates natively in Rust.
- [ ] **Mailables as Structs**: Define emails as Rust structs (e.g., `WelcomeEmail { user_name: String }`) that auto-render the subject and body dynamically.
