#![allow(clippy::unwrap_used, clippy::expect_used)]
#[allow(dead_code, unused_imports)]
#[path = "email_login/support.rs"]
mod support;
use rullst::mail::{AccountEvent, AccountMail, ActionLink, MailDriver, MailLocale, MemoryDriver};
use support::*;

#[tokio::test]
async fn packaged_auth_notice_survives_mail_preparation_and_redeems_once() {
    let directory = tempfile::tempdir().unwrap();
    let url = sqlite_url(&directory.path().join("mail.db"));
    let namespace = unique();
    let clock = Clock::new();
    let service = EmailLoginService::initialize(url, keys(), config(&namespace))
        .await
        .unwrap();
    let (subject, email, _) = account(&service, &clock).await;
    let browser = BrowserBinding::generate().unwrap();
    let notice = issue(&service, &email, &browser, &clock).await;
    let event = AccountEvent::EmailLoginAt {
        link: ActionLink::new(notice.expose_link().to_string(), "https://app.example").unwrap(),
        expires_at: chrono::DateTime::from_timestamp(notice.expires_at(), 0).unwrap(),
    };
    let message = AccountMail::new(notice.recipient(), "School", MailLocale::PtBr, event)
        .unwrap()
        .into_message();
    let (driver, inbox) = MemoryDriver::isolated();
    driver
        .send_with_delivery_id(&message, notice.delivery_id())
        .await
        .unwrap();
    service.complete_notice(&notice, &clock).await.unwrap();
    let text = {
        let messages = inbox.lock().unwrap();
        assert_eq!(messages.len(), 1);
        messages[0].body_text.clone().unwrap()
    };
    assert!(text.contains("Confirme seu acesso"));
    assert!(!text.contains(browser.expose_cookie()));
    let link = text
        .lines()
        .find(|line| line.starts_with("https://app.example/login?token="))
        .unwrap();
    let token = url::Url::parse(link)
        .unwrap()
        .query_pairs()
        .find(|(key, _)| key == "token")
        .unwrap()
        .1
        .into_owned();
    let session = service.redeem(&token, &browser, &clock).await.unwrap();
    assert_eq!(session.subject(), subject);
    assert!(service.redeem(&token, &browser, &clock).await.is_err());
    service.close().await;
}
