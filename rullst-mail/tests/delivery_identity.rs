use async_trait::async_trait;
use rullst_mail::drivers::FailoverDriver;
use rullst_mail::{
    BoundedMailObserver, MailDriver, MailError, Message, ObservedMailDriver, TenantMailResolver,
};
use std::sync::{Arc, Mutex};

struct Capture(Arc<Mutex<Vec<String>>>);
#[async_trait]
impl MailDriver for Capture {
    async fn send(&self, _: &Message) -> Result<(), MailError> {
        panic!("stable identity must reach the underlying provider");
    }
    async fn send_with_delivery_id(&self, _: &Message, identity: &str) -> Result<(), MailError> {
        self.0.lock().unwrap().push(identity.into());
        Ok(())
    }
}

#[tokio::test]
async fn nested_delivery_wrappers_preserve_provider_idempotency() {
    let captured = Arc::new(Mutex::new(Vec::new()));
    let observer = BoundedMailObserver::new(4).unwrap();
    let driver = TenantMailResolver::with_default(FailoverDriver::new(
        ObservedMailDriver::try_new("capture", Capture(captured.clone()), observer.clone())
            .unwrap(),
    ));
    let message = Message::new()
        .to("member@example.com")
        .subject("Account notice");
    driver
        .send_with_delivery_id(&message, "opaque_delivery_123456")
        .await
        .unwrap();
    assert_eq!(*captured.lock().unwrap(), ["opaque_delivery_123456"]);
    assert_eq!(observer.snapshot().unwrap().observations().len(), 1);
    assert!(
        driver
            .send_with_delivery_id(&message, "invalid")
            .await
            .is_err()
    );
    assert_eq!(captured.lock().unwrap().len(), 1);
}
