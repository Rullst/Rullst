//! Optional capability-oriented persistence adapters.
//!
//! The SQLx-backed ORM remains the default. Turso exposes an explicitly
//! selected typed relational profile, while document and analytics backends do
//! not pretend to share relational semantics they cannot provide.

mod capability;
mod document;
mod error;
mod mock;
mod recovery;

#[cfg(feature = "duckdb")]
mod duckdb;
#[cfg(feature = "mongodb")]
mod mongodb;
#[cfg(feature = "qdrant")]
mod qdrant;
#[cfg(feature = "surrealdb")]
mod surrealdb;
#[cfg(feature = "turso")]
mod turso;

pub use capability::{Backend, BackendCapabilities, Capability};
pub use document::{
    CollectionName, DocumentEntry, DocumentId, DocumentInventory, DocumentPage, DocumentRepository,
};
#[cfg(feature = "duckdb")]
pub use duckdb::{
    AnalyticsRepository, AnalyticsRow, AnalyticsTimeUnit, AnalyticsValue, DuckDbStore, QueryLimit,
};
pub use error::PolyglotError;
pub use mock::MockDocumentStore;
#[cfg(feature = "mongodb")]
pub use mongodb::MongoDbStore;
#[cfg(feature = "qdrant")]
pub use qdrant::{
    QdrantConfig, QdrantStore, VectorCollectionName, VectorDimensions, VectorMatch, VectorPoint,
    VectorQueryLimit, VectorRepository,
};
pub use recovery::{
    DocumentRecoveryBinding, DocumentRecoveryError, DocumentRecoveryKey, DocumentRecoveryPolicy,
    DocumentRecoveryReport, EncryptedDocumentSnapshot, export_document_snapshot,
    restore_document_snapshot,
};
#[cfg(feature = "surrealdb")]
pub use surrealdb::{GraphQuery, GraphRepository, SurrealAuth, SurrealConfig, SurrealDbStore};
#[cfg(feature = "turso")]
pub use turso::{
    TursoActiveRecord, TursoCodec, TursoConfig, TursoMigration, TursoMigrationReport, TursoModel,
    TursoOrder, TursoOrm, TursoPrimaryKey, TursoQuery, TursoQueryLimit, TursoRepository,
    TursoRollbackReport, TursoRow, TursoStatement, TursoStore, TursoValue,
};
