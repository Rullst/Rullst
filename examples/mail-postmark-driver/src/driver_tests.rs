use super::*;

#[test]
fn a_schedule_up_to_now_is_sent_now() {
    let now = Utc::now();
    let message = Message::new()
        .to("ada@example.com")
        .from("hello@acme.example")
        .subject("Hello")
        .text("body");
    assert!(payload(&message.clone().send_at(now), now).is_ok());
    let later = now + chrono::TimeDelta::nanoseconds(1);
    assert!(matches!(
        payload(&message.send_at(later), now),
        Err(MailError::ConfigError(_))
    ));
}
