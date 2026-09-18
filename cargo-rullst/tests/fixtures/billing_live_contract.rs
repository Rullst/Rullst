#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::*;
use std::sync::{Arc, Mutex, atomic::{AtomicBool, AtomicUsize, Ordering}};
use gateway::{Customer, Created, Checkout, Subscription};

#[derive(Clone, Default)]
struct Fake(Arc<Remote>);
#[derive(Default)]
struct Remote {
    customer_keys: Mutex<std::collections::HashSet<String>>,
    checkout_keys: Mutex<std::collections::HashSet<String>>,
    create_calls: AtomicUsize,
    reads: AtomicUsize,
    lose_customer: AtomicBool,
    lose_checkout: AtomicBool,
    complete: AtomicBool,
    canceled: AtomicBool,
    pause_read: AtomicBool,
    entered: tokio::sync::Notify,
    resume: tokio::sync::Notify,
}
impl Fake {
    fn checkout(request: &StripeCheckoutRequest, complete: bool) -> Checkout {
        Checkout { id: format!("cs_{}", request.idempotency_key()), status: if complete { "complete" } else { "open" }.into(),
            url: (!complete).then(|| format!("https://checkout.stripe.com/c/pay/cs_{}", request.idempotency_key())),
            subscription: complete.then(|| format!("sub_{}", request.idempotency_key())) }
    }
}
impl Gateway for Fake {
    async fn verify_account(&self, account: &str, mode: bool) -> Result<()> {
        assert_eq!(account, "acct_contract"); assert!(!mode); Ok(())
    }
    async fn create_customer(&self, request: &StripeCustomerRequest) -> Result<Customer> {
        self.0.customer_keys.lock().unwrap().insert(request.idempotency_key().into());
        assert!(request.email().is_none());
        if self.0.lose_customer.swap(false, Ordering::SeqCst) { return Err(UNAVAILABLE); }
        Ok(Customer { id: format!("cus_{}", request.owner_reference()), mode: false })
    }
    async fn find_bound_customer(&self, request: &StripeCustomerRequest) -> Result<Option<Customer>> {
        Ok(self.0.customer_keys.lock().unwrap().contains(request.idempotency_key()).then(||
            Customer { id: format!("cus_{}", request.owner_reference()), mode: false }))
    }
    async fn create_subscription_checkout(&self, request: &StripeCheckoutRequest) -> Result<Created> {
        self.0.create_calls.fetch_add(1, Ordering::SeqCst);
        self.0.checkout_keys.lock().unwrap().insert(request.idempotency_key().into());
        if self.0.lose_checkout.swap(false, Ordering::SeqCst) { return Err(UNAVAILABLE); }
        Ok(Created { id: format!("cs_{}", request.idempotency_key()),
            url: format!("https://checkout.stripe.com/c/pay/cs_{}", request.idempotency_key()), mode: false,
            digest: request.request_digest() })
    }
    async fn retrieve_checkout(&self, request: &StripeCheckoutRequest, id: &str, mode: bool) -> Result<Checkout> {
        assert!(!mode); assert_eq!(id, format!("cs_{}", request.idempotency_key()));
        self.0.reads.fetch_add(1, Ordering::SeqCst);
        Ok(Self::checkout(request, self.0.complete.load(Ordering::SeqCst)))
    }
    async fn find_checkout(&self, request: &StripeCheckoutRequest, _: i64, subscription: Option<&str>, mode: bool) -> Result<Option<Checkout>> {
        assert!(!mode);
        if let Some(id) = subscription { assert_eq!(id, format!("sub_{}", request.idempotency_key())); }
        Ok(self.0.checkout_keys.lock().unwrap().contains(request.idempotency_key())
            .then(|| Self::checkout(request, self.0.complete.load(Ordering::SeqCst))))
    }
    async fn retrieve_subscription(&self, request: &StripeSubscriptionLookup, _: &std::collections::HashSet<String>) -> Result<Subscription> {
        let canceled = self.0.canceled.load(Ordering::SeqCst);
        if self.0.pause_read.swap(false, Ordering::SeqCst) {
            self.0.entered.notify_one(); self.0.resume.notified().await;
        }
        Ok(Subscription { event: rullst::capital::WebhookEvent {
            subscription_id: request.subscription_id().into(), customer_id: request.customer_id().into(), customer_email: String::new(),
            plan_id: request.price_id().into(), status: if canceled { rullst::capital::SubscriptionStatus::Canceled } else { rullst::capital::SubscriptionStatus::Active },
            ends_at: Some(1900000000),
        }, status: if canceled { "canceled" } else { "active" }.into() })
    }
}
fn config() -> BillingConfig {
    BillingConfig { provider: "stripe".into(), api_key: "sk_test_contract".into(),
        webhook_secret: "whsec_contract_abcdefghijklmnopqrstuvwxyz0123456789".into(),
        redirect_url: "https://app.example/return".into(), store_id: None,
        allowed_plan_ids: ["price_pro".into(), "price_other".into()].into_iter().collect() }
}
fn notice(state: &State, id: &str, checkout: bool) -> events::Notice {
    let attempt = state.attempt.as_ref().unwrap();
    events::Notice { receipt: Receipt { id: id.into(), digest: format!("digest_{id}") },
        owner: state.reference.clone(), customer: state.customer.clone().unwrap(),
        session: checkout.then(|| format!("cs_{}", attempt.key)),
        subscription: Some(format!("sub_{}", attempt.key)), attempt: checkout.then(|| attempt.key.clone()) }
}
async fn process(fake: &Fake, notice: events::Notice) -> Result<Response> {
    events::process(&config(), fake, "acct_contract:test", "acct_contract", false, notice).await
}
async fn load(owner: i64) -> State { store::owner("acct_contract:test", owner).await.unwrap().unwrap() }
async fn sql(statement: &str) {
    __EXECUTE_SQL__
}

#[tokio::test]
async fn durable_live_billing_contract() {
    __INITIALIZE_DB__
    if std::env::var_os("BILLING_RESTART_CHECK").is_some() {
        let state = load(100).await;
        assert_eq!(state.entitlement.as_ref().unwrap().status, "canceled");
        assert!(store::receipt(&state.scope, "evt_newer").await.unwrap().is_some());
        assert!(store::receipt(&state.scope, "evt_older").await.unwrap().is_none());
        return;
    }
    let config = config();
    let fake = Fake::default();
    let identity = BillingIdentity { owner_id: 100, email: "original@example.com".into() };
    fake.0.lose_customer.store(true, Ordering::SeqCst);
    assert!(checkout_with(&config, &identity, "price_pro", &fake).await.is_err());
    let pending = load(100).await;
    assert!(pending.customer.is_none()); assert!(pending.attempt.is_none());
    fake.0.lose_checkout.store(true, Ordering::SeqCst);
    let changed_contact = BillingIdentity { owner_id: 100, email: "changed@example.com".into() };
    assert!(checkout_with(&config, &changed_contact, "price_pro", &fake).await.is_err());
    let pending = load(100).await;
    assert!(pending.customer.is_some()); assert!(pending.attempt.as_ref().unwrap().session.is_none());
    let url = checkout_with(&config, &identity, "price_pro", &fake).await.unwrap();
    assert!(url.starts_with("https://checkout.stripe.com/"));
    assert_eq!(fake.0.customer_keys.lock().unwrap().len(), 1);
    assert_eq!(fake.0.checkout_keys.lock().unwrap().len(), 1);
    let creates = fake.0.create_calls.load(Ordering::SeqCst);
    assert_eq!(checkout_with(&config, &changed_contact, "price_pro", &fake).await.unwrap(), url);
    assert_eq!(fake.0.create_calls.load(Ordering::SeqCst), creates);
    assert_eq!(checkout_with(&config, &identity, "price_other", &fake).await, Err(CONFLICT));
    assert_eq!(fake.0.create_calls.load(Ordering::SeqCst), creates);

    // A different authenticated owner cannot take over a customer by sharing email.
    let other = BillingIdentity { owner_id: 101, email: identity.email.clone() };
    checkout_with(&config, &other, "price_pro", &fake).await.unwrap();
    assert_ne!(load(100).await.customer, load(101).await.customer);
    let mut forged = notice(&load(100).await, "evt_forged", true);
    forged.customer = load(101).await.customer.unwrap();
    assert_eq!(process(&fake, forged).await.unwrap_err(), CONFLICT);

    // Subscription event arrives before the successful POST response was persisted.
    let old = load(100).await; let mut lost = old.clone();
    lost.attempt.as_mut().unwrap().session = None;
    commit(&old, lost, None).await.unwrap();
    fake.0.complete.store(true, Ordering::SeqCst);
    process(&fake, notice(&load(100).await, "evt_early", false)).await.unwrap();
    let active = load(100).await;
    assert_eq!(active.entitlement.as_ref().unwrap().status, "active");
    assert!(active.attempt.as_ref().unwrap().session.is_some());
    let before = fake.0.reads.load(Ordering::SeqCst);
    process(&fake, notice(&active, "evt_early", false)).await.unwrap();
    assert_eq!(fake.0.reads.load(Ordering::SeqCst), before);
    let mut conflict = notice(&active, "evt_early", false); conflict.receipt.digest = "changed".into();
    assert_eq!(process(&fake, conflict).await.unwrap_err(), CONFLICT);

    // Roll back the event claim, entitlement and revision together on storage failure.
    fake.0.canceled.store(true, Ordering::SeqCst);
    sql("CREATE TRIGGER reject_live_event BEFORE INSERT ON billing_events BEGIN SELECT RAISE(ABORT, 'injected failure'); END").await;
    assert!(process(&fake, notice(&active, "evt_rollback", true)).await.is_err());
    assert_eq!(load(100).await.entitlement.unwrap().status, "active");
    assert!(store::receipt(&active.scope, "evt_rollback").await.unwrap().is_none());
    sql("DROP TRIGGER reject_live_event").await;
    process(&fake, notice(&load(100).await, "evt_rollback", true)).await.unwrap();
    assert_eq!(load(100).await.entitlement.unwrap().status, "canceled");

    // Capture an earlier active read, commit a newer cancellation, then release
    // the delayed read. Its stale revision must fail without claiming that event.
    fake.0.canceled.store(false, Ordering::SeqCst);
    fake.0.pause_read.store(true, Ordering::SeqCst);
    let earlier = notice(&load(100).await, "evt_older", false);
    let slow = fake.clone();
    let task = tokio::spawn(async move { process(&slow, earlier).await });
    fake.0.entered.notified().await;
    fake.0.canceled.store(true, Ordering::SeqCst);
    process(&fake, notice(&load(100).await, "evt_newer", false)).await.unwrap();
    fake.0.resume.notify_one();
    assert!(task.await.unwrap().is_err());
    assert_eq!(load(100).await.entitlement.unwrap().status, "canceled");
    assert!(store::receipt("acct_contract:test", "evt_older").await.unwrap().is_none());

    // Replacement checkout keeps old signed notifications from overwriting it.
    let retired = load(100).await;
    fake.0.complete.store(false, Ordering::SeqCst);
    fake.0.canceled.store(false, Ordering::SeqCst);
    checkout_with(&config, &identity, "price_other", &fake).await.unwrap();
    fake.0.complete.store(true, Ordering::SeqCst);
    process(&fake, notice(&load(100).await, "evt_replacement", true)).await.unwrap();
    let replacement = load(100).await;
    assert_eq!(replacement.entitlement.as_ref().unwrap().status, "active");
    assert_ne!(replacement.entitlement.as_ref().unwrap().subscription,
        retired.entitlement.as_ref().unwrap().subscription);
    fake.0.canceled.store(true, Ordering::SeqCst);
    process(&fake, notice(&retired, "evt_retired_checkout", true)).await.unwrap();
    process(&fake, notice(&retired, "evt_retired_subscription", false)).await.unwrap();
    assert_eq!(load(100).await.entitlement.unwrap().status, "active");
    process(&fake, notice(&load(100).await, "evt_current_cancel", false)).await.unwrap();
    assert_eq!(load(100).await.entitlement.unwrap().status, "canceled");

    // An old unknown attempt uses read-only recovery, never a fresh POST.
    let original = load(101).await; let mut old = original.clone();
    old.attempt.as_mut().unwrap().session = None;
    old.attempt.as_mut().unwrap().created = now().unwrap() - 2 * 24 * 3600;
    commit(&original, old, None).await.unwrap();
    fake.0.complete.store(false, Ordering::SeqCst);
    let creates = fake.0.create_calls.load(Ordering::SeqCst);
    checkout_with(&config, &other, "price_pro", &fake).await.unwrap();
    assert_eq!(fake.0.create_calls.load(Ordering::SeqCst), creates);
    let original = load(101).await; let mut unknown = original.clone();
    unknown.attempt.as_mut().unwrap().session = None;
    unknown.attempt.as_mut().unwrap().key = "unknown_ancient_attempt".into();
    commit(&original, unknown, None).await.unwrap();
    assert_eq!(checkout_with(&config, &other, "price_pro", &fake).await, Err(UNAVAILABLE));
    assert_eq!(fake.0.create_calls.load(Ordering::SeqCst), creates);
}
