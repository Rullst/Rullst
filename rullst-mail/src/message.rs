use crate::attachment::Attachment;
use crate::error::MailError;
use crate::security::scan_content_security;
use chrono::{DateTime, Utc};

/// An email message structure to be sent via a mail driver.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Message {
    /// The recipient email address.
    pub to: String,
    /// The subject line of the email.
    pub subject: String,
    /// Optional HTML body content.
    pub body_html: Option<String>,
    /// Optional plain-text body content.
    pub body_text: Option<String>,
    /// Optional sender email address.
    pub from: Option<String>,
    /// Optional RFC 8058 One-Click List-Unsubscribe URL.
    pub unsubscribe_url: Option<String>,
    /// Optional RFC 8058 List-Unsubscribe email address.
    pub unsubscribe_email: Option<String>,
    /// Optional scheduled delivery timestamp (UTC).
    pub send_at: Option<DateTime<Utc>>,
    /// File attachments and inline Content-ID (`CID`) media assets.
    pub attachments: Vec<Attachment>,
}

impl Default for Message {
    fn default() -> Self {
        Self::new()
    }
}

impl Message {
    /// Creates a new, empty `Message`.
    pub fn new() -> Self {
        Message {
            to: String::new(),
            subject: String::new(),
            body_html: None,
            body_text: None,
            from: None,
            unsubscribe_url: None,
            unsubscribe_email: None,
            send_at: None,
            attachments: Vec::new(),
        }
    }

    /// Sets the recipient email address.
    pub fn to(mut self, to: impl Into<String>) -> Self {
        self.to = to.into();
        self
    }

    /// Sets the email subject.
    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = subject.into();
        self
    }

    /// Sets the HTML body content and automatically derives a clean plain-text fallback if `body_text` is unset.
    pub fn html(mut self, html: impl Into<String>) -> Self {
        let html_str = html.into();
        if self.body_text.is_none() {
            self.body_text = Some(strip_html_to_plain_text(&html_str));
        }
        self.body_html = Some(html_str);
        self
    }

    /// Sets the plain-text body content explicitly.
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.body_text = Some(text.into());
        self
    }

    /// Sets the sender email address.
    pub fn from(mut self, from: impl Into<String>) -> Self {
        self.from = Some(from.into());
        self
    }

    /// Schedules delivery for a specific future UTC timestamp.
    pub fn send_at(mut self, timestamp: DateTime<Utc>) -> Self {
        self.send_at = Some(timestamp);
        self
    }

    /// Schedules delivery after the specified duration from now.
    pub fn send_in(mut self, duration: std::time::Duration) -> Self {
        let Ok(chrono_dur) = chrono::Duration::from_std(duration) else {
            return self;
        };
        self.send_at = Some(Utc::now() + chrono_dur);
        self
    }

    /// Appends a raw pre-constructed `Attachment`.
    pub fn attach(mut self, attachment: Attachment) -> Self {
        self.attachments.push(attachment);
        self
    }

    /// Appends an in-memory byte attachment.
    pub fn attach_bytes(
        mut self,
        filename: impl Into<String>,
        content: impl Into<Vec<u8>>,
        mime_type: impl Into<String>,
    ) -> Self {
        self.attachments
            .push(Attachment::new(filename, content, mime_type));
        self
    }

    /// Appends an inline media asset with a designated Content-ID (`CID`).
    pub fn attach_cid(
        mut self,
        cid: impl Into<String>,
        filename: impl Into<String>,
        content: impl Into<Vec<u8>>,
        mime_type: impl Into<String>,
    ) -> Self {
        self.attachments
            .push(Attachment::inline(cid, filename, content, mime_type));
        self
    }

    /// Reads and attaches a local file from disk.
    pub fn attach_file(
        mut self,
        path: impl AsRef<std::path::Path>,
    ) -> Result<Self, std::io::Error> {
        let attachment = Attachment::from_file(path)?;
        self.attachments.push(attachment);
        Ok(self)
    }

    /// Sets the RFC 8058 One-Click List-Unsubscribe URL.
    pub fn unsubscribe_url(mut self, url: impl Into<String>) -> Self {
        self.unsubscribe_url = Some(url.into());
        self
    }

    /// Sets the RFC 8058 List-Unsubscribe email address.
    pub fn unsubscribe_email(mut self, email: impl Into<String>) -> Self {
        self.unsubscribe_email = Some(email.into());
        self
    }

    /// Computes the formatted `List-Unsubscribe` header value compliant with RFC 2369 / RFC 8058.
    pub fn list_unsubscribe_header(&self) -> Option<String> {
        match (&self.unsubscribe_email, &self.unsubscribe_url) {
            (Some(email), Some(url)) => Some(format!("<mailto:{}>, <{}>", email, url)),
            (None, Some(url)) => Some(format!("<{}>", url)),
            (Some(email), None) => Some(format!("<mailto:{}>", email)),
            (None, None) => None,
        }
    }

    /// Sanitizes sensitive secrets, AWS keys, passwords, and tokens from the email subject and bodies.
    pub fn sanitize_secrets(mut self) -> Self {
        self.subject = redact_email_secrets(&self.subject);
        if let Some(ref html) = self.body_html {
            self.body_html = Some(redact_email_secrets(html));
        }
        if let Some(ref text) = self.body_text {
            self.body_text = Some(redact_email_secrets(text));
        }
        self
    }

    /// Validates that the email content does not contain dangerous URI schemes or homograph URL spoofing.
    pub fn validate_security(&self) -> Result<(), MailError> {
        if let Some(ref html) = self.body_html {
            scan_content_security(html)?;
        }
        if let Some(ref text) = self.body_text {
            scan_content_security(text)?;
        }
        Ok(())
    }

    /// Validates recipient deliverability syntax and blocks known disposable email providers.
    pub fn validate_deliverability(&self) -> Result<(), crate::validator::DeliverabilityError> {
        crate::validator::validate_email_deliverability(&self.to)
    }

    /// Checks whether the recipient address belongs to a disposable email provider.
    pub fn is_disposable(&self) -> bool {
        crate::validator::is_disposable_email(&self.to)
    }

    /// Injects a zero-cookie 1x1 tracking pixel into the HTML body.
    pub fn with_open_tracking(
        mut self,
        base_tracker_url: &str,
        secret: &[u8],
        campaign_id: &str,
    ) -> Self {
        if let Some(ref html) = self.body_html {
            let token = crate::tracking::TrackingEngine::generate_open_token(
                secret,
                &self.to,
                campaign_id,
                chrono::Utc::now().timestamp() as u64,
            );
            let pixel_url = format!(
                "{}/track/open/{}",
                base_tracker_url.trim_end_matches('/'),
                token
            );
            self.body_html = Some(crate::tracking::TrackingEngine::inject_open_pixel(
                html, &pixel_url,
            ));
        }
        self
    }

    /// Rewrites all HTML links to route through the privacy-preserving click tracker.
    pub fn with_click_tracking(mut self, base_tracker_url: &str, secret: &[u8]) -> Self {
        if let Some(ref html) = self.body_html {
            self.body_html = Some(crate::tracking::TrackingEngine::rewrite_links(
                html,
                base_tracker_url,
                secret,
                &self.to,
                chrono::Utc::now().timestamp() as u64,
            ));
        }
        self
    }
}

/// Converts HTML content into a clean, accessible plain-text representation for email clients.
pub fn strip_html_to_plain_text(html: &str) -> String {
    if html.is_empty() {
        return String::new();
    }

    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut tag_buffer = String::new();
    let mut in_style_or_script = false;

    for c in html.chars() {
        if c == '<' {
            in_tag = true;
            tag_buffer.clear();
            continue;
        }

        if in_tag {
            if c == '>' {
                in_tag = false;
                let tag_lower = tag_buffer.trim().to_lowercase();

                if tag_lower.starts_with("style") || tag_lower.starts_with("script") {
                    in_style_or_script = true;
                } else if tag_lower.starts_with("/style") || tag_lower.starts_with("/script") {
                    in_style_or_script = false;
                } else if tag_lower == "br"
                    || tag_lower == "br/"
                    || tag_lower == "br /"
                    || tag_lower == "p"
                    || tag_lower == "/p"
                    || tag_lower == "div"
                    || tag_lower == "/div"
                    || tag_lower == "tr"
                    || tag_lower == "/tr"
                    || tag_lower.starts_with("h1")
                    || tag_lower.starts_with("h2")
                    || tag_lower.starts_with("h3")
                    || tag_lower.starts_with("h4")
                    || tag_lower.starts_with("h5")
                    || tag_lower.starts_with("h6")
                    || tag_lower == "/h1"
                    || tag_lower == "/h2"
                    || tag_lower == "/h3"
                    || tag_lower == "/h4"
                    || tag_lower == "/h5"
                    || tag_lower == "/h6"
                {
                    result.push('\n');
                } else if tag_lower == "li" {
                    result.push_str("\n• ");
                }
            } else {
                tag_buffer.push(c);
            }
            continue;
        }

        if !in_style_or_script {
            result.push(c);
        }
    }

    // Decode standard HTML entities
    let decoded = result
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'");

    // Normalize multiple newlines and spaces
    let mut normalized = String::new();
    let mut consecutive_newlines = 0;

    for line in decoded.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if consecutive_newlines < 1 {
                normalized.push('\n');
                consecutive_newlines += 1;
            }
        } else {
            if !normalized.is_empty() && !normalized.ends_with('\n') {
                normalized.push('\n');
            }
            normalized.push_str(trimmed);
            normalized.push('\n');
            consecutive_newlines = 0;
        }
    }

    normalized.trim().to_string()
}

/// Outbound DLP Scanner: Sanitizes sensitive credentials, AWS keys, passwords, and tokens.
pub fn redact_email_secrets(input: &str) -> String {
    if input.is_empty() {
        return String::new();
    }

    let mut result = input.to_string();

    // 1. Mask Authorization Bearer tokens
    if let Some(idx) = result.find("Bearer ") {
        let start = idx + 7;
        if result.is_char_boundary(start) {
            let end = result[start..]
                .find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == ',' || c == '<')
                .map(|i| start + i)
                .unwrap_or(result.len());

            if end > start && result.is_char_boundary(end) {
                let token_slice = &result[start..end];
                if token_slice.chars().count() > 6 {
                    let prefix: String = token_slice.chars().take(4).collect();
                    let masked = format!("{}...", prefix);
                    result.replace_range(start..end, &masked);
                }
            }
        }
    }

    // 2. Mask password=, secret=, api_key=, key=, token=
    for key in &["password=", "secret=", "api_key=", "key=", "token="] {
        let lower = result.to_lowercase();
        if let Some(idx) = lower.find(key) {
            let start = idx + key.len();
            if result.is_char_boundary(start) {
                let end = result[start..]
                    .find(|c: char| {
                        c.is_whitespace()
                            || c == '"'
                            || c == '\''
                            || c == '&'
                            || c == ','
                            || c == '<'
                    })
                    .map(|i| start + i)
                    .unwrap_or(result.len());

                if end > start
                    && result.is_char_boundary(end)
                    && &result[start..end] != "[REDACTED]"
                {
                    result.replace_range(start..end, "[REDACTED]");
                }
            }
        }
    }

    // 3. Mask AWS keys (AKIA...)
    if let Some(idx) = result.find("AKIA") {
        let mut end = (idx + 20).min(result.len());
        while end < result.len() && !result.is_char_boundary(end) {
            end += 1;
        }
        if result.is_char_boundary(idx) && result.is_char_boundary(end) {
            result.replace_range(idx..end, "AKIA****************");
        }
    }

    // 4. Mask Private Keys
    if result.contains("-----BEGIN PRIVATE KEY-----")
        || result.contains("-----BEGIN RSA PRIVATE KEY-----")
    {
        result = result
            .replace("-----BEGIN PRIVATE KEY-----", "[REDACTED PRIVATE KEY]")
            .replace(
                "-----BEGIN RSA PRIVATE KEY-----",
                "[REDACTED RSA PRIVATE KEY]",
            );
    }

    result
}
