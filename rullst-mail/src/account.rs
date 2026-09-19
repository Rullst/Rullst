//! Deterministic account-lifecycle mail. No marketing consent or tracking is added.

use crate::{ActionLink, DeliveryPipeline, MailError, Message, escape_html};

/// Application-selected language; unknown preferences deterministically use English.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum MailLocale {
    En,
    PtBr,
    Es,
}

impl MailLocale {
    pub fn from_preference(preference: &str) -> Self {
        match preference.to_ascii_lowercase().as_str() {
            "pt" | "pt-br" => Self::PtBr,
            "es" | "es-es" | "es-419" => Self::Es,
            _ => Self::En,
        }
    }

    fn select<'a>(self, en: &'a str, pt: &'a str, es: &'a str) -> &'a str {
        match self {
            Self::En => en,
            Self::PtBr => pt,
            Self::Es => es,
        }
    }
}

/// Purpose of a message, independent from marketing permissions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum MailPurpose {
    Security,
    AccountLifecycle,
    Billing,
    Marketing,
}

/// Minimal typed events. Action links have redacted Debug output and no Serialize
/// implementation; never copy them into generic event/telemetry payloads.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum AccountEvent {
    Welcome,
    PasswordReset {
        link: ActionLink,
        expires_in_minutes: u32,
    },
    /// Absolute expiry for durable retries: the rendered payload remains identical.
    PasswordResetAt {
        link: ActionLink,
        expires_at: chrono::DateTime<chrono::Utc>,
    },
    PasswordChanged,
    EmailVerification {
        link: ActionLink,
        expires_in_minutes: u32,
    },
    EmailVerified,
    NewDevice,
    SecurityAlert,
    EmailChangeRequested {
        link: ActionLink,
        expires_in_minutes: u32,
    },
    EmailChanged,
    AccountClosed,
    ExportReady {
        link: ActionLink,
    },
}

impl AccountEvent {
    pub fn purpose(&self) -> MailPurpose {
        match self {
            Self::Welcome | Self::AccountClosed | Self::ExportReady { .. } => {
                MailPurpose::AccountLifecycle
            }
            _ => MailPurpose::Security,
        }
    }

    pub fn template_id(&self) -> &'static str {
        match self {
            Self::Welcome => "account.welcome.v1",
            Self::PasswordReset { .. } | Self::PasswordResetAt { .. } => {
                "account.password-reset.v1"
            }
            Self::PasswordChanged => "account.password-changed.v1",
            Self::EmailVerification { .. } => "account.email-verification.v1",
            Self::EmailVerified => "account.email-verified.v1",
            Self::NewDevice => "account.new-device.v1",
            Self::SecurityAlert => "account.security-alert.v1",
            Self::EmailChangeRequested { .. } => "account.email-change-requested.v1",
            Self::EmailChanged => "account.email-changed.v1",
            Self::AccountClosed => "account.closed.v1",
            Self::ExportReady { .. } => "account.export-ready.v1",
        }
    }

    fn action(&self) -> Option<(&ActionLink, Option<u32>)> {
        match self {
            Self::PasswordReset {
                link,
                expires_in_minutes,
            }
            | Self::EmailVerification {
                link,
                expires_in_minutes,
            }
            | Self::EmailChangeRequested {
                link,
                expires_in_minutes,
            } => Some((link, Some(*expires_in_minutes))),
            Self::ExportReady { link } => Some((link, None)),
            Self::PasswordResetAt { link, .. } => Some((link, None)),
            _ => None,
        }
    }

    fn title(&self, locale: MailLocale) -> &str {
        match self {
            Self::Welcome => locale.select("Welcome!", "Boas-vindas!", "¡Bienvenido!"),
            Self::PasswordReset { .. } | Self::PasswordResetAt { .. } => locale.select(
                "Reset your password",
                "Redefina sua senha",
                "Restablece tu contraseña",
            ),
            Self::PasswordChanged => locale.select(
                "Your password was changed",
                "Sua senha foi alterada",
                "Tu contraseña fue cambiada",
            ),
            Self::EmailVerification { .. } => locale.select(
                "Verify your email",
                "Confirme seu e-mail",
                "Verifica tu correo",
            ),
            Self::EmailVerified => {
                locale.select("Email verified", "E-mail confirmado", "Correo verificado")
            }
            Self::NewDevice => locale.select(
                "Sign-in from a new device",
                "Acesso de um novo dispositivo",
                "Acceso desde un dispositivo nuevo",
            ),
            Self::SecurityAlert => locale.select(
                "Account security alert",
                "Alerta de segurança da conta",
                "Alerta de seguridad de la cuenta",
            ),
            Self::EmailChangeRequested { .. } => locale.select(
                "Confirm your email change",
                "Confirme a alteração de e-mail",
                "Confirma el cambio de correo",
            ),
            Self::EmailChanged => locale.select(
                "Your email was changed",
                "Seu e-mail foi alterado",
                "Tu correo fue cambiado",
            ),
            Self::AccountClosed => {
                locale.select("Account closed", "Conta encerrada", "Cuenta cerrada")
            }
            Self::ExportReady { .. } => locale.select(
                "Your export is ready",
                "Sua exportação está pronta",
                "Tu exportación está lista",
            ),
        }
    }
}

/// Safe template output. Debug contains only purpose, locale and template version.
#[derive(Clone)]
pub struct AccountMail {
    message: Message,
    purpose: MailPurpose,
    locale: MailLocale,
    template_id: &'static str,
}

impl std::fmt::Debug for AccountMail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AccountMail")
            .field("purpose", &self.purpose)
            .field("locale", &self.locale)
            .field("template_id", &self.template_id)
            .finish()
    }
}

impl AccountMail {
    /// Builds deterministic HTML/text with a validated server-owned action link.
    /// These templates do not generate tokens or change authentication state.
    pub fn new(
        to: impl Into<String>,
        application_name: impl Into<String>,
        locale: MailLocale,
        event: AccountEvent,
    ) -> Result<Self, MailError> {
        let app = application_name.into();
        if app.is_empty() || app.len() > 128 || app.chars().any(char::is_control) {
            return Err(MailError::ValidationError(
                "application name must contain 1–128 safe bytes".into(),
            ));
        }
        let title = event.title(locale);
        let mut text = format!("{app}\n{title}\n");
        let mut html = format!(
            "<h1>{}</h1><p>{}</p>",
            escape_html(&app),
            escape_html(title)
        );
        if let AccountEvent::PasswordResetAt { expires_at, .. } = &event {
            let expiry = format!(
                "{} {}",
                locale.select("Expires at", "Expira em", "Caduca el"),
                expires_at.to_rfc3339()
            );
            text.push_str(&format!("{expiry}\n"));
            html.push_str(&format!("<p>{expiry}</p>"));
        }
        if let Some((link, expiry)) = event.action() {
            if let Some(minutes) = expiry {
                if !(1..=30).contains(&minutes) {
                    return Err(MailError::ValidationError(
                        "remaining account action lifetime must be 1–30 minutes".into(),
                    ));
                }
                let expiry = format!(
                    "{} {minutes} {}.",
                    locale.select(
                        "This link expires in",
                        "Este link expira em",
                        "Este enlace caduca en"
                    ),
                    locale.select("minutes", "minutos", "minutos")
                );
                text.push_str(&format!("{expiry}\n"));
                html.push_str(&format!("<p>{expiry}</p>"));
            }
            text.push_str(link.expose_url());
            html.push_str(&format!(
                "<p><a href=\"{}\">{}</a></p>",
                escape_html(link.expose_url()),
                escape_html(title)
            ));
        }
        if event.purpose() == MailPurpose::Security {
            let notice = locale.select(
                "If you did not request this action, contact support through the application. Never share this message or its links.",
                "Se você não solicitou esta ação, contate o suporte pelo aplicativo. Não compartilhe esta mensagem ou seus links.",
                "Si no solicitaste esta acción, contacta al soporte desde la aplicación. No compartas este mensaje ni sus enlaces.");
            text.push_str(&format!("\n{notice}"));
            html.push_str(&format!("<p>{}</p>", escape_html(notice)));
        }
        let message = Message::new()
            .to(to)
            .subject(format!("{app}: {title}"))
            .text(text)
            .html(html);
        let message = DeliveryPipeline::prepare(&message)?.into_message();
        Ok(Self {
            message,
            purpose: event.purpose(),
            locale,
            template_id: event.template_id(),
        })
    }

    pub fn purpose(&self) -> MailPurpose {
        self.purpose
    }
    pub fn template_id(&self) -> &'static str {
        self.template_id
    }

    /// Sets the application-owned verified sender; transport pre-flight validates it.
    pub fn from(mut self, sender: impl Into<String>) -> Self {
        self.message.from = Some(sender.into());
        self
    }

    /// Sensitive delivery content. Do not log or place in an unencrypted queue.
    pub fn into_message(self) -> Message {
        self.message
    }
}
