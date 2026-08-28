//! Unified Object Storage & Media Pipeline for Rullst applications.

use std::path::{Component, Path, PathBuf};

mod tenant;
pub use tenant::TenantStorage;

/// Strongly-typed error domain for Rullst Storage operations.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum StorageError {
    /// File or directory I/O failure.
    #[error("Storage I/O error: {0}")]
    Io(String),

    /// The requested object does not exist.
    #[error("Storage object not found: {0}")]
    NotFound(String),

    /// Path traversal attack attempt intercepted.
    #[error("Invalid path traversal attempt: {0}")]
    PathTraversal(String),

    /// Storage driver error.
    #[error("Storage driver error: {0}")]
    Driver(String),

    /// The selected backend or operation is not implemented by this build.
    #[error("Unsupported storage operation: {0}")]
    Unsupported(String),
}

impl From<std::io::Error> for StorageError {
    fn from(err: std::io::Error) -> Self {
        if err.kind() == std::io::ErrorKind::NotFound {
            StorageError::NotFound(err.to_string())
        } else {
            StorageError::Io(err.to_string())
        }
    }
}

/// Storage Driver Target Adapter
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum StorageDriver {
    /// Local file system storage
    Local {
        /// Base directory path on local disk
        base_path: String,
    },
    /// Amazon Web Services (AWS) S3 Storage
    S3 {
        /// S3 Bucket name
        bucket: String,
        /// AWS Region
        region: String,
    },
    /// Cloudflare R2 Object Storage
    R2 {
        /// R2 Bucket name
        bucket: String,
        /// Cloudflare Account ID
        account_id: String,
    },
}

/// Unified Storage Engine Client
#[derive(Debug, Clone)]
pub struct Storage {
    driver: StorageDriver,
}

impl Storage {
    /// Initialize local disk storage provider
    pub fn local<S: Into<String>>(base_path: S) -> Self {
        Self {
            driver: StorageDriver::Local {
                base_path: base_path.into(),
            },
        }
    }

    /// Initialize AWS S3 storage provider
    pub fn s3(bucket: impl Into<String>, region: impl Into<String>) -> Self {
        Self {
            driver: StorageDriver::S3 {
                bucket: bucket.into(),
                region: region.into(),
            },
        }
    }

    /// Initialize Cloudflare R2 storage provider
    pub fn r2(bucket: impl Into<String>, account_id: impl Into<String>) -> Self {
        Self {
            driver: StorageDriver::R2 {
                bucket: bucket.into(),
                account_id: account_id.into(),
            },
        }
    }

    /// Put binary payload to target path
    pub async fn put(&self, relative_path: &str, bytes: &[u8]) -> Result<(), StorageError> {
        match &self.driver {
            StorageDriver::Local { base_path } => {
                LocalDriver::new(base_path).put(relative_path, bytes).await
            }
            StorageDriver::S3 { .. } => Err(StorageError::Unsupported(
                "AWS S3 uploads require an installed S3 backend".to_string(),
            )),
            StorageDriver::R2 { .. } => Err(StorageError::Unsupported(
                "Cloudflare R2 uploads require an installed R2 backend".to_string(),
            )),
        }
    }

    /// Retrieve binary payload from target path
    pub async fn get(&self, relative_path: &str) -> Result<Vec<u8>, StorageError> {
        match &self.driver {
            StorageDriver::Local { base_path } => {
                LocalDriver::new(base_path).get(relative_path).await
            }
            StorageDriver::S3 { .. } => Err(StorageError::Unsupported(
                "AWS S3 downloads require an installed S3 backend".to_string(),
            )),
            StorageDriver::R2 { .. } => Err(StorageError::Unsupported(
                "Cloudflare R2 downloads require an installed R2 backend".to_string(),
            )),
        }
    }

    /// Public URL resolution helper for uploaded asset
    pub fn url(&self, relative_path: &str) -> Result<String, StorageError> {
        let relative_path = validate_relative_path(relative_path)?;
        let relative_path = relative_path.to_string_lossy();
        match &self.driver {
            StorageDriver::Local { base_path } => Ok(format!(
                "{}/{}",
                base_path.trim_end_matches('/'),
                relative_path.trim_start_matches('/')
            )),
            StorageDriver::S3 { bucket, region } => Ok(format!(
                "https://{}.s3.{}.amazonaws.com/{}",
                bucket,
                region,
                relative_path.trim_start_matches('/')
            )),
            StorageDriver::R2 { bucket, account_id } => Ok(format!(
                "https://{}.r2.cloudflarestorage.com/{}/{}",
                account_id,
                bucket,
                relative_path.trim_start_matches('/')
            )),
        }
    }

    /// Resize media buffer pipeline helper
    pub fn resize_webp(
        &self,
        bytes: &[u8],
        _width: u32,
        _height: u32,
    ) -> Result<Vec<u8>, StorageError> {
        let _ = bytes;
        Err(StorageError::Unsupported(
            "WebP resizing is not enabled in this build".to_string(),
        ))
    }
}

/// Local disk storage driver supporting path traversal protection
#[derive(Debug, Clone)]
pub struct LocalDriver {
    base_dir: PathBuf,
}

impl LocalDriver {
    /// Create new LocalDriver
    pub fn new<P: Into<PathBuf>>(base_dir: P) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }

    async fn canonical_base(&self) -> Result<PathBuf, StorageError> {
        tokio::fs::create_dir_all(&self.base_dir).await?;
        tokio::fs::canonicalize(&self.base_dir)
            .await
            .map_err(StorageError::from)
    }

    async fn resolve_existing_path(&self, relative_path: &str) -> Result<PathBuf, StorageError> {
        let validated_path = validate_relative_path(relative_path)?;
        let lexical_path = self.base_dir.join(&validated_path);
        let canonical_base = self.canonical_base().await?;
        reject_symlink_components(&canonical_base, &validated_path).await?;
        let canonical_path = tokio::fs::canonicalize(&lexical_path)
            .await
            .map_err(StorageError::from)?;

        if !canonical_path.starts_with(&canonical_base) {
            return Err(StorageError::PathTraversal(relative_path.to_string()));
        }

        Ok(canonical_path)
    }

    async fn resolve_write_path(&self, relative_path: &str) -> Result<PathBuf, StorageError> {
        let validated_path = validate_relative_path(relative_path)?;
        let lexical_path = self.base_dir.join(&validated_path);
        let canonical_base = self.canonical_base().await?;
        reject_symlink_components(&canonical_base, &validated_path).await?;
        let parent = lexical_path
            .parent()
            .ok_or_else(|| StorageError::PathTraversal("storage path has no parent".to_string()))?;

        tokio::fs::create_dir_all(parent).await?;
        let canonical_parent = tokio::fs::canonicalize(parent).await?;
        if !canonical_parent.starts_with(&canonical_base) {
            return Err(StorageError::PathTraversal(relative_path.to_string()));
        }

        if let Ok(metadata) = tokio::fs::symlink_metadata(&lexical_path).await
            && metadata.file_type().is_symlink()
        {
            return Err(StorageError::PathTraversal(relative_path.to_string()));
        }

        let file_name = lexical_path.file_name().ok_or_else(|| {
            StorageError::PathTraversal("storage path must identify a file".to_string())
        })?;
        Ok(canonical_parent.join(file_name))
    }

    /// Put binary payload to target path
    pub async fn put(&self, path: &str, bytes: &[u8]) -> Result<(), StorageError> {
        let full_path = self.resolve_write_path(path).await?;
        tokio::fs::write(full_path, bytes)
            .await
            .map_err(|e| StorageError::Io(e.to_string()))
    }

    /// Check if target path exists
    pub async fn exists(&self, path: &str) -> Result<bool, StorageError> {
        match self.resolve_existing_path(path).await {
            Ok(_) => Ok(true),
            Err(StorageError::NotFound(_)) => Ok(false),
            Err(error) => Err(error),
        }
    }

    /// Retrieve binary payload from target path
    pub async fn get(&self, path: &str) -> Result<Vec<u8>, StorageError> {
        let full_path = self.resolve_existing_path(path).await?;
        tokio::fs::read(full_path)
            .await
            .map_err(|e| StorageError::Io(e.to_string()))
    }

    /// Resolve public URL path
    pub async fn url(&self, path: &str) -> Result<String, StorageError> {
        let _ = validate_relative_path(path)?;
        Ok(format!("/storage/{}", path.trim_start_matches('/')))
    }

    /// Delete file from target path
    pub async fn delete(&self, path: &str) -> Result<(), StorageError> {
        let full_path = self.resolve_existing_path(path).await?;
        tokio::fs::remove_file(full_path)
            .await
            .map_err(|e| StorageError::Io(e.to_string()))
    }
}

fn validate_relative_path(relative_path: &str) -> Result<PathBuf, StorageError> {
    if relative_path.is_empty() || relative_path.contains('\\') {
        return Err(StorageError::PathTraversal(relative_path.to_string()));
    }

    let path = Path::new(relative_path);
    let mut validated = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(segment) => validated.push(segment),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(StorageError::PathTraversal(relative_path.to_string()));
            }
        }
    }

    if validated.as_os_str().is_empty() {
        return Err(StorageError::PathTraversal(relative_path.to_string()));
    }

    Ok(validated)
}

async fn reject_symlink_components(
    canonical_base: &Path,
    relative_path: &Path,
) -> Result<(), StorageError> {
    let mut candidate = canonical_base.to_path_buf();
    for component in relative_path.components() {
        let Component::Normal(segment) = component else {
            return Err(StorageError::PathTraversal(
                relative_path.to_string_lossy().into_owned(),
            ));
        };
        candidate.push(segment);
        match tokio::fs::symlink_metadata(&candidate).await {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(StorageError::PathTraversal(
                    relative_path.to_string_lossy().into_owned(),
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
            Err(error) => return Err(StorageError::from(error)),
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn relative_path_validation_rejects_escape_attempts() {
        for path in [
            "",
            "../secret",
            "a/../../secret",
            "/etc/passwd",
            r"..\\secret",
        ] {
            assert!(matches!(
                validate_relative_path(path),
                Err(StorageError::PathTraversal(_))
            ));
        }
    }

    #[tokio::test]
    async fn cloud_backends_never_report_false_success() {
        let s3 = Storage::s3("bucket", "sa-east-1");
        let r2 = Storage::r2("bucket", "account");

        assert!(matches!(
            s3.put("file.txt", b"payload").await,
            Err(StorageError::Unsupported(_))
        ));
        assert!(matches!(
            r2.get("file.txt").await,
            Err(StorageError::Unsupported(_))
        ));
    }

    #[test]
    fn resize_never_returns_unmodified_bytes_as_success() {
        let storage = Storage::local("storage");
        assert!(matches!(
            storage.resize_webp(b"not-an-image", 100, 100),
            Err(StorageError::Unsupported(_))
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn local_storage_rejects_symlink_escape() {
        use std::os::unix::fs::symlink;

        let suffix = uuid::Uuid::new_v4();
        let root = std::env::temp_dir().join(format!("rullst-storage-root-{suffix}"));
        let outside = std::env::temp_dir().join(format!("rullst-storage-outside-{suffix}"));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(outside.join("secret.txt"), b"secret").unwrap();
        symlink(&outside, root.join("escape")).unwrap();

        let storage = Storage::local(root.to_string_lossy());
        assert!(matches!(
            storage.get("escape/secret.txt").await,
            Err(StorageError::PathTraversal(_))
        ));
        assert!(matches!(
            storage.put("escape/created.txt", b"payload").await,
            Err(StorageError::PathTraversal(_))
        ));

        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(outside);
    }
}
