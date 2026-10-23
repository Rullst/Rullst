use rullst_mail::{
    AccountEvent, AccountMail, ActionLink, DeliveryPipeline, MailDriver, MailFactory, MailLocale,
    MailPurpose, MemoryDriver, Message,
};

#[test]
fn factory_reset_survives_mandatory_pipeline_and_repeated_preparation() {
    let url = "https://app.example/reset?token=abc123&source=account";
    let mut message = MailFactory::fake_password_reset("member@example.com", url, 15);
    message.subject.push_str(" password=must-not-leak");
    message
        .body_html
        .as_mut()
        .unwrap()
        .push_str("<p>api_key=private-key secret=private-secret token=unrelated</p>");
    message
        .body_text
        .as_mut()
        .unwrap()
        .push_str("\npassword=private-password");
    for _ in 0..3 {
        message = DeliveryPipeline::prepare(&message).unwrap().into_message();
    }
    assert!(message.body_text.as_ref().unwrap().contains(url));
    assert!(
        message
            .body_html
            .as_ref()
            .unwrap()
            .contains("token=abc123&amp;source=account")
    );
    let serialized = serde_json::to_string(&message).unwrap();
    for secret in [
        "must-not-leak",
        "private-key",
        "private-secret",
        "private-password",
        "unrelated",
    ] {
        assert!(!serialized.contains(secret));
    }
    assert!(!format!("{message:?}").contains("abc123"));
    assert!(!format!("{message:?}").contains("member@example.com"));
}

#[test]
fn url_context_does_not_exempt_arbitrary_secrets() {
    for text in [
        "password=https://app.example/reset?token=accidental",
        "http://app.example/reset?token=accidental",
        "https://user:pass@app.example/reset?token=accidental",
        "https://app.example/reset?api_key=accidental",
        "https://app.example/reset?token=AKIA1234567890ABCDEF",
        "https://app.example/token=accidental",
        "https://app.example/reset#token=accidental",
        "token=accidental",
    ] {
        let message = Message::new().text(text).sanitize_secrets();
        let body = message.body_text.unwrap();
        assert!(!body.contains("accidental"), "{text}");
        assert!(!body.contains("AKIA1234567890ABCDEF"));
    }
}

#[test]
fn typed_links_bind_configured_origin_and_redact_debug() {
    let url = "https://app.example/reset?token=abc123";
    let link = ActionLink::new(url, "https://app.example").unwrap();
    assert_eq!(link.expose_url(), url);
    assert!(!format!("{link:?}").contains("abc123"));
    for (url, origin) in [
        (url, "https://other.example"),
        (url, "https://app.example:8443"),
        (url, "https://app.example/config"),
        (
            "http://app.example/reset?token=abc123",
            "http://app.example",
        ),
        (
            "https://app.example/reset?secret=bad",
            "https://app.example",
        ),
        (
            "https://user:password@app.example/reset",
            "https://app.example",
        ),
        (
            "https://app.example/reset\r\nX-Test: injected",
            "https://app.example",
        ),
    ] {
        assert!(ActionLink::new(url, origin).is_err());
    }
    assert!(
        ActionLink::new(
            "http://127.0.0.1:8080/reset?token=abc",
            "http://127.0.0.1:8080"
        )
        .is_ok()
    );
}

#[tokio::test]
async fn account_templates_deliver_both_parts_without_tracking() {
    let driver = MemoryDriver::new();
    for locale in [MailLocale::En, MailLocale::PtBr, MailLocale::Es] {
        let link = ActionLink::new(
            "https://app.example/reset?token=abc123",
            "https://app.example",
        )
        .unwrap();
        let mail = AccountMail::new(
            "member@example.com",
            "My <App>",
            locale,
            AccountEvent::PasswordReset {
                link,
                expires_in_minutes: 20,
            },
        )
        .unwrap();
        assert_eq!(mail.purpose(), MailPurpose::Security);
        assert_eq!(mail.template_id(), "account.password-reset.v1");
        assert!(!format!("{mail:?}").contains("abc123"));
        let message = mail.into_message();
        assert!(message.body_text.as_ref().unwrap().contains("token=abc123"));
        assert!(
            message
                .body_html
                .as_ref()
                .unwrap()
                .contains("My &lt;App&gt;")
        );
        assert!(!message.body_html.as_ref().unwrap().contains("<img"));
        assert!(message.unsubscribe_url.is_none());
        driver.send(&message).await.unwrap();
    }
    let welcome = AccountMail::new(
        "member@example.com",
        "App",
        MailLocale::PtBr,
        AccountEvent::Welcome,
    )
    .unwrap()
    .into_message();
    assert!(welcome.subject.contains("Boas-vindas"));
    assert!(!welcome.body_html.unwrap().contains("example.com/verify"));
    assert_eq!(MailLocale::from_preference("unknown"), MailLocale::En);
}

#[test]
fn reset_templates_reject_unsafe_expiry_and_header_injection() {
    for minutes in [0, 1441, u32::MAX] {
        let link = ActionLink::new(
            "https://app.example/reset?token=abc123",
            "https://app.example",
        )
        .unwrap();
        assert!(
            AccountMail::new(
                "member@example.com",
                "App",
                MailLocale::En,
                AccountEvent::PasswordReset {
                    link,
                    expires_in_minutes: minutes
                }
            )
            .is_err()
        );
    }
    assert!(
        AccountMail::new(
            "member@example.com",
            "App\r\nBcc: other@example.com",
            MailLocale::En,
            AccountEvent::Welcome
        )
        .is_err()
    );
}

fn reset_text(minutes: u32, locale: MailLocale) -> Result<String, rullst_mail::MailError> {
    let link = ActionLink::new(
        "https://app.example/reset?token=abc123",
        "https://app.example",
    )
    .unwrap();
    AccountMail::new(
        "member@example.com",
        "App",
        locale,
        AccountEvent::PasswordReset {
            link,
            expires_in_minutes: minutes,
        },
    )
    .map(|mail| mail.into_message().body_text.unwrap())
}

#[test]
fn reset_links_state_a_lifetime_of_up_to_twenty_four_hours() {
    for (minutes, english, portuguese, spanish) in [
        (1, "1 minute", "1 minuto", "1 minuto"),
        (10, "10 minutes", "10 minutos", "10 minutos"),
        (30, "30 minutes", "30 minutos", "30 minutos"),
        (60, "1 hour", "1 hora", "1 hora"),
        (90, "90 minutes", "90 minutos", "90 minutos"),
        (120, "2 hours", "2 horas", "2 horas"),
        (1440, "24 hours", "24 horas", "24 horas"),
    ] {
        assert!(
            reset_text(minutes, MailLocale::En)
                .unwrap()
                .contains(&format!("This link expires in {english}.\n")),
            "{minutes}"
        );
        assert!(
            reset_text(minutes, MailLocale::PtBr)
                .unwrap()
                .contains(&format!("Este link expira em {portuguese}.\n"))
        );
        assert!(
            reset_text(minutes, MailLocale::Es)
                .unwrap()
                .contains(&format!("Este enlace caduca en {spanish}.\n"))
        );
    }
    let error = reset_text(1441, MailLocale::En).unwrap_err().to_string();
    assert!(error.contains("1–1440 minutes"), "{error}");
    // Verification and email-change links keep the 30-minute ceiling.
    let events: [fn(ActionLink) -> AccountEvent; 2] = [
        |link| AccountEvent::EmailVerification {
            link,
            expires_in_minutes: 31,
        },
        |link| AccountEvent::EmailChangeRequested {
            link,
            expires_in_minutes: 31,
        },
    ];
    for event in events {
        let link =
            ActionLink::new("https://app.example/verify?t=1", "https://app.example").unwrap();
        let error = AccountMail::new("member@example.com", "App", MailLocale::En, event(link))
            .unwrap_err()
            .to_string();
        assert!(error.contains("1–30 minutes"), "{error}");
    }
    let link = ActionLink::new("https://app.example/verify?t=1", "https://app.example").unwrap();
    assert!(
        AccountMail::new(
            "member@example.com",
            "App",
            MailLocale::En,
            AccountEvent::EmailVerification {
                link,
                expires_in_minutes: 30,
            },
        )
        .is_ok()
    );
}
