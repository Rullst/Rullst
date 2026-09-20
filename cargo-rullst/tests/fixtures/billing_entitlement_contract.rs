//! Test-only provider protocol boundary; authentication, storage, gate and HTTP are real.
use super::*;
use rullst::server::{Body, Extension, Request};
use std::sync::{Arc, Mutex, atomic::{AtomicBool, AtomicI64, Ordering}};
use tower::ServiceExt;

#[derive(Default)]
struct Remote {
    status: Mutex<Option<String>>,
    plan: Mutex<Option<String>>,
    ends: AtomicI64,
    failed: AtomicBool,
    paused: AtomicBool,
    entered: tokio::sync::Notify,
    resume: tokio::sync::Notify,
}
#[derive(Clone, Default)]
struct Provider(Arc<Remote>);
impl Gateway for Provider {
    async fn verify_account(&self, account: &str, mode: bool) -> Result<()> {
        assert_eq!(account, "acct_contract"); assert!(!mode); Ok(())
    }
    async fn create_customer(&self, _: &StripeCustomerRequest) -> Result<gateway::Customer> { Err(UNAVAILABLE) }
    async fn find_bound_customer(&self, _: &StripeCustomerRequest) -> Result<Option<gateway::Customer>> { Err(UNAVAILABLE) }
    async fn create_subscription_checkout(&self, _: &StripeCheckoutRequest) -> Result<gateway::Created> { Err(UNAVAILABLE) }
    async fn retrieve_checkout(&self, _: &StripeCheckoutRequest, _: &str, _: bool) -> Result<gateway::Checkout> { Err(UNAVAILABLE) }
    async fn find_checkout(&self, _: &StripeCheckoutRequest, _: i64, _: Option<&str>, _: bool) -> Result<Option<gateway::Checkout>> { Err(UNAVAILABLE) }
    async fn retrieve_subscription(&self, request: &StripeSubscriptionLookup, _: &std::collections::HashSet<String>) -> Result<gateway::Subscription> {
        if self.0.failed.load(Ordering::SeqCst) { return Err(UNAVAILABLE); }
        let status = self.0.status.lock().unwrap().clone().unwrap_or_else(|| "active".into());
        let plan = self.0.plan.lock().unwrap().clone().unwrap_or_else(|| "price_pro".into());
        let ends = self.0.ends.load(Ordering::SeqCst);
        if self.0.paused.swap(false, Ordering::SeqCst) {
            self.0.entered.notify_one(); self.0.resume.notified().await;
        }
        Ok(gateway::Subscription { event: rullst::capital::WebhookEvent {
            subscription_id: request.subscription_id().into(), customer_id: request.customer_id().into(),
            customer_email: String::new(), plan_id: plan,
            status: rullst::capital::SubscriptionStatus::Active,
            ends_at: if ends == -1 { None } else if ends == 0 { Some(now().unwrap() + 3600) } else { Some(ends) },
        }, status })
    }
}
fn config() -> BillingConfig {
    BillingConfig { provider: "stripe".into(), api_key: "sk_test_contract".into(),
        webhook_secret: "whsec_contract_abcdefghijklmnopqrstuvwxyz0123456789".into(),
        redirect_url: "https://app.example/return".into(), store_id: None,
        allowed_plan_ids: ["price_pro".into(), "price_basic".into()].into_iter().collect() }
}
async fn handler(Extension(identity): Extension<BillingIdentity>, Extension(provider): Extension<Provider>) -> Response {
    match report::report_with(&config(), &identity, &provider).await {
        Ok(response) => response,
        Err(status) => status.into_response(),
    }
}
async fn request(app: &rullst::web::axum::Router, session: Option<&str>, query: &str) -> Response {
    let mut request = Request::builder().uri(format!("/reports/billing{query}"))
        .header("x-tenant-id", "acct_other.test").header("x-user-id", "1");
    if let Some(session) = session { request = request.header("cookie", format!("rullst_session={session}")); }
    app.clone().oneshot(request.body(Body::empty()).unwrap()).await.unwrap()
}
async fn seed(owner: i64, namespace: &str, known_subscription: bool) {
    let state = State { scope: namespace.into(), owner, reference: format!("owner_{owner}_{namespace}").replace(':', "_"),
        revision: token(), customer_key: token(), customer_started: now().unwrap(), customer: Some(format!("cus_{owner}")),
        attempt: Some(Attempt { key: token(), created: now().unwrap(), price: "price_pro".into(),
            success: "https://app.example/return".into(), cancel: "https://app.example/return".into(), session: Some(format!("cs_{owner}")),
            subscription: known_subscription.then(|| format!("sub_{owner}")), closed: false }),
        previous_attempts: vec![], entitlement: None };
    store::insert(&state).await.unwrap();
}
#[tokio::test]
async fn generated_entitlement_http_and_revision_contract() {
    rullst::orm::Orm::init("sqlite::memory:").await.unwrap();
    for migration in crate::migrations::get_migrations() { migration.up().await.unwrap(); }
    let pool = rullst::orm::Orm::pool().unwrap();
    for id in 1..=4 {
        rullst::db::sqlx::query("INSERT INTO users (id,name,email,created_at,updated_at) VALUES (?,?,?,?,?)")
            .bind(id).bind(format!("Member {id}")).bind(format!("member{id}@example.test"))
            .bind("2026-09-20").bind("2026-09-20").execute(pool).await.unwrap();
    }
    seed(1, "acct_contract:test", true).await;
    seed(2, "acct_contract:test", false).await;
    seed(3, "acct_other:test", true).await;
    // An editable CMS projection cannot grant access, even with a future end.
    rullst::db::sqlx::query("INSERT INTO subscriptions (user_id,customer_id,subscription_id,plan_id,status,ends_at,created_at,updated_at) VALUES (?,?,?,?,?,?,?,?)")
        .bind(2).bind("cus_forged").bind("sub_forged").bind("price_pro").bind("active").bind(1900000000_i64)
        .bind("2026-09-20").bind("2026-09-20").execute(pool).await.unwrap();
    let key = rullst::auth::get_app_key().unwrap();
    let sessions = (1..=4).map(|id| rullst::auth::encrypt_session(id, &key).unwrap()).collect::<Vec<_>>();
    let production_router = crate::router().unwrap();
    assert_eq!(request(production_router.as_axum(), None, "").await.status(), StatusCode::SEE_OTHER);
    assert_eq!(request(production_router.as_axum(), Some("forged"), "").await.status(), StatusCode::SEE_OTHER);
    // The actual handler uses configured offline credentials: never entitled.
    let offline = request(production_router.as_axum(), Some(&sessions[0]), "").await;
    assert_eq!(offline.status(), UNAVAILABLE);
    assert_eq!(offline.headers()["cache-control"], "no-store");

    let provider = Provider::default();
    let app = rullst::web::axum::Router::new()
        .route("/reports/billing", rullst::routing::get(handler))
        .layer(rullst::server::from_fn(crate::middlewares::auth_middleware::auth_middleware))
        .layer(Extension(provider.clone()));
    let app = rullst::security::apply_security_baseline(app, rullst::config::SecurityConfig::default(),
        rullst::config::Environment::Production).unwrap();
    assert_eq!(request(&app, None, "").await.status(), StatusCode::SEE_OTHER);
    let policy_invalid = std::env::var("BILLING_REPORT_PLAN_IDS").as_deref() != Ok("price_pro");
    let expect_status = if policy_invalid { UNAVAILABLE } else { StatusCode::OK };
    let allowed = request(&app, Some(&sessions[0]), "").await;
    assert_eq!(allowed.status(), expect_status);
    if policy_invalid { return; }
    assert_eq!(allowed.headers()["cache-control"], "no-store");
    assert_eq!(allowed.headers()["referrer-policy"], "no-referrer");
    let bytes = rullst::web::axum::body::to_bytes(allowed.into_body(), 4096).await.unwrap();
    let content: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(content["feature"], "reports.billing"); assert_eq!(content["plan"], "price_pro");
    assert_eq!(content["status"], "active");
    assert!(!String::from_utf8(bytes.to_vec()).unwrap().contains("cus_"));
    for session in &sessions[1..] {
        assert_eq!(request(&app, Some(session), "?owner_id=1&plan=price_pro&tenant=acct_contract.test").await.status(), StatusCode::FORBIDDEN);
    }
    for status in ["trialing", "past_due", "canceled", "unpaid", "incomplete_expired", "unknown"] {
        *provider.0.status.lock().unwrap() = Some(status.into());
        assert_eq!(request(&app, Some(&sessions[0]), "").await.status(), StatusCode::FORBIDDEN);
    }
    *provider.0.status.lock().unwrap() = None;
    *provider.0.plan.lock().unwrap() = Some("price_basic".into());
    assert_eq!(request(&app, Some(&sessions[0]), "").await.status(), StatusCode::FORBIDDEN);
    *provider.0.plan.lock().unwrap() = None;
    for end in [-1, 1, now().unwrap()] {
        provider.0.ends.store(end, Ordering::SeqCst);
        assert_eq!(request(&app, Some(&sessions[0]), "").await.status(), StatusCode::FORBIDDEN);
    }
    provider.0.ends.store(0, Ordering::SeqCst);
    provider.0.failed.store(true, Ordering::SeqCst);
    assert_eq!(request(&app, Some(&sessions[0]), "").await.status(), UNAVAILABLE);
    provider.0.failed.store(false, Ordering::SeqCst);

    // An earlier active response cannot overwrite a newer revocation.
    provider.0.paused.store(true, Ordering::SeqCst);
    let first_app = app.clone(); let first_session = sessions[0].clone();
    let earlier = tokio::spawn(async move { request(&first_app, Some(&first_session), "").await });
    tokio::time::timeout(std::time::Duration::from_secs(10), provider.0.entered.notified()).await.unwrap();
    *provider.0.status.lock().unwrap() = Some("canceled".into());
    assert_eq!(request(&app, Some(&sessions[0]), "").await.status(), StatusCode::FORBIDDEN);
    provider.0.resume.notify_one();
    assert_eq!(earlier.await.unwrap().status(), UNAVAILABLE);
    assert_eq!(store::owner("acct_contract:test", 1).await.unwrap().unwrap().entitlement.unwrap().status, "canceled");

    // The same production deadline cancels a stuck provider read without granting.
    *provider.0.status.lock().unwrap() = None;
    provider.0.paused.store(true, Ordering::SeqCst);
    let stuck_app = app.clone(); let stuck_session = sessions[0].clone();
    let stuck = tokio::spawn(async move { request(&stuck_app, Some(&stuck_session), "").await });
    tokio::time::timeout(std::time::Duration::from_secs(10), provider.0.entered.notified()).await.unwrap();
    assert_eq!(tokio::time::timeout(std::time::Duration::from_secs(25), stuck).await.unwrap().unwrap().status(), UNAVAILABLE);
    assert_eq!(store::owner("acct_contract:test", 1).await.unwrap().unwrap().entitlement.unwrap().status, "canceled");

    // Corrupted payload scope is rejected even when its indexed lookup matches.
    let state = store::owner("acct_contract:test", 1).await.unwrap().unwrap();
    let mut forged = state.clone(); forged.scope = "acct_other:test".into();
    rullst::db::sqlx::query("UPDATE billing_state SET payload = ? WHERE scope = ? AND owner_id = ?")
        .bind(forged.encode().unwrap()).bind("acct_contract:test").bind("1").execute(pool).await.unwrap();
    assert_eq!(request(&app, Some(&sessions[0]), "").await.status(), UNAVAILABLE);
    rullst::db::sqlx::query("DROP TABLE billing_state").execute(pool).await.unwrap();
    assert_eq!(request(&app, Some(&sessions[0]), "").await.status(), UNAVAILABLE);
}
