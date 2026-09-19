use rullst_mail::{
    AccountEvent, AccountMail, ActionLink, DeliveryPipeline, MailLocale, MailPurpose,
};

const URL: &str = "https://app.example/action?token=opaque_code&source=account";

fn link() -> ActionLink {
    ActionLink::new(URL, "https://app.example").unwrap()
}

#[test]
fn lifecycle_catalog_keeps_localized_purpose_links_and_versions_through_delivery() {
    let cases = [
        (
            AccountEvent::EmailVerification {
                link: link(),
                expires_in_minutes: 15,
            },
            "account.email-verification.v1",
            MailPurpose::Security,
            true,
            [
                "Verify your email",
                "Confirme seu e-mail",
                "Verifica tu correo",
            ],
        ),
        (
            AccountEvent::EmailVerified,
            "account.email-verified.v1",
            MailPurpose::Security,
            false,
            ["Email verified", "E-mail confirmado", "Correo verificado"],
        ),
        (
            AccountEvent::NewDevice,
            "account.new-device.v1",
            MailPurpose::Security,
            false,
            [
                "Sign-in from a new device",
                "Acesso de um novo dispositivo",
                "Acceso desde un dispositivo nuevo",
            ],
        ),
        (
            AccountEvent::SecurityAlert,
            "account.security-alert.v1",
            MailPurpose::Security,
            false,
            [
                "Account security alert",
                "Alerta de segurança da conta",
                "Alerta de seguridad de la cuenta",
            ],
        ),
        (
            AccountEvent::EmailChangeRequested {
                link: link(),
                expires_in_minutes: 15,
            },
            "account.email-change-requested.v1",
            MailPurpose::Security,
            true,
            [
                "Confirm your email change",
                "Confirme a alteração de e-mail",
                "Confirma el cambio de correo",
            ],
        ),
        (
            AccountEvent::EmailChanged,
            "account.email-changed.v1",
            MailPurpose::Security,
            false,
            [
                "Your email was changed",
                "Seu e-mail foi alterado",
                "Tu correo fue cambiado",
            ],
        ),
        (
            AccountEvent::AccountClosed,
            "account.closed.v1",
            MailPurpose::AccountLifecycle,
            false,
            ["Account closed", "Conta encerrada", "Cuenta cerrada"],
        ),
        (
            AccountEvent::ExportReady { link: link() },
            "account.export-ready.v1",
            MailPurpose::AccountLifecycle,
            true,
            [
                "Your export is ready",
                "Sua exportação está pronta",
                "Tu exportación está lista",
            ],
        ),
    ];
    for (event, template, purpose, has_link, titles) in cases {
        for (index, locale) in [MailLocale::En, MailLocale::PtBr, MailLocale::Es]
            .into_iter()
            .enumerate()
        {
            let mail = AccountMail::new("member@example.com", "My <App>", locale, event.clone())
                .unwrap()
                .from("accounts@example.com");
            assert_eq!(mail.template_id(), template);
            assert_eq!(mail.purpose(), purpose);
            assert!(!format!("{mail:?}").contains("opaque_code"));
            let message = mail.into_message();
            assert_eq!(message.subject, format!("My <App>: {}", titles[index]));
            assert_eq!(message.from.as_deref(), Some("accounts@example.com"));
            let html = message.body_html.as_ref().unwrap();
            let text = message.body_text.as_ref().unwrap();
            assert!(html.contains("My &lt;App&gt;"));
            assert!(text.contains(titles[index]));
            assert_eq!(text.contains(URL), has_link);
            assert_eq!(
                html.contains("token=opaque_code&amp;source=account"),
                has_link
            );
            assert_eq!(html.contains("<a href="), has_link);
            let safety_notice = [
                "Never share this message",
                "Não compartilhe esta mensagem",
                "No compartas este mensaje",
            ][index];
            assert_eq!(
                text.contains(safety_notice),
                purpose == MailPurpose::Security
            );
            assert!(message.unsubscribe_url.is_none());
            assert!(!html.contains("<img"));
            let prepared = DeliveryPipeline::prepare(&message).unwrap().into_message();
            assert_eq!(prepared.body_html, message.body_html);
            assert_eq!(prepared.body_text, message.body_text);
        }
    }
}

#[test]
fn durable_reset_renders_the_same_absolute_expiry_on_every_retry() {
    let expiry = chrono::DateTime::parse_from_rfc3339("2026-09-18T12:20:00+00:00")
        .unwrap()
        .to_utc();
    for (preference, expected, title) in [
        ("PT-BR", MailLocale::PtBr, "Redefina sua senha"),
        ("es-419", MailLocale::Es, "Restablece tu contraseña"),
        ("unsupported", MailLocale::En, "Reset your password"),
    ] {
        let locale = MailLocale::from_preference(preference);
        assert_eq!(locale, expected);
        let event = AccountEvent::PasswordResetAt {
            link: link(),
            expires_at: expiry,
        };
        let first = AccountMail::new("member@example.com", "App", locale, event.clone())
            .unwrap()
            .into_message();
        let retry = AccountMail::new("member@example.com", "App", locale, event)
            .unwrap()
            .into_message();
        assert_eq!(first.subject, format!("App: {title}"));
        assert_eq!(first.body_html, retry.body_html);
        assert_eq!(first.body_text, retry.body_text);
        assert!(
            first
                .body_text
                .unwrap()
                .contains("2026-09-18T12:20:00+00:00")
        );
        assert!(
            first
                .body_html
                .unwrap()
                .contains("2026-09-18T12:20:00+00:00")
        );
    }
}
