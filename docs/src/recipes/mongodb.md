# MongoDB with the official driver

This recipe uses the official [`mongodb`](https://crates.io/crates/mongodb)
crate in a Rullst application: one client created at start-up, a small
application trait that handlers depend on, an in-memory fake for tests and
local work, and integration tests against a real server. It replaces the
`MongoDbStore` adapter that 13.0.0-alpha.5 removed; see
[Migrating from `MongoDbStore`](#migrating-from-mongodbstore). The code is
yours (see [what a recipe is](index.md#what-a-recipe-is)).

The blocks that use Rullst types (configuration, the trait, the fake, the
handler and its test) are compiled and run as doctests in Rullst's CI. The
blocks marked as driver code are not compiled there, because the Rullst
workspace deliberately does not depend on `mongodb`; they are written against
the `mongodb` 3.x API (checked against 3.10.0).

## When to use it

Use MongoDB when the application already has a MongoDB deployment, or when the
data is a set of documents whose shape another system owns. For relational
data keep the SQLx ORM (SQLite, PostgreSQL, MySQL or MariaDB). If you want
Rullst's bounded document contract with an offline store and encrypted
recovery, the SurrealDB adapter of
[Polyglot Persistence](../polyglot-persistence.md) still provides it.

## Dependency

```toml
[dependencies]
mongodb = { version = "3", default-features = false, features = ["compat-3-0-0", "dns-resolver", "rustls-tls-aws-lc"] }
```

The driver's default `rustls-tls` feature makes ring its rustls provider.
Since 13.0.0-alpha.4 every TLS connection Rullst makes uses aws-lc-rs
([cryptography providers](../security-architecture.md#cryptography-providers)),
so `rustls-tls-aws-lc` keeps MongoDB on the same provider. `dns-resolver` is
needed for `mongodb+srv://` URLs, and `compat-3-0-0` selects the driver's
3.0 API, with `mongodb::bson` at version 2.

## Configuration

Read the connection settings with `rullst::config::project_setting`, which
checks the process environment first and then the project's `.env`, the
precedence `Server` uses for `DATABASE_URL`. An empty or absent `MONGODB_URL`,
or one that starts with `mock_` (or `mock://`), selects the in-memory store, so
a fresh checkout runs without a server:

```rust,no_run
use rullst::config::project_setting;

/// Where the event store lives.
pub enum EventStoreSettings {
    /// No server configured: use the in-memory store.
    Offline,
    /// A MongoDB deployment and the database the application uses in it.
    Mongo { url: String, database: String },
}

/// Reads `MONGODB_URL` and `MONGODB_DATABASE`.
pub fn event_store_settings()
-> Result<EventStoreSettings, Box<dyn std::error::Error + Send + Sync>> {
    let url = project_setting("MONGODB_URL")?.unwrap_or_default();
    if url.is_empty() || url.starts_with("mock_") || url.starts_with("mock://") {
        return Ok(EventStoreSettings::Offline);
    }
    let database = project_setting("MONGODB_DATABASE")?
        .filter(|name| !name.is_empty())
        .ok_or("MONGODB_DATABASE must name the application's database")?;
    Ok(EventStoreSettings::Mongo { url, database })
}
```

```text
# .env (development; never commit production credentials)
MONGODB_URL=mock_local
MONGODB_DATABASE=app_development
```

In production, inject `MONGODB_URL` from the platform's secret store. It
usually carries a password: never log it, and do not echo it in error
messages (the code below replaces a parse error with a fixed text).

## The application boundary

Handlers depend on a small trait that names what the application needs, not
on the driver. Its methods return futures that are `Send`, so handlers that
await them can run on Rullst's multi-threaded runtime, and handlers are
generic over the store (static dispatch). The in-memory fake implements the
same trait:

```rust
use std::collections::BTreeMap;
use std::fmt;
use std::future::Future;
use std::sync::{Arc, Mutex, MutexGuard};

use rullst::Router;
use rullst::server::{Extension, IntoResponse, Json, Response, StatusCode, get};
use serde::{Deserialize, Serialize};

/// One audit event, as handlers and views see it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: String,
    pub owner_id: String,
    pub action: String,
}

/// Failures of the event store. Driver details stay in the server log.
#[derive(Debug)]
pub enum StoreError {
    /// An event with this identifier already exists.
    Conflict,
    /// The store could not complete the operation.
    Unavailable(String),
}

impl fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Conflict => formatter.write_str("the event already exists"),
            Self::Unavailable(detail) => write!(formatter, "event store unavailable: {detail}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl IntoResponse for StoreError {
    fn into_response(self) -> Response {
        match self {
            Self::Conflict => StatusCode::CONFLICT.into_response(),
            Self::Unavailable(detail) => {
                tracing::error!(%detail, "event store unavailable");
                StatusCode::SERVICE_UNAVAILABLE.into_response()
            }
        }
    }
}

/// What the application needs from its document store.
pub trait EventStore: Clone + Send + Sync + 'static {
    /// Stores a new event; an existing `id` is `StoreError::Conflict`.
    fn insert(&self, event: AuditEvent) -> impl Future<Output = Result<(), StoreError>> + Send;

    /// The owner's events in identifier order, at most `limit` of them.
    fn list_for_owner(
        &self,
        owner_id: &str,
        limit: u32,
    ) -> impl Future<Output = Result<Vec<AuditEvent>, StoreError>> + Send;
}

/// In-process store for tests and for an offline `MONGODB_URL`. Clones share
/// the same events, which live as long as the process.
#[derive(Clone, Default)]
pub struct MemoryEventStore {
    events: Arc<Mutex<BTreeMap<String, AuditEvent>>>,
}

impl MemoryEventStore {
    fn events(&self) -> Result<MutexGuard<'_, BTreeMap<String, AuditEvent>>, StoreError> {
        self.events
            .lock()
            .map_err(|_| StoreError::Unavailable("the in-memory store lock is poisoned".to_owned()))
    }
}

impl EventStore for MemoryEventStore {
    async fn insert(&self, event: AuditEvent) -> Result<(), StoreError> {
        let mut events = self.events()?;
        if events.contains_key(&event.id) {
            return Err(StoreError::Conflict);
        }
        events.insert(event.id.clone(), event);
        Ok(())
    }

    async fn list_for_owner(
        &self,
        owner_id: &str,
        limit: u32,
    ) -> Result<Vec<AuditEvent>, StoreError> {
        let events = self.events()?;
        Ok(events
            .values()
            .filter(|event| event.owner_id == owner_id)
            .take(limit as usize)
            .cloned()
            .collect())
    }
}

/// The signed-in user, inserted by the application's authentication layer.
#[derive(Clone)]
pub struct CurrentUser {
    pub id: String,
}

/// Bounds every listing, whatever the client asks for.
const MAX_EVENTS: u32 = 100;

/// `GET /events`: the signed-in user's own events.
async fn my_events<S: EventStore>(
    Extension(store): Extension<S>,
    Extension(user): Extension<CurrentUser>,
) -> Result<Json<Vec<AuditEvent>>, StoreError> {
    let events = store.list_for_owner(&user.id, MAX_EVENTS).await?;
    Ok(Json(events))
}

/// The event routes with the store selected at start-up.
pub fn event_routes<S: EventStore>(store: S) -> Router {
    Router::new()
        .route("/events", get(my_events::<S>))
        .layer(Extension(store))
}

# #[tokio::main]
# async fn main() -> Result<(), Box<dyn std::error::Error>> {
// A unit test of the application: the fake, the real routes, no server.
let store = MemoryEventStore::default();
for (id, owner_id) in [("evt-1", "user-1"), ("evt-2", "user-2")] {
    let event = AuditEvent {
        id: id.into(),
        owner_id: owner_id.into(),
        action: "login".into(),
    };
    store.insert(event).await?;
}
let app = rullst::testing::TestApp::new(
    event_routes(store)
        .layer(Extension(CurrentUser { id: "user-1".into() }))
        .into_axum(),
);
let response = app.get("/events").send().await;
response.assert_status(200);
let events: Vec<AuditEvent> = response.json();
assert_eq!(events.len(), 1); // user-2's event is not visible
assert_eq!(events[0].id, "evt-1");
# Ok(())
# }
```

`event_routes` returns a `rullst::Router`; mount it with the application's
other routes, behind the authentication layer that inserts `CurrentUser`, and
behind the production security layers (CSRF for state-changing routes, secure
headers, rate limits) like any other route.

## The MongoDB implementation

Driver code (not compiled by Rullst's CI). The stored form maps the
application's `id` to MongoDB's `_id` explicitly, so the identifier is the one
the application chose, not a generated `ObjectId`:

```rust,ignore
use std::time::Duration;

use mongodb::bson::doc;
use mongodb::error::{ErrorKind, WriteFailure};
use mongodb::options::ClientOptions;
use mongodb::{Client, Collection, IndexModel};
use serde::{Deserialize, Serialize};

/// An event as MongoDB stores it.
#[derive(Serialize, Deserialize)]
struct EventDocument {
    #[serde(rename = "_id")]
    id: String,
    owner_id: String,
    action: String,
}

impl From<AuditEvent> for EventDocument {
    fn from(event: AuditEvent) -> Self {
        Self { id: event.id, owner_id: event.owner_id, action: event.action }
    }
}

impl From<EventDocument> for AuditEvent {
    fn from(document: EventDocument) -> Self {
        Self { id: document.id, owner_id: document.owner_id, action: document.action }
    }
}

/// MongoDB-backed store. Cloning shares the driver's connection pool.
#[derive(Clone)]
pub struct MongoEventStore {
    events: Collection<EventDocument>,
}

/// How long the server may spend on one listing.
const QUERY_BUDGET: Duration = Duration::from_secs(2);

impl MongoEventStore {
    /// Builds the client once, at start-up. The driver connects lazily, so
    /// this fails only on an invalid URL (or an unresolvable `mongodb+srv`
    /// name); `ensure_indexes` is the first call that reaches the server.
    pub async fn connect(
        url: impl Into<String>,
        database: impl Into<String>,
    ) -> Result<Self, StoreError> {
        let url = url.into();
        let database = database.into();
        // The URL may hold a password: report a fixed text, not the error.
        let mut options = ClientOptions::parse(url.as_str())
            .await
            .map_err(|_| StoreError::Unavailable("MONGODB_URL could not be parsed".to_owned()))?;
        options.app_name = Some("my-app".to_owned());
        options.server_selection_timeout = Some(Duration::from_secs(5));
        let client = Client::with_options(options).map_err(unavailable)?;
        Ok(Self { events: client.database(&database).collection("audit_events") })
    }

    /// Creates the index the owner listing uses. Run it at start-up or in a
    /// deployment step.
    pub async fn ensure_indexes(&self) -> Result<(), StoreError> {
        let index = IndexModel::builder().keys(doc! { "owner_id": 1, "_id": 1 }).build();
        self.events.create_index(index).await.map(|_| ()).map_err(unavailable)
    }
}

fn unavailable(error: mongodb::error::Error) -> StoreError {
    StoreError::Unavailable(error.to_string())
}

/// MongoDB's duplicate-key error (code 11000).
fn is_duplicate_key(error: &mongodb::error::Error) -> bool {
    matches!(
        error.kind.as_ref(),
        ErrorKind::Write(WriteFailure::WriteError(write)) if write.code == 11000
    )
}

impl EventStore for MongoEventStore {
    async fn insert(&self, event: AuditEvent) -> Result<(), StoreError> {
        match self.events.insert_one(EventDocument::from(event)).await {
            Ok(_) => Ok(()),
            Err(error) if is_duplicate_key(&error) => Err(StoreError::Conflict),
            Err(error) => Err(unavailable(error)),
        }
    }

    async fn list_for_owner(
        &self,
        owner_id: &str,
        limit: u32,
    ) -> Result<Vec<AuditEvent>, StoreError> {
        let mut cursor = self
            .events
            .find(doc! { "owner_id": owner_id })
            .sort(doc! { "_id": 1 })
            .limit(i64::from(limit))
            .max_time(QUERY_BUDGET)
            .await
            .map_err(unavailable)?;
        let mut events = Vec::new();
        while cursor.advance().await.map_err(unavailable)? {
            let document = cursor.deserialize_current().map_err(unavailable)?;
            events.push(AuditEvent::from(document));
        }
        Ok(events)
    }
}
```

`mongodb::Client` holds the connection pool and is cheap to clone: create it
once and share it (here inside `Collection`, which keeps a client handle).
Creating a client per request opens new connections every time.

## Start-up

Driver code. The application chooses its store once, from the settings above.
An enum keeps static dispatch while either store can serve:

```rust,ignore
/// The store the application runs with, chosen at start-up.
#[derive(Clone)]
pub enum AppEventStore {
    Mongo(MongoEventStore),
    Memory(MemoryEventStore),
}

impl EventStore for AppEventStore {
    async fn insert(&self, event: AuditEvent) -> Result<(), StoreError> {
        match self {
            Self::Mongo(store) => store.insert(event).await,
            Self::Memory(store) => store.insert(event).await,
        }
    }

    async fn list_for_owner(
        &self,
        owner_id: &str,
        limit: u32,
    ) -> Result<Vec<AuditEvent>, StoreError> {
        match self {
            Self::Mongo(store) => store.list_for_owner(owner_id, limit).await,
            Self::Memory(store) => store.list_for_owner(owner_id, limit).await,
        }
    }
}

#[rullst::runtime::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let store = match event_store_settings()? {
        EventStoreSettings::Offline => AppEventStore::Memory(MemoryEventStore::default()),
        EventStoreSettings::Mongo { url, database } => {
            let store = MongoEventStore::connect(url, database).await?;
            store.ensure_indexes().await?;
            AppEventStore::Mongo(store)
        }
    };
    let app = rullst::Router::new().merge_axum(event_routes(store).into_axum());
    rullst::Server::new(app).run(3000).await?;
    Ok(())
}
```

A production deployment should refuse to start with the in-memory store: make
`Offline` an error there (for example when `RULLST_ENV` is `production`), so a
missing secret is not silently replaced by an empty, process-local store.

## Ownership and query safety

- **Scope every query to its owner.** `list_for_owner` filters by
  `owner_id`; a lookup by identifier filters by both, for example
  `find_one(doc! { "_id": id, "owner_id": &user.id })`, so another user's
  identifier answers 404, not their document. This is the IDOR rule of
  section 3.4 of
  [`AGENTS.md`](https://github.com/Rullst/Rullst/blob/main/AGENTS.md).
- **Build filters from typed values.** Construct filters with `doc!` from
  validated fields. Never pass a request's JSON body as a filter or update
  document: a value such as `{"$ne": null}` is an operator, not data.
- **Bound every read.** Cap `limit` on the server side, as `MAX_EVENTS` does,
  and give long queries a server-side `max_time`.

## Tests against a real server

Unit tests use `MemoryEventStore`, as in the doctest above. Test
`MongoEventStore` itself against a real server, opt-in so `cargo test` stays
offline by default. With Testcontainers:

```toml
[dev-dependencies]
testcontainers = { version = "0.27.3", default-features = false, features = ["aws-lc-rs"] }
testcontainers-modules = { version = "0.15.0", default-features = false, features = ["aws-lc-rs", "mongo"] }
```

Driver code, in a `#[cfg(test)]` module next to `MongoEventStore`:

```rust,ignore
#[cfg(test)]
mod mongo_tests {
    use super::*;
    use testcontainers::{ImageExt, runners::AsyncRunner};
    use testcontainers_modules::mongo::Mongo;

    #[tokio::test]
    async fn mongo_store_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        if std::env::var_os("RUN_MONGO_TESTS").is_none() {
            eprintln!("skipped: set RUN_MONGO_TESTS=1 with Docker running");
            return Ok(());
        }
        // Pin the server version you deploy; the module's default is older.
        let container = Mongo::default().with_tag("8.0").start().await?;
        let host = container.get_host().await?;
        let port = container.get_host_port_ipv4(27017).await?;
        let store = MongoEventStore::connect(format!("mongodb://{host}:{port}"), "app_test").await?;
        store.ensure_indexes().await?;

        let event = AuditEvent { id: "evt-1".into(), owner_id: "user-1".into(), action: "login".into() };
        store.insert(event.clone()).await?;
        assert!(matches!(store.insert(event.clone()).await, Err(StoreError::Conflict)));
        assert_eq!(store.list_for_owner("user-1", 10).await?, vec![event]);
        assert!(store.list_for_owner("user-2", 10).await?.is_empty());
        Ok(())
    }
}
```

Run the same assertions against `MemoryEventStore` too, so the fake keeps the
real store's behaviour (the conflict, the ordering, the owner filter).

Without Testcontainers, start a disposable server bound to loopback, with a
memory limit, and have the test read its URL from an environment variable of
your own (`MONGODB_TEST_URL` here) instead of starting a container:

```text
docker run --rm -d --name app-mongo-test --memory 512m \
  -p 127.0.0.1:27017:27017 mongo:8.0
MONGODB_TEST_URL=mongodb://127.0.0.1:27017 RUN_MONGO_TESTS=1 cargo test
docker rm -f app-mongo-test
```

## TLS

- Use TLS for every connection that leaves the host: `mongodb+srv://` URLs
  (MongoDB Atlas and other managed services) enable it by default; for a
  `mongodb://` URL add `tls=true`.
- For a server certificate from a private CA, pass the CA bundle with
  `tlsCAFile=/path/to/ca.pem`; for client-certificate authentication,
  `tlsCertificateKeyFile=/path/to/client.pem`.
- Never set `tlsAllowInvalidCertificates` or `tlsInsecure` in production: they
  turn off server authentication.
- Give the database user only the roles the application needs on its own
  database (`readWrite`), not cluster-wide roles.

## Migrating from `MongoDbStore`

Up to 13.0.0-alpha.4, `rullst-orm`'s `mongodb` feature (`orm-mongodb` on
`rullst`) provided `MongoDbStore<T>`. 13.0.0-alpha.5 removed the feature and
the type. To move:

1. Remove `orm-mongodb` from the `rullst` features (or `mongodb` from the
   `rullst-orm` features) and add the `mongodb` dependency shown above.
   `cargo rullst upgrade` reports both the feature and the code that names
   `MongoDbStore`.
2. Replace `MongoDbStore::connect_or_mock(uri, database)` with the start-up
   code above: `MongoEventStore::connect` for a real URL, and
   `MemoryEventStore` where the adapter selected its offline store (an empty
   URL or one starting with `mock_` or `mock://`). The `MONGODB_URL` and
   `MONGODB_DATABASE` lines `cargo rullst new --mongodb` wrote to `.env` keep
   working with `event_store_settings`.
3. Replace the generic `create`, `find`, `replace`, `delete` and `list` calls
   with the operations the application actually needs, as `EventStore` does.

Existing data needs no migration: the adapter stored each document with `_id`
set to its `DocumentId` and the model's fields at the top level, which is the
`EventDocument` layout above when the field names match. The adapter's source,
which you may copy under its MIT licence, is
[`rullst-orm/src/polyglot/mongodb.rs` at v13.0.0-alpha.4](https://github.com/Rullst/Rullst/blob/v13.0.0-alpha.4/rullst-orm/src/polyglot/mongodb.rs).

The portable document contract stays in `rullst::orm::polyglot`:
`CollectionName`, `DocumentId`, `DocumentPage`, the `DocumentRepository` and
`DocumentInventory` traits, the deterministic `MockDocumentStore`, and the
encrypted `export_document_snapshot`/`restore_document_snapshot` recovery.
An application that relied on that contract (for example to move documents
between MongoDB and SurrealDB) can implement the traits for its own MongoDB
type. Driver code, abbreviated:

```rust,ignore
use mongodb::bson::Document;
use rullst::orm::async_trait;
use rullst::orm::polyglot::{CollectionName, DocumentId, DocumentRepository, PolyglotError};
use serde::{Serialize, de::DeserializeOwned};

/// The application's own replacement for `MongoDbStore`.
#[derive(Clone)]
pub struct MongoDocuments {
    database: mongodb::Database,
}

#[async_trait]
impl<T> DocumentRepository<T> for MongoDocuments
where
    T: Serialize + DeserializeOwned + Send + Sync,
{
    async fn create(
        &self,
        collection: &CollectionName,
        id: &DocumentId,
        entity: &T,
    ) -> Result<(), PolyglotError> {
        let mut document = mongodb::bson::to_document(entity)
            .map_err(|error| PolyglotError::Serialization(error.to_string()))?;
        document.insert("_id", id.as_str());
        match self.database.collection::<Document>(collection.as_str()).insert_one(document).await {
            Ok(_) => Ok(()),
            Err(error) if is_duplicate_key(&error) => Err(PolyglotError::Conflict),
            Err(error) => Err(PolyglotError::Driver { backend: "MongoDB", message: error.to_string() }),
        }
    }

    // `find`, `replace` (`PolyglotError::NotFound` when nothing matched),
    // `delete` and `list` (sorted by `_id`, with `page.offset()` and
    // `page.limit()`) follow the same shape; `DocumentInventory::list_entries`
    // returns each `_id` with its document for the recovery functions.
}
```
