//! Transactional email fixtures & factory generator for testing, preview, and local development.

use crate::message::Message;

/// Factory generator for common transactional email blueprints.
pub struct MailFactory;

impl MailFactory {
    /// Generates a welcome and onboarding verification email.
    pub fn fake_welcome(to: &str, user_name: &str, app_name: &str) -> Message {
        let subject = format!("Welcome to {}!", app_name);
        let html = format!(
            r#"<!DOCTYPE html>
<html>
<head><meta charset="utf-8"><title>{app_name}</title></head>
<body style="font-family: sans-serif; background-color: #0f172a; color: #f8fafc; padding: 40px 20px;">
  <div style="max-width: 560px; margin: 0 auto; background-color: #1e293b; border-radius: 12px; padding: 32px; border: 1px solid #334155;">
    <h1 style="color: #38bdf8; font-size: 24px; margin-bottom: 16px;">Welcome aboard, {user_name}! 👋</h1>
    <p style="color: #cbd5e1; font-size: 14px; line-height: 1.6;">Thanks for joining {app_name}. We're excited to have you on board.</p>
    <div style="margin: 28px 0;">
      <a href="https://example.com/verify?user={user_name}" style="background-color: #0ea5e9; color: #ffffff; padding: 12px 24px; border-radius: 8px; text-decoration: none; font-weight: bold; font-size: 14px; display: inline-block;">Verify Email Address</a>
    </div>
    <p style="color: #64748b; font-size: 12px; margin-top: 32px; border-top: 1px solid #334155; padding-top: 16px;">If you did not create an account on {app_name}, please ignore this email.</p>
  </div>
</body>
</html>"#,
            app_name = app_name,
            user_name = user_name
        );

        Message::new().to(to).subject(subject).html(html)
    }

    /// Generates a time-limited password reset email.
    pub fn fake_password_reset(to: &str, reset_url: &str, expires_in_mins: u32) -> Message {
        let subject = "Reset your account password".to_string();
        let html = format!(
            r#"<!DOCTYPE html>
<html>
<head><meta charset="utf-8"><title>Reset Password</title></head>
<body style="font-family: sans-serif; background-color: #0f172a; color: #f8fafc; padding: 40px 20px;">
  <div style="max-width: 560px; margin: 0 auto; background-color: #1e293b; border-radius: 12px; padding: 32px; border: 1px solid #334155;">
    <h1 style="color: #f59e0b; font-size: 22px; margin-bottom: 16px;">Password Reset Request 🔐</h1>
    <p style="color: #cbd5e1; font-size: 14px; line-height: 1.6;">We received a request to reset your password. This link expires in <strong>{expires} minutes</strong>.</p>
    <div style="margin: 28px 0;">
      <a href="{url}" style="background-color: #d97706; color: #ffffff; padding: 12px 24px; border-radius: 8px; text-decoration: none; font-weight: bold; font-size: 14px; display: inline-block;">Reset Password</a>
    </div>
    <p style="color: #64748b; font-size: 12px; margin-top: 32px; border-top: 1px solid #334155; padding-top: 16px;">If you didn't request a password reset, you can safely ignore this email.</p>
  </div>
</body>
</html>"#,
            url = reset_url,
            expires = expires_in_mins
        );

        Message::new().to(to).subject(subject).html(html)
    }

    /// Generates a high-visibility OTP authentication code email.
    pub fn fake_otp(to: &str, code: &str, expires_in_mins: u32) -> Message {
        let subject = format!("Your verification code: {}", code);
        let html = format!(
            r#"<!DOCTYPE html>
<html>
<head><meta charset="utf-8"><title>Verification Code</title></head>
<body style="font-family: sans-serif; background-color: #0f172a; color: #f8fafc; padding: 40px 20px;">
  <div style="max-width: 560px; margin: 0 auto; background-color: #1e293b; border-radius: 12px; padding: 32px; border: 1px solid #334155; text-align: center;">
    <h1 style="color: #38bdf8; font-size: 22px; margin-bottom: 12px;">Security Verification Code 🔑</h1>
    <p style="color: #cbd5e1; font-size: 14px;">Use the verification code below to complete your sign in:</p>
    <div style="margin: 24px 0; padding: 16px; background-color: #0f172a; border: 2px dashed #0284c7; border-radius: 8px;">
      <span style="font-size: 32px; font-weight: 800; letter-spacing: 6px; color: #38bdf8; font-family: monospace;">{code}</span>
    </div>
    <p style="color: #94a3b8; font-size: 12px;">This code will expire in {expires} minutes. Never share this code with anyone.</p>
  </div>
</body>
</html>"#,
            code = code,
            expires = expires_in_mins
        );

        Message::new().to(to).subject(subject).html(html)
    }

    /// Generates an invoice and billing receipt email.
    pub fn fake_invoice(
        to: &str,
        invoice_number: &str,
        amount_cents: u64,
        currency: &str,
    ) -> Message {
        let formatted_amount = format!(
            "{:.2} {}",
            (amount_cents as f64) / 100.0,
            currency.to_uppercase()
        );
        let subject = format!(
            "Receipt for Invoice #{} ({})",
            invoice_number, formatted_amount
        );
        let html = format!(
            r#"<!DOCTYPE html>
<html>
<head><meta charset="utf-8"><title>Invoice #{num}</title></head>
<body style="font-family: sans-serif; background-color: #0f172a; color: #f8fafc; padding: 40px 20px;">
  <div style="max-width: 560px; margin: 0 auto; background-color: #1e293b; border-radius: 12px; padding: 32px; border: 1px solid #334155;">
    <div style="display: flex; justify-content: space-between; border-bottom: 1px solid #334155; padding-bottom: 16px; margin-bottom: 24px;">
      <h1 style="color: #38bdf8; font-size: 20px; margin: 0;">Payment Receipt ✅</h1>
      <span style="color: #94a3b8; font-size: 14px; font-mono;">#{num}</span>
    </div>
    <p style="color: #cbd5e1; font-size: 14px;">Thank you for your payment! Here is the summary of your transaction:</p>
    <div style="background-color: #0f172a; border-radius: 8px; padding: 16px; margin: 20px 0;">
      <div style="font-size: 24px; font-weight: bold; color: #10b981;">{amount}</div>
      <div style="color: #64748b; font-size: 12px; margin-top: 4px;">Status: Paid &bull; Automated SaaS Billing</div>
    </div>
  </div>
</body>
</html>"#,
            num = invoice_number,
            amount = formatted_amount
        );

        Message::new().to(to).subject(subject).html(html)
    }

    /// Generates a suspicious login / security alert email.
    pub fn fake_security_alert(to: &str, action: &str, ip_address: &str, device: &str) -> Message {
        let subject = format!("Security Alert: {} detected", action);
        let html = format!(
            r#"<!DOCTYPE html>
<html>
<head><meta charset="utf-8"><title>Security Alert</title></head>
<body style="font-family: sans-serif; background-color: #0f172a; color: #f8fafc; padding: 40px 20px;">
  <div style="max-width: 560px; margin: 0 auto; background-color: #1e293b; border-radius: 12px; padding: 32px; border: 1px solid #f43f5e;">
    <h1 style="color: #f43f5e; font-size: 22px; margin-bottom: 16px;">Security Notice 🚨</h1>
    <p style="color: #cbd5e1; font-size: 14px; line-height: 1.6;">We detected a new security event: <strong>{action}</strong></p>
    <ul style="color: #94a3b8; font-size: 13px; font-family: monospace; background-color: #0f172a; padding: 16px 32px; border-radius: 8px;">
      <li>IP Address: {ip}</li>
      <li>Device / Client: {dev}</li>
    </ul>
    <p style="color: #e2e8f0; font-size: 13px; margin-top: 20px;">If this wasn't you, please change your password immediately in your account settings.</p>
  </div>
</body>
</html>"#,
            action = action,
            ip = ip_address,
            dev = device
        );

        Message::new().to(to).subject(subject).html(html)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mail_factory_fixtures() {
        let welcome = MailFactory::fake_welcome("user@example.com", "Alice", "Rullst");
        assert_eq!(welcome.to, "user@example.com");
        assert!(welcome.subject.contains("Welcome to Rullst"));
        assert!(welcome.body_html.unwrap().contains("Alice"));

        let reset = MailFactory::fake_password_reset(
            "bob@example.com",
            "https://app.com/reset?token=123",
            15,
        );
        assert!(reset.subject.to_lowercase().contains("password"));
        assert!(reset.body_html.unwrap().contains("15 minutes"));

        let otp = MailFactory::fake_otp("carol@example.com", "123456", 5);
        assert!(otp.subject.contains("123456"));
        assert!(otp.body_html.unwrap().contains("123456"));

        let invoice = MailFactory::fake_invoice("david@example.com", "INV-2026-001", 9900, "USD");
        assert!(invoice.subject.contains("INV-2026-001"));
        assert!(invoice.body_html.unwrap().contains("99.00 USD"));

        let sec = MailFactory::fake_security_alert(
            "eve@example.com",
            "New Login from Japan",
            "192.0.2.1",
            "Firefox / Linux",
        );
        assert!(sec.subject.contains("Security Alert"));
        assert!(sec.body_html.unwrap().contains("192.0.2.1"));
    }
}
