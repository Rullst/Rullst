use rullst_messaging::{
    Clock, DeadLetterQuery, FailureCode, MessageAdmin, MessageBroker, MessagingError,
    PublishRequest, PurgeRequest, ReceiveRequest, Result, StartPosition, SubscriptionRequest,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ManualClock(Arc<AtomicI64>);

impl ManualClock {
    pub fn new(now_millis: i64) -> Self {
        Self(Arc::new(AtomicI64::new(now_millis)))
    }

    pub fn advance(&self, millis: i64) {
        self.0.fetch_add(millis, Ordering::SeqCst);
    }
}

impl Clock for ManualClock {
    fn now_millis(&self) -> Result<i64> {
        Ok(self.0.load(Ordering::SeqCst))
    }
}

pub async fn run_core_contract<B>(broker: &B)
where
    B: MessageBroker + MessageAdmin,
{
    let alpha = SubscriptionRequest::try_new("orders", "alpha", StartPosition::Earliest)
        .expect("valid alpha subscription");
    let beta = SubscriptionRequest::try_new("orders", "beta", StartPosition::Earliest)
        .expect("valid beta subscription");
    assert!(
        broker
            .subscribe(alpha.clone())
            .await
            .expect("alpha")
            .was_created()
    );
    assert!(broker.subscribe(beta).await.expect("beta").was_created());
    assert!(
        !broker
            .subscribe(alpha)
            .await
            .expect("alpha replay")
            .was_created()
    );
    assert_eq!(
        broker
            .dead_letters(DeadLetterQuery::try_new("orders", "missing", 10).expect("missing query"))
            .await,
        Err(MessagingError::SubscriptionNotFound)
    );

    let request = PublishRequest::try_new(
        "orders",
        "order.created",
        "order/42/v1",
        br#"{"secret":"payload-marker"}"#.to_vec(),
    )
    .expect("valid publication")
    .with_content_type("application/json")
    .expect("valid content type")
    .with_header("trace-id", "private-header-marker")
    .expect("valid header");
    let first = broker
        .publish(request.clone())
        .await
        .expect("first publish");
    let replay = broker.publish(request.clone()).await.expect("exact replay");
    assert!(!first.is_duplicate());
    assert!(replay.is_duplicate());
    assert_eq!(first.id(), replay.id());

    let conflict = PublishRequest::try_new(
        "orders",
        "order.created",
        "order/42/v1",
        b"different".to_vec(),
    )
    .expect("valid conflict shape");
    assert_eq!(
        broker.publish(conflict).await,
        Err(MessagingError::IdempotencyConflict)
    );

    let receive_alpha =
        ReceiveRequest::try_new("orders", "alpha", "worker-a", 10, Duration::from_secs(30))
            .expect("valid receive");
    let receive_beta =
        ReceiveRequest::try_new("orders", "beta", "worker-b", 10, Duration::from_secs(30))
            .expect("valid receive");
    let alpha_messages = broker.receive(receive_alpha).await.expect("alpha receive");
    let beta_messages = broker.receive(receive_beta).await.expect("beta receive");
    assert_eq!(alpha_messages.len(), 1);
    assert_eq!(beta_messages.len(), 1);
    assert_eq!(alpha_messages[0].envelope().id(), first.id());
    assert_eq!(beta_messages[0].envelope().id(), first.id());
    assert_eq!(alpha_messages[0].envelope().schema(), "rullst.messaging.v1");

    let debug = format!("{:?}", alpha_messages[0]);
    assert!(!debug.contains("payload-marker"));
    assert!(!debug.contains("private-header-marker"));
    assert!(!debug.contains("order/42/v1"));

    broker
        .ack(alpha_messages[0].ack_token())
        .await
        .expect("alpha ack");
    assert_eq!(
        broker.ack(alpha_messages[0].ack_token()).await,
        Err(MessagingError::LeaseNotFound)
    );
    broker
        .dead_letter(
            beta_messages[0].ack_token(),
            FailureCode::try_new("handler.rejected").expect("failure code"),
        )
        .await
        .expect("beta dead letter");

    let dead = broker
        .dead_letters(DeadLetterQuery::try_new("orders", "beta", 10).expect("query"))
        .await
        .expect("dead letters");
    assert_eq!(dead.len(), 1);
    assert_eq!(dead[0].failure_code().as_str(), "handler.rejected");
    let purge = broker
        .purge_terminal(PurgeRequest::try_new("orders", 10).expect("purge"))
        .await
        .expect("purge terminal");
    assert_eq!(purge.removed(), 1);
}
