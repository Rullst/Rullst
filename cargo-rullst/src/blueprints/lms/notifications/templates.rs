pub(super) const NOTIFICATION_TEMPLATE_SERVICE: &str = r##"use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RenderedNotification {
    pub locale: String,
    pub title: String,
    pub body: String,
}

#[derive(Debug)]
pub enum NotificationTemplateError {
    UnsupportedKey,
    InvalidPayload(serde_json::Error),
    InvalidField(&'static str),
}

impl std::fmt::Display for NotificationTemplateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedKey => formatter.write_str("unsupported notification localization key"),
            Self::InvalidPayload(error) => write!(formatter, "invalid notification template payload: {error}"),
            Self::InvalidField(field) => write!(formatter, "invalid notification template field: {field}"),
        }
    }
}

impl std::error::Error for NotificationTemplateError {}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AchievementAwardedV1 {
    schema_version: i32,
    achievement_code: String,
    recorded_actor_user_id: i32,
}

fn effective_locale(requested: &str) -> &'static str {
    let primary = requested.split('-').next().unwrap_or_default();
    if primary.eq_ignore_ascii_case("pt") {
        "pt-BR"
    } else if primary.eq_ignore_ascii_case("es") {
        "es"
    } else {
        "en"
    }
}

fn valid_code(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.')
        })
}

pub fn render_notification(
    requested_locale: &str,
    localization_key: &str,
    payload_json: &str,
) -> Result<RenderedNotification, NotificationTemplateError> {
    if localization_key != "academy.achievement.awarded" {
        return Err(NotificationTemplateError::UnsupportedKey);
    }
    let payload: AchievementAwardedV1 = serde_json::from_str(payload_json)
        .map_err(NotificationTemplateError::InvalidPayload)?;
    if payload.schema_version != 1
        || payload.recorded_actor_user_id <= 0
        || !valid_code(&payload.achievement_code)
    {
        return Err(NotificationTemplateError::InvalidField("achievement payload"));
    }
    let locale = effective_locale(requested_locale);
    let (title, body) = match locale {
        "pt-BR" => (
            "Nova conquista desbloqueada".to_string(),
            format!("Você desbloqueou a conquista {}.", payload.achievement_code),
        ),
        "es" => (
            "Nuevo logro desbloqueado".to_string(),
            format!("Desbloqueaste el logro {}.", payload.achievement_code),
        ),
        _ => (
            "New achievement unlocked".to_string(),
            format!("You unlocked the {} achievement.", payload.achievement_code),
        ),
    };
    Ok(RenderedNotification {
        locale: locale.to_string(),
        title,
        body,
    })
}
"##;

#[cfg(test)]
mod tests {
    use super::NOTIFICATION_TEMPLATE_SERVICE;

    #[test]
    fn generated_templates_are_closed_versioned_and_have_three_locales() {
        assert!(NOTIFICATION_TEMPLATE_SERVICE.contains("#[serde(deny_unknown_fields)]"));
        assert!(NOTIFICATION_TEMPLATE_SERVICE.contains("schema_version != 1"));
        assert!(NOTIFICATION_TEMPLATE_SERVICE.contains("\"pt-BR\""));
        assert!(NOTIFICATION_TEMPLATE_SERVICE.contains("\"es\""));
        assert!(NOTIFICATION_TEMPLATE_SERVICE.contains("\"en\""));
        assert!(NOTIFICATION_TEMPLATE_SERVICE.contains("UnsupportedKey"));
    }
}
