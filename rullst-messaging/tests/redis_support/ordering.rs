use super::redis_support::*;
use rullst_messaging::*;

fn numbered(index: usize) -> PublishRequest {
    PublishRequest::try_new(
        "orders",
        "order.created",
        format!("order-{index}"),
        index.to_string().into_bytes(),
    )
    .unwrap()
}

fn payloads(deliveries: &[Delivery]) -> Vec<String> {
    deliveries
        .iter()
        .map(|delivery| String::from_utf8(delivery.envelope().payload().to_vec()).unwrap())
        .collect()
}

#[tokio::test]
#[ignore = "requires the owned disposable Redis fixture"]
async fn same_time_entries_are_delivered_in_sequence_order() {
    let broker = RedisBroker::provision(configuration(&unique("ordering")))
        .await
        .unwrap();
    // Sequences 2..=12 cross a digit boundary ("10" sorts before "9" as text),
    // and an earliest subscription gives every retained message one score.
    for index in 0..11 {
        broker.publish(numbered(index)).await.unwrap();
    }
    let expected: Vec<String> = (0..11).map(|index: usize| index.to_string()).collect();

    broker
        .subscribe(subscription("orders", "single", StartPosition::Earliest))
        .await
        .unwrap();
    let mut single = Vec::new();
    for _ in 0..11 {
        let deliveries = broker
            .receive(receive("orders", "single", "worker", 1))
            .await
            .unwrap();
        assert_eq!(deliveries.len(), 1);
        broker.ack(deliveries[0].ack_token()).await.unwrap();
        single.extend(payloads(&deliveries));
    }
    assert_eq!(single, expected, "one consumer sees publication order");

    broker
        .subscribe(subscription("orders", "batch", StartPosition::Earliest))
        .await
        .unwrap();
    let batch = broker
        .receive(receive("orders", "batch", "worker", 11))
        .await
        .unwrap();
    assert_eq!(payloads(&batch), expected, "one batch is in sequence order");
    for delivery in &batch {
        broker.ack(delivery.ack_token()).await.unwrap();
    }
}
