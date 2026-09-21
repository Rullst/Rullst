#[cfg(feature = "storage-s3")]
use super::cloud::{CloudClient, CloudStorageConfig, SignedDownload};
use super::{LocalDriver, Storage, StorageDriver, StorageError, TenantStorage};
#[cfg(feature = "storage-s3")]
use std::{sync::Arc, time::Duration};

/// Metadata for one object. ETags are opaque provider values, not SHA-256 guarantees.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectMetadata {
    /// Byte length reported by the backend.
    pub size_bytes: u64,
    /// Opaque entity tag when supplied by the backend; local files have no ETag.
    pub etag: Option<String>,
}

impl Storage {
    /// Configures an existing S3/R2 destination without environment lookups or network I/O.
    /// Use `CloudStorageConfig::require_production` at a production startup boundary.
    #[cfg(feature = "storage-s3")]
    pub fn with_cloud_config(mut self, config: CloudStorageConfig) -> Result<Self, StorageError> {
        self.cloud = Some(Arc::new(CloudClient::new(&self.driver, config)?));
        Ok(self)
    }

    /// Whether this configured cloud destination is the deterministic offline fallback.
    #[cfg(feature = "storage-s3")]
    pub fn is_cloud_mock(&self) -> bool {
        self.cloud.as_ref().is_some_and(|cloud| cloud.is_mock())
    }

    /// Returns object metadata after application authorization.
    pub async fn metadata(&self, key: &str) -> Result<ObjectMetadata, StorageError> {
        #[cfg(feature = "storage-s3")]
        if let Some(cloud) = &self.cloud {
            return cloud.metadata(key).await.map_err(StorageError::from);
        }
        if let StorageDriver::Local { base_path } = &self.driver {
            let path = LocalDriver::new(base_path)
                .resolve_existing_path(key)
                .await?;
            let metadata = tokio::fs::metadata(path).await?;
            return Ok(ObjectMetadata {
                size_bytes: metadata.len(),
                etag: None,
            });
        }
        Err(StorageError::Unsupported(
            "cloud storage is not configured".into(),
        ))
    }

    /// Deletes an object. Remote deletion of an absent key is idempotent.
    /// Versioned buckets may retain earlier versions according to provider policy.
    pub async fn delete(&self, key: &str) -> Result<(), StorageError> {
        #[cfg(feature = "storage-s3")]
        if let Some(cloud) = &self.cloud {
            return cloud.delete(key).await.map_err(StorageError::from);
        }
        if let StorageDriver::Local { base_path } = &self.driver {
            return LocalDriver::new(base_path).delete(key).await;
        }
        Err(StorageError::Unsupported(
            "cloud storage is not configured".into(),
        ))
    }

    /// Grants private GET access after the caller authorizes the current user/object.
    /// The bearer URL is reusable until expiry; this performs no object-existence check.
    #[cfg(feature = "storage-s3")]
    pub fn signed_download(
        &self,
        key: &str,
        lifetime: Duration,
    ) -> Result<SignedDownload, StorageError> {
        self.cloud
            .as_ref()
            .ok_or_else(|| {
                StorageError::Unsupported(
                    "signed downloads require a configured cloud backend".into(),
                )
            })?
            .signed_download(key, lifetime)
            .map_err(StorageError::from)
    }
}

impl TenantStorage {
    /// Reads metadata inside this authenticated tenant's namespace.
    pub async fn metadata(&self, relative_path: &str) -> Result<ObjectMetadata, StorageError> {
        self.storage
            .metadata(&self.object_key(relative_path)?)
            .await
    }

    /// Deletes only inside this tenant's namespace after application authorization.
    pub async fn delete(&self, relative_path: &str) -> Result<(), StorageError> {
        self.storage.delete(&self.object_key(relative_path)?).await
    }

    /// Issues a private bearer download confined to this tenant's namespace.
    /// The host must still authorize the account and this specific object first.
    #[cfg(feature = "storage-s3")]
    pub fn signed_download(
        &self,
        relative_path: &str,
        lifetime: Duration,
    ) -> Result<SignedDownload, StorageError> {
        self.storage
            .signed_download(&self.object_key(relative_path)?, lifetime)
    }
}
