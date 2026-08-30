// src/generators/mail.rs — Mailable Struct and Email Template generator (`cargo rullst make:mail`).

use crate::generators::chat::ensure_rullst_features;
use crate::generators::{is_rullst_project, is_valid_rust_identifier, register_mod_ast};
use colored::*;
use std::fs;
use std::io::{Error as IoError, ErrorKind};
use std::path::Path;

mod extended;
mod names;
use names::{project_root_module, to_pascal_case, to_snake_case};

/// Template family selected by the CLI after it rejects conflicting flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MailableKind {
    Custom,
    Welcome,
    Reset,
    Otp,
    Invoice,
    FiscalInvoice,
    Dunning,
}

pub(crate) fn create_new_mailable(
    name: &str,
    kind: MailableKind,
) -> Result<(), Box<dyn std::error::Error>> {
    if !is_rullst_project() {
        return Err(IoError::new(
            ErrorKind::InvalidInput,
            "make:mail must be run inside a Rullst project",
        )
        .into());
    }
    let pascal_name = to_pascal_case(name);
    let snake_name = to_snake_case(&pascal_name);
    if !is_valid_rust_identifier(&pascal_name) || !is_valid_rust_identifier(&snake_name) {
        return Err(IoError::new(
            ErrorKind::InvalidInput,
            "mailable name must produce valid non-keyword Rust identifiers",
        )
        .into());
    }

    let root = project_root_module()?;
    let root_content = fs::read_to_string(&root)?;
    syn::parse_file(&root_content).map_err(|error| {
        IoError::new(
            ErrorKind::InvalidData,
            format!("{} is not valid Rust: {error}", root.display()),
        )
    })?;

    let mail_dir = Path::new("src/mail");
    let mod_path = mail_dir.join("mod.rs");
    let file_path = mail_dir.join(format!("{snake_name}.rs"));
    if file_path.exists() {
        return Err(IoError::new(
            ErrorKind::AlreadyExists,
            format!("refusing to overwrite {}", file_path.display()),
        )
        .into());
    }

    let mut mod_content = if mod_path.exists() {
        fs::read_to_string(&mod_path)?
    } else {
        String::new()
    };
    syn::parse_file(&mod_content).map_err(|error| {
        IoError::new(
            ErrorKind::InvalidData,
            format!("{} is not valid Rust: {error}", mod_path.display()),
        )
    })?;
    let module_declaration = format!("pub mod {snake_name};");
    if mod_content
        .lines()
        .any(|line| line.trim() == module_declaration)
    {
        return Err(IoError::new(
            ErrorKind::AlreadyExists,
            format!("mailable module {snake_name} is already registered"),
        )
        .into());
    }
    if !mod_content.is_empty() && !mod_content.ends_with('\n') {
        mod_content.push('\n');
    }
    mod_content.push_str(&format!(
        "pub mod {snake_name};\npub use {snake_name}::{pascal_name};\n"
    ));

    let manifest_path = Path::new("Cargo.toml");
    let manifest = fs::read_to_string(manifest_path)?;
    let required_features = match kind {
        MailableKind::FiscalInvoice => &["mailer", "capital"][..],
        _ => &["mailer"][..],
    };
    let updated_manifest = ensure_rullst_features(&manifest, required_features)?;

    println!(
        "{}",
        format!("📬 Generating Mailable Rullst: {}...", pascal_name)
            .cyan()
            .bold()
    );

    let template = if kind == MailableKind::Welcome {
        r##"//! Welcome & Onboarding Mailable template.
use rullst::mail::{escape_html, Mail, MailError, Message};

/// Welcome email sent to newly registered users with email verification CTA.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct __NAME__ {
    pub to: String,
    pub user_name: String,
    pub verification_url: String,
    pub unsubscribe_url: String,
}

impl __NAME__ {
    pub fn new(
        to: impl Into<String>,
        user_name: impl Into<String>,
        verification_url: impl Into<String>,
        unsubscribe_url: impl Into<String>,
    ) -> Self {
        Self {
            to: to.into(),
            user_name: user_name.into(),
            verification_url: verification_url.into(),
            unsubscribe_url: unsubscribe_url.into(),
        }
    }

    /// Builds the `rullst::mail::Message` with an escaped HTML layout and plain-text fallback.
    pub fn build(&self) -> Message {
        let user_name = escape_html(&self.user_name);
        let verification_url = escape_html(&self.verification_url);
        let unsubscribe_url = escape_html(&self.unsubscribe_url);
        let html_content = format!(
            r#"<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Welcome to Rullst</title>
</head>
<body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif; background-color: #030712; color: #f9fafb; margin: 0; padding: 40px 20px;">
  <div style="max-width: 600px; margin: 0 auto; background-color: #111827; border: 1px solid #1f2937; border-radius: 12px; padding: 32px; box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1);">
    <h1 style="color: #60a5fa; font-size: 24px; font-weight: 700; margin-top: 0;">Welcome, {}! 👋</h1>
    <p style="color: #9ca3af; font-size: 16px; line-height: 1.6;">
      Thank you for joining us. We're excited to have you on board! Please verify your email address to get started.
    </p>
    <div style="margin: 32px 0;">
      <a href="{}" style="background-color: #2563eb; color: #ffffff; padding: 12px 24px; border-radius: 8px; font-weight: 600; text-decoration: none; display: inline-block;">
        Verify Email Address &rarr;
      </a>
    </div>
    <hr style="border: 0; border-top: 1px solid #1f2937; margin: 32px 0;" />
    <p style="color: #6b7280; font-size: 12px; line-height: 1.5;">
      If you did not create an account, you can safely ignore this email.<br>
      <a href="{}" style="color: #9ca3af; text-decoration: underline;">Unsubscribe from notification emails</a>
    </p>
  </div>
</body>
</html>"#,
            user_name, verification_url, unsubscribe_url
        );

        Message::new()
            .to(&self.to)
            .subject("Welcome to Rullst! Confirm your email")
            .html(html_content)
            .unsubscribe_url(&self.unsubscribe_url)
            .sanitize_secrets()
    }

    /// Sends the email using the configured Mail driver or queue.
    pub async fn send(&self) -> Result<(), MailError> {
        Mail::send(self.build()).await
    }
}
"##
    } else if kind == MailableKind::Reset {
        r##"//! Time-limited Password Reset Mailable.
use rullst::mail::{escape_html, Mail, MailError, Message};

/// Secure password reset email with expiration indicator.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct __NAME__ {
    pub to: String,
    pub user_name: String,
    pub reset_url: String,
    pub expires_in_minutes: u32,
}

impl __NAME__ {
    pub fn new(
        to: impl Into<String>,
        user_name: impl Into<String>,
        reset_url: impl Into<String>,
        expires_in_minutes: u32,
    ) -> Self {
        Self {
            to: to.into(),
            user_name: user_name.into(),
            reset_url: reset_url.into(),
            expires_in_minutes,
        }
    }

    pub fn build(&self) -> Message {
        let user_name = escape_html(&self.user_name);
        let reset_url = escape_html(&self.reset_url);
        let html_content = format!(
            r#"<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <title>Reset your password</title>
</head>
<body style="font-family: sans-serif; background-color: #030712; color: #f9fafb; padding: 40px 20px;">
  <div style="max-width: 600px; margin: 0 auto; background-color: #111827; border: 1px solid #1f2937; border-radius: 12px; padding: 32px;">
    <h2 style="color: #ef4444; margin-top: 0;">Password Reset Request 🔒</h2>
    <p style="color: #9ca3af; font-size: 15px; line-height: 1.6;">
      Hello {}, we received a request to reset your password. This link will expire in <strong>{} minutes</strong>.
    </p>
    <div style="margin: 28px 0;">
      <a href="{}" style="background-color: #ef4444; color: #ffffff; padding: 12px 24px; border-radius: 8px; font-weight: 600; text-decoration: none; display: inline-block;">
        Reset Password
      </a>
    </div>
    <p style="color: #6b7280; font-size: 13px;">
      If you did not request this password reset, no action is required and your account remains secure.
    </p>
  </div>
</body>
</html>"#,
            user_name, self.expires_in_minutes, reset_url
        );

        Message::new()
            .to(&self.to)
            .subject("Reset your password")
            .html(html_content)
            .sanitize_secrets()
    }

    pub async fn send(&self) -> Result<(), MailError> {
        Mail::send(self.build()).await
    }
}
"##
    } else if kind == MailableKind::Otp {
        r##"//! High-visibility Two-Factor Authentication (OTP) Mailable.
use rullst::mail::{escape_html, Mail, MailError, Message};

/// Two-Factor authentication OTP security code email.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct __NAME__ {
    pub to: String,
    pub otp_code: String,
    pub expires_in_minutes: u32,
}

impl __NAME__ {
    pub fn new(to: impl Into<String>, otp_code: impl Into<String>, expires_in_minutes: u32) -> Self {
        Self {
            to: to.into(),
            otp_code: otp_code.into(),
            expires_in_minutes,
        }
    }

    pub fn build(&self) -> Message {
        let otp_code = escape_html(&self.otp_code);
        let html_content = format!(
            r#"<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <title>Your Verification Code</title>
</head>
<body style="font-family: sans-serif; background-color: #030712; color: #f9fafb; padding: 40px 20px;">
  <div style="max-width: 500px; margin: 0 auto; background-color: #111827; border: 1px solid #1f2937; border-radius: 12px; padding: 32px; text-align: center;">
    <h2 style="color: #a855f7; margin-top: 0;">Security Verification Code 🛡️</h2>
    <p style="color: #9ca3af; font-size: 15px;">Use the verification code below to complete your login. Valid for {} minutes.</p>
    <div style="background-color: #1e1b4b; border: 1px dashed #6366f1; border-radius: 8px; padding: 18px; margin: 24px 0; font-size: 32px; font-weight: 800; letter-spacing: 6px; color: #c084fc;">
      {}
    </div>
    <p style="color: #6b7280; font-size: 12px;">Never share this code with anyone. Our team will never ask for your verification code.</p>
  </div>
</body>
</html>"#,
            self.expires_in_minutes, otp_code
        );

        Message::new()
            .to(&self.to)
            .subject(format!("Your verification code: {}", self.otp_code))
            .html(html_content)
            .sanitize_secrets()
    }

    pub async fn send(&self) -> Result<(), MailError> {
        Mail::send(self.build()).await
    }
}
"##
    } else if kind == MailableKind::Invoice {
        r##"//! SaaS Invoice / Receipt Mailable.
use rullst::mail::{escape_html, Mail, MailError, Message};

/// Transactional invoice receipt for SaaS billing.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct __NAME__ {
    pub to: String,
    pub customer_name: String,
    pub invoice_id: String,
    pub amount: String,
    pub invoice_url: String,
}

impl __NAME__ {
    pub fn new(
        to: impl Into<String>,
        customer_name: impl Into<String>,
        invoice_id: impl Into<String>,
        amount: impl Into<String>,
        invoice_url: impl Into<String>,
    ) -> Self {
        Self {
            to: to.into(),
            customer_name: customer_name.into(),
            invoice_id: invoice_id.into(),
            amount: amount.into(),
            invoice_url: invoice_url.into(),
        }
    }

    pub fn build(&self) -> Message {
        let customer_name = escape_html(&self.customer_name);
        let invoice_id = escape_html(&self.invoice_id);
        let amount = escape_html(&self.amount);
        let invoice_url = escape_html(&self.invoice_url);
        let html_content = format!(
            r#"<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <title>Payment Receipt</title>
</head>
<body style="font-family: sans-serif; background-color: #030712; color: #f9fafb; padding: 40px 20px;">
  <div style="max-width: 600px; margin: 0 auto; background-color: #111827; border: 1px solid #1f2937; border-radius: 12px; padding: 32px;">
    <h2 style="color: #10b981; margin-top: 0;">Payment Receipt ✅</h2>
    <p style="color: #9ca3af; font-size: 15px;">Hello {}, thank you for your payment!</p>
    <div style="background-color: #064e3b; border-radius: 8px; padding: 20px; margin: 24px 0;">
      <div style="color: #a7f3d0; font-size: 14px;">Invoice #{}</div>
      <div style="color: #ffffff; font-size: 28px; font-weight: 700; margin-top: 4px;">{}</div>
    </div>
    <div style="margin: 24px 0;">
      <a href="{}" style="background-color: #10b981; color: #ffffff; padding: 10px 20px; border-radius: 6px; font-weight: 600; text-decoration: none; display: inline-block;">
        View Invoice &amp; PDF &rarr;
      </a>
    </div>
  </div>
</body>
</html>"#,
            customer_name, invoice_id, amount, invoice_url
        );

        Message::new()
            .to(&self.to)
            .subject(format!("Receipt for Invoice #{}", self.invoice_id))
            .html(html_content)
            .sanitize_secrets()
    }

    pub async fn send(&self) -> Result<(), MailError> {
        Mail::send(self.build()).await
    }
}
"##
    } else if kind == MailableKind::FiscalInvoice {
        extended::FISCAL_INVOICE_TEMPLATE
    } else if kind == MailableKind::Dunning {
        extended::DUNNING_TEMPLATE
    } else {
        r##"//! Custom Transactional Mailable template.
use rullst::mail::{escape_html, Mail, MailError, Message};

/// Strongly-typed mailable struct.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct __NAME__ {
    pub to: String,
    pub subject: String,
    pub message_body: String,
    pub unsubscribe_url: Option<String>,
}

impl __NAME__ {
    pub fn new(to: impl Into<String>, subject: impl Into<String>, message_body: impl Into<String>) -> Self {
        Self {
            to: to.into(),
            subject: subject.into(),
            message_body: message_body.into(),
            unsubscribe_url: None,
        }
    }

    pub fn with_unsubscribe(mut self, url: impl Into<String>) -> Self {
        self.unsubscribe_url = Some(url.into());
        self
    }

    pub fn build(&self) -> Message {
        let subject = escape_html(&self.subject);
        let message_body = escape_html(&self.message_body);
        let html_content = format!(
            r#"<!DOCTYPE html>
<html>
<head><meta charset="utf-8"></head>
<body style="font-family: sans-serif; background-color: #030712; color: #f9fafb; padding: 32px 16px;">
  <div style="max-width: 600px; margin: 0 auto; background-color: #111827; border: 1px solid #1f2937; border-radius: 8px; padding: 24px;">
    <h2 style="color: #60a5fa; margin-top: 0;">{}</h2>
    <p style="color: #d1d5db; line-height: 1.6;">{}</p>
  </div>
</body>
</html>"#,
            subject, message_body
        );

        let mut msg = Message::new()
            .to(&self.to)
            .subject(&self.subject)
            .html(html_content);

        if let Some(ref unsub) = self.unsubscribe_url {
            msg = msg.unsubscribe_url(unsub);
        }

        msg.sanitize_secrets()
    }

    pub async fn send(&self) -> Result<(), MailError> {
        Mail::send(self.build()).await
    }
}
"##
    };

    let content = template.replace("__NAME__", &pascal_name);
    syn::parse_file(&content).map_err(|error| {
        IoError::new(
            ErrorKind::InvalidData,
            format!("generated mailable is not valid Rust: {error}"),
        )
    })?;

    fs::create_dir_all(mail_dir)?;
    fs::write(&file_path, content)?;
    fs::write(manifest_path, updated_manifest)?;
    fs::write(&mod_path, mod_content)?;
    register_mod_ast(&root, "mail")?;

    println!(
        "{}",
        format!(
            "✅ Mailable created successfully at: {}",
            file_path.display()
        )
        .green()
    );

    Ok(())
}
