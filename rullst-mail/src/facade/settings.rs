//! `Mail` facade configuration: a `MAIL_*` variable wins, then `Rullst.toml`.

use crate::error::MailError;
use crate::message::Message;
use crate::security::is_crlf_safe;
use crate::validator::recipient_address;

/// Facade settings read once per facade call.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(super) struct MailSettings {
    /// `MAIL_DRIVER`, else `[mail] driver`.
    driver: Option<String>,
    /// `MAIL_FROM`, else `[mail] from`; validated when it is used.
    from: Option<String>,
    /// `[app] env`, consulted after `RULLST_ENV` and `APP_ENV`.
    app_env: Option<String>,
}

impl MailSettings {
    /// Reads the process environment, then `Rullst.toml` in the working
    /// directory for anything the environment leaves unset. An empty
    /// `MAIL_FROM` counts as unset.
    pub(super) async fn load() -> Self {
        let mut settings = Self {
            driver: std::env::var("MAIL_DRIVER").ok(),
            from: std::env::var("MAIL_FROM")
                .ok()
                .filter(|from| !from.trim().is_empty()),
            app_env: None,
        };
        if (settings.driver.is_none() || settings.from.is_none())
            && let Ok(content) = tokio::fs::read_to_string("Rullst.toml").await
        {
            settings.fill_from_rullst_toml(&content);
        }
        settings
    }

    /// Fills unset values from `[mail]`/`[mailer]` `driver` and `from` and
    /// from `[app] env`, with the facade's line-based parsing: a `#` starts a
    /// comment and surrounding quotes are removed.
    pub(super) fn fill_from_rullst_toml(&mut self, content: &str) {
        let (mut driver, mut from, mut app_env) = (None, None, None);
        let (mut in_mail, mut in_app) = (false, false);
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') {
                in_mail = trimmed == "[mail]" || trimmed == "[mailer]";
                in_app = trimmed == "[app]";
                continue;
            }
            let Some((key, value)) = trimmed.split_once('=') else {
                continue;
            };
            let value = value.split('#').next().unwrap_or(value).trim();
            let value = value.trim_matches('"').trim_matches('\'').to_string();
            match key.trim() {
                "driver" if in_mail => driver = Some(value),
                "from" if in_mail => from = Some(value).filter(|from| !from.trim().is_empty()),
                "env" if in_app => app_env = Some(value),
                _ => {}
            }
        }
        self.driver = self.driver.take().or(driver);
        self.from = self.from.take().or(from);
        self.app_env = app_env;
    }

    /// The configured default sender after the pre-flight sender rules: one
    /// line holding one address or `Name <address>`.
    pub(super) fn default_sender(&self) -> Result<Option<&str>, MailError> {
        let Some(sender) = self.from.as_deref().map(str::trim) else {
            return Ok(None);
        };
        if !is_crlf_safe(sender) || recipient_address(sender).is_err() {
            return Err(MailError::ConfigError(
                "MAIL_FROM or [mail] from must be one address or `Name <address>` on one line"
                    .to_string(),
            ));
        }
        Ok(Some(sender))
    }

    /// Gives a message without a `from` the configured default sender. An
    /// explicit `from` always wins, but an invalid configured sender is
    /// rejected either way, so a bad setting cannot go unnoticed.
    pub(super) fn apply_default_sender(&self, mut message: Message) -> Result<Message, MailError> {
        let sender = self.default_sender()?;
        if message.from.is_none() {
            message.from = sender.map(str::to_string);
        }
        Ok(message)
    }

    /// The configured driver, or the default for this environment.
    pub(super) fn driver_name(&self) -> Result<String, MailError> {
        match &self.driver {
            Some(driver) => Ok(driver.clone()),
            None => default_driver_name(self.app_env.as_deref()).map(str::to_string),
        }
    }
}

/// The driver used when neither `MAIL_DRIVER` nor `[mail] driver` is set.
///
/// Logging is only a development/test default. In staging or production an
/// unconfigured facade would report success for mail it never sends, so it
/// fails closed unless `log` is selected explicitly.
pub(super) fn default_driver_name(configured_env: Option<&str>) -> Result<&'static str, MailError> {
    let environment = rullst_core::config::Environment::detect(configured_env).map_err(|_| {
        MailError::ConfigError("RULLST_ENV, APP_ENV or [app].env is invalid".to_string())
    })?;
    if environment.requires_secure_defaults() {
        return Err(MailError::ConfigError(format!(
            "no mail driver is configured for the {environment} environment; set MAIL_DRIVER \
             or [mail] driver (MAIL_DRIVER=log only logs metadata and delivers nothing)"
        )));
    }
    Ok("log")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(driver: Option<&str>, from: Option<&str>) -> MailSettings {
        MailSettings {
            driver: driver.map(str::to_string),
            from: from.map(str::to_string),
            app_env: None,
        }
    }

    #[test]
    fn variables_win_over_rullst_toml_and_other_sections_are_ignored() {
        let toml = r#"
[app]
env = "staging"
[mail]
driver = "resend" # comment
from = "Acme Billing <billing@acme.example>"
[other]
from = "ignored@example.com"
"#;
        let mut from_file = MailSettings::default();
        from_file.fill_from_rullst_toml(toml);
        assert_eq!(from_file.driver_name().unwrap(), "resend");
        assert_eq!(
            from_file.default_sender().unwrap(),
            Some("Acme Billing <billing@acme.example>")
        );
        assert_eq!(from_file.app_env.as_deref(), Some("staging"));

        let mut variables = settings(Some("log"), Some("ops@acme.example"));
        variables.fill_from_rullst_toml(toml);
        assert_eq!(variables.driver_name().unwrap(), "log");
        assert_eq!(
            variables.default_sender().unwrap(),
            Some("ops@acme.example")
        );

        let mut empty = MailSettings::default();
        empty.fill_from_rullst_toml("[mailer]\nfrom = \"\"\n");
        assert_eq!(empty.default_sender().unwrap(), None);
    }

    #[test]
    fn default_sender_uses_the_pre_flight_sender_rules() {
        for valid in [
            "billing@acme.example",
            "Acme Billing <billing@acme.example>",
            "\"Acme, Inc.\" <billing@acme.example>",
        ] {
            assert_eq!(
                settings(None, Some(valid)).default_sender().unwrap(),
                Some(valid)
            );
        }
        for invalid in [
            "billing@acme.example\r\nBcc: attacker@example.com",
            "Acme <billing@acme.example",
            "first@acme.example, second@acme.example",
            "not an address",
        ] {
            let error = settings(None, Some(invalid)).default_sender().unwrap_err();
            assert!(matches!(&error, MailError::ConfigError(text) if text.contains("MAIL_FROM")));
        }
    }

    #[test]
    fn an_explicit_from_wins_but_a_bad_default_still_fails() {
        let configured = settings(None, Some("Acme <billing@acme.example>"));
        let defaulted = configured
            .apply_default_sender(Message::new().to("member@example.com"))
            .unwrap();
        assert_eq!(
            defaulted.from.as_deref(),
            Some("Acme <billing@acme.example>")
        );
        let explicit = configured
            .apply_default_sender(Message::new().from("team@acme.example"))
            .unwrap();
        assert_eq!(explicit.from.as_deref(), Some("team@acme.example"));
        assert_eq!(
            MailSettings::default()
                .apply_default_sender(Message::new())
                .unwrap()
                .from,
            None
        );
        assert!(matches!(
            settings(None, Some("bad\nsender@acme.example"))
                .apply_default_sender(Message::new().from("team@acme.example")),
            Err(MailError::ConfigError(_))
        ));
    }
}
