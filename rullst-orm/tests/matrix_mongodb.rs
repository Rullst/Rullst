#![cfg(feature = "mongodb")]

mod support;

use rullst_orm::polyglot::{
    CollectionName, DocumentId, DocumentPage, DocumentRepository, MongoDbStore, PolyglotError,
};
use serde::{Deserialize, Serialize};
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::mongo::Mongo;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Event {
    sequence: u64,
    label: String,
}

#[tokio::test]
async fn test_matrix_mongodb_document_contract() {
    let container = match Mongo::default().start().await {
        Ok(container) => container,
        Err(error) => {
            support::handle_container_start_error("MongoDB", error);
            return;
        }
    };
    let host = container
        .get_host()
        .await
        .expect("MongoDB container host should be available");
    let port = container
        .get_host_port_ipv4(27017)
        .await
        .expect("MongoDB container port should be available");
    let store =
        MongoDbStore::<Event>::connect_or_mock(format!("mongodb://{host}:{port}"), "rullst_matrix")
            .await
            .expect("MongoDB driver should initialize");
    assert!(!store.is_mock());

    let collection = CollectionName::new("events").expect("valid collection");
    let first = DocumentId::new("event-01").expect("valid id");
    let second = DocumentId::new("event-02").expect("valid id");
    let event = |sequence| Event {
        sequence,
        label: format!("event-{sequence}"),
    };

    store
        .create(&collection, &second, &event(2))
        .await
        .expect("create second event");
    store
        .create(&collection, &first, &event(1))
        .await
        .expect("create first event");
    assert!(matches!(
        store.create(&collection, &first, &event(10)).await,
        Err(PolyglotError::Conflict)
    ));
    assert_eq!(
        store.find(&collection, &first).await.expect("find event"),
        Some(event(1))
    );

    store
        .replace(&collection, &first, &event(11))
        .await
        .expect("replace event");
    assert_eq!(
        store
            .list(&collection, DocumentPage::new(0, 1).expect("bounded page"),)
            .await
            .expect("list events"),
        vec![event(11)]
    );
    assert!(
        store
            .delete(&collection, &first)
            .await
            .expect("delete event")
    );
    assert!(
        !store
            .delete(&collection, &first)
            .await
            .expect("repeat delete")
    );
}
