//! Optional capability-oriented persistence adapters.
//!
//! The SQLx-backed ORM remains the default. Turso exposes an explicitly
//! selected typed relational profile, while the document and graph backends do
//! not pretend to share relational semantics they cannot provide.

mod capability;
mod document;
mod error;
mod mock;
mod recovery;

#[cfg(feature = "surrealdb")]
mod surrealdb;
#[cfg(feature = "turso")]
mod turso;

pub use capability::{Backend, BackendCapabilities, Capability};
pub use document::{
    CollectionName, DocumentEntry, DocumentId, DocumentInventory, DocumentPage, DocumentRepository,
};
pub use error::PolyglotError;
pub use mock::MockDocumentStore;
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
