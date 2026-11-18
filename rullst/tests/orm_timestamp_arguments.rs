//! The ORM's `Timestamp`, such as a model's field, passes straight to Mail's
//! `Message::send_at` and Capital's `Invoice::new`, which take
//! `impl Into<jiff::Timestamp>`.
#![cfg(all(
    feature = "orm",
    feature = "mail",
    feature = "capital",
    not(target_arch = "wasm32")
))]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use rullst::orm::Timestamp;

#[test]
fn an_orm_timestamp_schedules_mail_and_dates_an_invoice() {
    let at: Timestamp = "2026-11-18T09:30:00Z".parse().unwrap();

    let message = rullst::mail::Message::new().send_at(at);
    assert_eq!(message.send_at, Some(*at));

    let invoice = rullst::capital::Invoice::new("INV-1", "owner@example.com", at, "BRL");
    assert_eq!(invoice.date, *at);
    assert_eq!(invoice.date.as_second(), 1_794_994_200);
}
