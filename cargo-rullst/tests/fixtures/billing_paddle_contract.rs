#![allow(clippy::unwrap_used,clippy::expect_used)]
use super::*;
#[path = "billing_paddle_fake.rs"]
mod fake;
use fake::Fake;
use std::sync::atomic::Ordering;
const PRICE: &str = "pri_00000000000000000000000001";
const OTHER: &str = "pri_00000000000000000000000002";
const NAMESPACE: &str = "paddle:acct_contract:sandbox";
fn config() -> BillingConfig {
    BillingConfig {provider:"paddle".into(),api_key:"fixture_paddle_api_key_not_functional".into(),
        webhook_secret:"fixture_paddle_webhook_abcdefghijklmnopqrstuvwxyz0123456789".into(),
        redirect_url:"https://app.example/return".into(),store_id:None,
        allowed_plan_ids:[PRICE.into(),OTHER.into()].into_iter().collect()}
}
fn identity(id: __OWNER_ID_TYPE__) -> BillingIdentity { BillingIdentity {owner_id:id,email:"original@example.test".into()} }
async fn load(id:i64) -> State {store::owner(NAMESPACE,id).await.unwrap().unwrap()}
async fn sql(statement: &str) { __EXECUTE_SQL__ }
async fn process(gateway:&Fake,state:&State,id:&str,digest:&str) -> Result<Response> {
    let attempt=state.attempt.as_ref().unwrap();
    let notice=events::Notice {owner:state.reference.clone(),customer:state.customer.clone().unwrap(),
        attempt:attempt.key.clone(), transaction:attempt.transaction.clone()};
    events::process(&config(),gateway,notice,&serde_json::to_vec(&serde_json::json!({"id":id,"digest":digest})).unwrap(),
        &std::collections::HashMap::new()).await
}
#[tokio::test]
async fn durable_paddle_billing_contract() {
    __INITIALIZE_DB__
    if std::env::var_os("BILLING_RESTART_CHECK").is_some() {
        assert_eq!(load(200).await.entitlement.unwrap().status,"canceled");
        assert!(store::receipt(NAMESPACE,"evt_newer_paddle").await.unwrap().is_some());
        assert!(store::receipt(NAMESPACE,"evt_older_paddle").await.unwrap().is_none());
        // A process restart cannot re-enable a claimed creation.
        let remote=Fake::default();
        assert!(checkout_with(&config(),&identity(203),PRICE,&remote).await.is_err());
        assert_eq!(remote.0.customer_creates.load(Ordering::SeqCst),0);
        assert!(checkout_with(&config(),&identity(204),PRICE,&remote).await.is_err());
        assert_eq!(remote.0.creates.load(Ordering::SeqCst),0);
        return;
    }
    let config=config();let remote=Fake::default();let owner=identity(200);
    remote.0.lose_customer.store(true,Ordering::SeqCst);
    assert!(checkout_with(&config,&owner,PRICE,&remote).await.is_err());
    assert!(load(200).await.customer_dispatched);
    let mut changed=identity(200);changed.email="changed@example.test".into();
    for _ in 0..3 {assert!(checkout_with(&config,&changed,PRICE,&remote).await.is_err());}
    assert_eq!(remote.0.customer_creates.load(Ordering::SeqCst),1);
    recover_with(&config,&changed,&remote.customer_id(),false,&remote).await.unwrap();
    assert_eq!(load(200).await.email,owner.email);
    remote.0.lose_checkout.store(true,Ordering::SeqCst);
    assert!(checkout_with(&config,&owner,PRICE,&remote).await.is_err());
    for _ in 0..3 {assert!(checkout_with(&config,&owner,PRICE,&remote).await.is_err());}
    assert_eq!(remote.0.creates.load(Ordering::SeqCst),1);
    recover_with(&config,&owner,&remote.transaction_id(),true,&remote).await.unwrap();
    let url=checkout_with(&config,&owner,PRICE,&remote).await.unwrap();
    assert!(url.starts_with("https://app.example/pay?_ptxn=txn_"));
    assert_eq!(remote.0.creates.load(Ordering::SeqCst),1);
    assert_eq!(checkout_with(&config,&owner,OTHER,&remote).await,Err(CONFLICT));
    assert!(load(200).await.entitlement.is_none());
    assert_eq!(report::report_with(&config,&owner,&remote).await.unwrap_err(),StatusCode::FORBIDDEN);

    // A different owner with the same email has a separate binding; recovered
    // provider IDs and generated references cannot be reassigned between them.
    checkout_with(&config,&identity(201),PRICE,&remote).await.unwrap();
    let foreign=load(201).await;
    assert_ne!(foreign.customer,load(200).await.customer);
    assert!(recover_with(&config,&owner,foreign.customer.as_deref().unwrap(),false,&remote).await.is_err());
    assert!(recover_with(&config,&owner,foreign.attempt.as_ref().unwrap().transaction.as_deref().unwrap(),true,&remote).await.is_err());
    remote.0.wrong_mode.store(true,Ordering::SeqCst);
    assert!(checkout_with(&config,&owner,PRICE,&remote).await.is_err());
    remote.0.wrong_mode.store(false,Ordering::SeqCst);

    // An early subscription event recovers a known signed transaction after a
    // creation response was lost. Current provider state owns the entitlement.
    let old=load(200).await;let mut lost=old.clone();lost.attempt.as_mut().unwrap().transaction=None;
    commit(&old,lost,None).await.unwrap();
    remote.0.complete.store(true,Ordering::SeqCst);
    process(&remote,&old,"evt_early_paddle","first").await.unwrap();
    let active=load(200).await;
    assert_eq!(active.entitlement.as_ref().unwrap().status,"active");
    assert!(active.entitlement.as_ref().unwrap().subscription.starts_with("paddle:acct_contract:sandbox/sub_"));
    assert_eq!(report::report_with(&config,&owner,&remote).await.unwrap().status(),StatusCode::OK);
    process(&remote,&active,"evt_early_paddle","first").await.unwrap();
    assert_eq!(process(&remote,&active,"evt_early_paddle","different").await.unwrap_err(),CONFLICT);
    let mut forged=active.clone();forged.customer=foreign.customer.clone();
    assert_eq!(process(&remote,&forged,"evt_forged_paddle","first").await.unwrap_err(),CONFLICT);

    // A failed event insert rolls back projection, state payload and receipt.
    remote.0.canceled.store(true,Ordering::SeqCst);
    sql("CREATE TRIGGER reject_paddle_event BEFORE INSERT ON billing_events BEGIN SELECT RAISE(ABORT,'injected failure'); END").await;
    assert!(process(&remote,&active,"evt_rollback_paddle","first").await.is_err());
    assert_eq!(load(200).await.entitlement.unwrap().status,"active");
    assert!(store::receipt(NAMESPACE,"evt_rollback_paddle").await.unwrap().is_none());
    sql("DROP TRIGGER reject_paddle_event").await;
    process(&remote,&active,"evt_rollback_paddle","first").await.unwrap();
    assert_eq!(report::report_with(&config,&owner,&remote).await.unwrap_err(),StatusCode::FORBIDDEN);

    // Old deliveries cannot overwrite a replacement subscription.
    let retired=load(200).await;
    remote.0.complete.store(false,Ordering::SeqCst);remote.0.canceled.store(false,Ordering::SeqCst);
    checkout_with(&config,&owner,OTHER,&remote).await.unwrap();
    let replacement=load(200).await;
    remote.0.complete.store(true,Ordering::SeqCst);
    process(&remote,&replacement,"evt_replacement_paddle","first").await.unwrap();
    remote.0.canceled.store(true,Ordering::SeqCst);
    process(&remote,&retired,"evt_retired_paddle","first").await.unwrap();
    assert_eq!(load(200).await.entitlement.unwrap().status,"active");

    // Fence an earlier active read while a later cancellation commits.
    remote.0.canceled.store(false,Ordering::SeqCst);remote.0.pause.store(true,Ordering::SeqCst);
    let earlier=load(200).await;let slow=remote.clone();
    let task=tokio::spawn(async move {process(&slow,&earlier,"evt_older_paddle","first").await});
    tokio::time::timeout(std::time::Duration::from_secs(10),remote.0.entered.notified()).await.unwrap();
    remote.0.canceled.store(true,Ordering::SeqCst);
    process(&remote,&load(200).await,"evt_newer_paddle","first").await.unwrap();
    remote.0.resume.notify_one();
    assert!(tokio::time::timeout(std::time::Duration::from_secs(10),task).await.unwrap().unwrap().is_err());
    assert_eq!(load(200).await.entitlement.unwrap().status,"canceled");
    assert!(store::receipt(NAMESPACE,"evt_older_paddle").await.unwrap().is_none());

    remote.0.complete.store(false,Ordering::SeqCst);remote.0.canceled.store(false,Ordering::SeqCst);
    let count=remote.0.customer_creates.load(Ordering::SeqCst);let creates=remote.0.creates.load(Ordering::SeqCst);
    let concurrent=identity(202);
    let (a,b)=tokio::join!(checkout_with(&config,&concurrent,PRICE,&remote),checkout_with(&config,&concurrent,PRICE,&remote));
    assert!(a.is_ok() || b.is_ok());
    assert_eq!(remote.0.customer_creates.load(Ordering::SeqCst),count+1);
    assert_eq!(remote.0.creates.load(Ordering::SeqCst),creates+1);
    remote.0.lose_customer.store(true,Ordering::SeqCst);
    assert!(checkout_with(&config,&identity(203),PRICE,&remote).await.is_err());
    remote.0.lose_checkout.store(true,Ordering::SeqCst);
    assert!(checkout_with(&config,&identity(204),PRICE,&remote).await.is_err());

    // Wrong signatures and duplicate headers fail before provider I/O.
    let request=rullst::server::Request::builder().method("POST").header("paddle-signature","invalid")
        .body(rullst::web::axum::body::Body::from("{}")).unwrap();
    assert_eq!(events::webhook(&config,request).await.unwrap_err(),StatusCode::BAD_REQUEST);
    let request=rullst::server::Request::builder().method("POST").header("paddle-signature","one").header("paddle-signature","two")
        .body(rullst::web::axum::body::Body::from("{}")).unwrap();
    assert_eq!(events::webhook(&config,request).await.unwrap_err(),StatusCode::BAD_REQUEST);
}

#[test]
fn paddle_activation_gate_requires_exact_acknowledgement() {
    let allowed=std::env::var("BILLING_PADDLE_ENVIRONMENT").as_deref()==Ok("sandbox")
        || std::env::var("BILLING_LIVE_ACKNOWLEDGEMENT").as_deref()==Ok("I_UNDERSTAND_REAL_CHARGES");
    assert_eq!(scope(&config()).is_ok(),allowed);
}
