use std::sync::OnceLock;

use async_trait::async_trait;

use super::{
    PolyglotError, TursoCodec, TursoConfig, TursoMigration, TursoMigrationReport, TursoModel,
    TursoOrder, TursoQuery, TursoRepository, TursoRollbackReport, TursoStore,
};
use crate::PaginationResult;

static TURSO_PRIMARY: OnceLock<TursoStore> = OnceLock::new();

/// Global facade used when Turso/libSQL is the application's primary database.
pub struct TursoOrm;

impl TursoOrm {
    /// Connects and publishes the primary store exactly once.
    pub async fn init(config: TursoConfig) -> Result<(), PolyglotError> {
        if TURSO_PRIMARY.get().is_some() {
            return Err(PolyglotError::InvalidConfiguration {
                backend: "Turso primary",
                reason: "primary store is already initialized",
            });
        }
        let store = TursoStore::connect(config).await?;
        TURSO_PRIMARY
            .set(store)
            .map_err(|_| PolyglotError::InvalidConfiguration {
                backend: "Turso primary",
                reason: "primary store was initialized concurrently",
            })
    }

    /// Initializes from the generated Turso environment contract.
    ///
    /// Empty or `mock_*` endpoints select the deterministic fallback persisted
    /// at `TURSO_OFFLINE_PATH` (default: `turso-development.db`).
    pub async fn init_from_env() -> Result<(), PolyglotError> {
        let url = std::env::var("TURSO_DATABASE_URL").unwrap_or_default();
        let token = std::env::var("TURSO_AUTH_TOKEN").unwrap_or_default();
        let mut config = TursoConfig::new(&url, token);
        if url.is_empty() || url.starts_with("mock_") || url.starts_with("mock://") {
            let path = std::env::var("TURSO_OFFLINE_PATH")
                .unwrap_or_else(|_| "turso-development.db".to_owned());
            config = config.with_offline_path(path)?;
        }
        Self::init(config).await
    }

    /// Returns the initialized primary store.
    pub fn store() -> Result<&'static TursoStore, PolyglotError> {
        TURSO_PRIMARY
            .get()
            .ok_or(PolyglotError::InvalidConfiguration {
                backend: "Turso primary",
                reason: "call TursoOrm::init or init_from_env before database operations",
            })
    }

    /// Returns a typed repository over the primary store.
    pub fn repository<Model>() -> Result<TursoRepository<'static, Model>, PolyglotError>
    where
        Model: TursoModel,
    {
        Ok(Self::store()?.models())
    }

    /// Applies ordered checksummed migrations to the primary store.
    pub async fn migrate(
        migrations: Vec<TursoMigration>,
    ) -> Result<TursoMigrationReport, PolyglotError> {
        Self::store()?.migrate(migrations).await
    }

    /// Returns the ordered applied-migration history.
    pub async fn migration_status() -> Result<Vec<String>, PolyglotError> {
        Self::store()?.migration_status().await
    }

    /// Rolls back the latest migration using its declared down statements.
    pub async fn rollback_last(
        migrations: Vec<TursoMigration>,
    ) -> Result<TursoRollbackReport, PolyglotError> {
        Self::store()?.rollback_last(migrations).await
    }
}

/// Active Record conveniences for typed Turso models.
#[async_trait]
pub trait TursoActiveRecord: TursoModel {
    /// Starts a typed query against the initialized primary store.
    fn query() -> Result<TursoQuery<'static, Self>, PolyglotError> {
        Ok(TursoOrm::repository::<Self>()?.query())
    }

    /// Returns at most 10,000 rows ordered by primary key.
    async fn all() -> Result<Vec<Self>, PolyglotError> {
        TursoOrm::repository::<Self>()?.all().await
    }

    /// Finds one row by primary key.
    async fn find<Key>(key: Key) -> Result<Option<Self>, PolyglotError>
    where
        Key: TursoCodec + Send,
    {
        TursoOrm::repository::<Self>()?.find(key).await
    }

    /// Inserts a new model or updates one whose primary key is set.
    async fn save(&mut self) -> Result<(), PolyglotError> {
        TursoOrm::repository::<Self>()?.save(self).await
    }

    /// Inserts this model, including an application-assigned primary key.
    async fn create(&mut self) -> Result<(), PolyglotError> {
        TursoOrm::repository::<Self>()?.create(self).await
    }

    /// Deletes this model by primary key.
    async fn delete(&self) -> Result<(), PolyglotError> {
        TursoOrm::repository::<Self>()?.delete(self).await
    }

    /// Counts all rows for this model.
    async fn count() -> Result<u64, PolyglotError> {
        Self::query()?.count().await
    }

    /// Returns one checked page ordered by primary key.
    async fn paginate(
        page: usize,
        per_page: usize,
    ) -> Result<PaginationResult<Self>, PolyglotError> {
        if page == 0 || per_page == 0 || per_page > 500 {
            return Err(PolyglotError::InvalidIdentifier {
                kind: "Turso pagination",
                reason: "page must be positive and per_page must be between 1 and 500",
            });
        }
        let offset = page
            .checked_sub(1)
            .and_then(|value| value.checked_mul(per_page))
            .ok_or(PolyglotError::InvalidIdentifier {
                kind: "Turso pagination",
                reason: "page offset overflowed",
            })?;
        let total = Self::count().await?;
        let data = Self::query()?
            .order_by(Self::primary_key_column(), TursoOrder::Asc)?
            .limit(u32::try_from(per_page).map_err(PolyglotError::serialization)?)?
            .offset(u64::try_from(offset).map_err(PolyglotError::serialization)?)?
            .get()
            .await?;
        let total_usize = usize::try_from(total).unwrap_or(usize::MAX);
        let last_page = total_usize.div_ceil(per_page);
        Ok(PaginationResult {
            data,
            total: i64::try_from(total).unwrap_or(i64::MAX),
            per_page,
            current_page: page,
            last_page,
        })
    }
}

impl<Model> TursoActiveRecord for Model where Model: TursoModel {}
