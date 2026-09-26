use super::*;
use gateway::{Customer, Transaction, Subscription};
use std::sync::{Arc, Mutex, atomic::{AtomicBool, AtomicUsize, Ordering}};

#[derive(Clone, Default)]
pub(super) struct Fake(pub Arc<Remote>);
#[derive(Default)]
pub(super) struct Remote {
    customers: Mutex<std::collections::HashMap<String, ([u8;32], String)>>,
    transactions: Mutex<std::collections::HashMap<String, [u8;32]>>,
    pub customer_creates: AtomicUsize, pub creates: AtomicUsize,
    pub lose_customer: AtomicBool, pub lose_checkout: AtomicBool,
    pub complete: AtomicBool, pub canceled: AtomicBool, pub wrong_mode: AtomicBool,
    pub pause: AtomicBool, pub entered: tokio::sync::Notify, pub resume: tokio::sync::Notify,
}
impl Fake {
    pub fn customer_id(&self) -> String { format!("ctm_{:026}", self.0.customer_creates.load(Ordering::SeqCst)) }
    pub fn transaction_id(&self) -> String { format!("txn_{:026}", self.0.creates.load(Ordering::SeqCst)) }
    fn snapshot(&self, request: &PaddleCheckoutRequest, id: &str) -> Transaction {
        let complete = self.0.complete.load(Ordering::SeqCst);
        Transaction { id: id.into(), digest: request.request_digest(), sandbox: !self.0.wrong_mode.load(Ordering::SeqCst),
            status: if complete {"completed"} else {"draft"}.into(),
            url: (!complete).then(|| format!("{}?_ptxn={id}",request.payment_link())),
            subscription: complete.then(|| id.replacen("txn_", "sub_", 1)) }
    }
}
impl Gateway for Fake {
    async fn create_customer(&self, request: &PaddleCustomerRequest) -> Result<Customer> {
        let serial = self.0.customer_creates.fetch_add(1,Ordering::SeqCst)+1;
        let id = format!("ctm_{serial:026}");
        self.0.customers.lock().unwrap().insert(id.clone(), (request.request_digest(),request.owner_reference().into()));
        if self.0.lose_customer.swap(false,Ordering::SeqCst) { return Err(UNAVAILABLE); }
        Ok(Customer {id,digest:request.request_digest(),sandbox:true})
    }
    async fn customer(&self, request: &PaddleCustomerRequest, id: &str) -> Result<Customer> {
        let records = self.0.customers.lock().unwrap();
        let (digest, owner) = records.get(id).ok_or(CONFLICT)?;
        if *digest != request.request_digest() || owner != request.owner_reference() { return Err(CONFLICT); }
        Ok(Customer { id:id.into(), digest:*digest, sandbox:true })
    }
    async fn create_checkout(&self, request: &PaddleCheckoutRequest) -> Result<Transaction> {
        let serial = self.0.creates.fetch_add(1,Ordering::SeqCst)+1;
        let id = format!("txn_{serial:026}");
        self.0.transactions.lock().unwrap().insert(id.clone(),request.request_digest());
        if self.0.lose_checkout.swap(false,Ordering::SeqCst) { return Err(UNAVAILABLE); }
        Ok(self.snapshot(request,&id))
    }
    async fn transaction(&self, request: &PaddleCheckoutRequest, id: &str) -> Result<Transaction> {
        if self.0.transactions.lock().unwrap().get(id) != Some(&request.request_digest()) { return Err(CONFLICT); }
        Ok(self.snapshot(request,id))
    }
    async fn subscription(&self, request: &PaddleCheckoutRequest, id: &str) -> Result<Subscription> {
        let canceled = self.0.canceled.load(Ordering::SeqCst);
        if self.0.pause.swap(false,Ordering::SeqCst) {
            self.0.entered.notify_one();
            tokio::time::timeout(std::time::Duration::from_secs(10),self.0.resume.notified()).await.unwrap();
        }
        let ends = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64 + 86400;
        Ok(Subscription { status:if canceled {"canceled"} else {"active"}.into(),sandbox:true,
            event:rullst::capital::WebhookEvent {subscription_id:id.into(), customer_id:request.customer_id().into(),
                customer_email:String::new(), plan_id:request.price_id().into(), ends_at:Some(ends),
                status:if canceled {rullst::capital::SubscriptionStatus::Canceled} else {rullst::capital::SubscriptionStatus::Active} } })
    }
    async fn notice(&self, request: &PaddleCheckoutRequest, transaction: &str, body: &[u8], _: &std::collections::HashMap<String,String>) -> Result<(Receipt,String)> {
        // This fake covers application state transitions only. Capital's signed
        // wire fixtures cover actual HMAC/metadata parsing independently.
        self.transaction(request,transaction).await?;
        let value:serde_json::Value = serde_json::from_slice(body).unwrap();
        Ok((Receipt {id:value["id"].as_str().unwrap().into(),digest:value["digest"].as_str().unwrap().into()},
            transaction.replacen("txn_","sub_",1)))
    }
}
