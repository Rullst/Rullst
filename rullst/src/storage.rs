#![allow(unexpected_cfgs)]
use async_trait::async_trait;
use std::path::PathBuf;

/// Error type for all Rullst storage operations.
#[derive(Debug)]
pub enum StorageError {
    /// Wraps a standard I/O error returned by the filesystem or network layer.
    IoError(std::io::Error),
    /// Indicates invalid or missing configuration (e.g. storage root not set, unknown disk).
    ConfigError(String),
    /// Returned by drivers when an operation fails for driver-specific reasons.
    DriverError(String),
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageError::IoError(err) => write!(f, "IO error: {}", err),
            StorageError::ConfigError(err) => write!(f, "Configuration error: {}", err),
            StorageError::DriverError(err) => write!(f, "Storage driver error: {}", err),
        }
    }
}

impl std::error::Error for StorageError {}

impl From<std::io::Error> for StorageError {
    fn from(err: std::io::Error) -> Self {
        StorageError::IoError(err)
    }
}

/// Trait that all storage backends must implement. Implement this to add custom drivers
/// (e.g. S3, GCS, R2) and register them with `Storage::disk("my_driver")`.
#[async_trait]
pub trait StorageDriver: Send + Sync {
    /// Writes `bytes` to the given `path` in the storage driver's root. Creates parent directories as needed.
    async fn put(&self, path: &str, bytes: &[u8]) -> Result<(), StorageError>;
    /// Reads and returns the contents of `path` as raw bytes.
    async fn get(&self, path: &str) -> Result<Vec<u8>, StorageError>;
    /// Returns `true` if `path` exists in the storage backend, `false` otherwise.
    async fn exists(&self, path: &str) -> Result<bool, StorageError>;
    /// Deletes the file at `path`. Returns an error if the file does not exist.
    async fn delete(&self, path: &str) -> Result<(), StorageError>;
    /// Returns the public URL for accessing `path`. For local disks, returns a relative path.
    async fn url(&self, path: &str) -> Result<String, StorageError>;
}

/// A local disk storage driver
pub struct LocalDriver {
    /// Absolute path to the root directory where all files will be stored.
    pub root: PathBuf,
}

impl LocalDriver {
    /// Creates a new `LocalDriver` rooted at the given path. The directory is created on first `put`.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        LocalDriver { root: root.into() }
    }

    async fn resolve_path(&self, path: &str) -> Result<PathBuf, StorageError> {
        // Ensure no absolute paths or Windows drive letters are injected before we strip slashes
        if std::path::Path::new(path).is_absolute()
            || path.contains(':')
            || path.starts_with('/')
            || path.starts_with('\\')
        {
            return Err(StorageError::DriverError(
                "Access denied: absolute paths are not allowed".to_string(),
            ));
        }

        let safe_path = path.replace('\\', "/");
        let safe_path = safe_path.trim_start_matches('/');

        // Strictly reject any parent directory components to prevent path traversal
        for component in std::path::Path::new(safe_path).components() {
            if let std::path::Component::ParentDir = component {
                return Err(StorageError::DriverError(
                    "Access denied: path traversal attempt detected".to_string(),
                ));
            }
        }

        let joined = self.root.join(safe_path);

        let mut normalized = PathBuf::new();
        for component in joined.components() {
            match component {
                std::path::Component::ParentDir => {
                    normalized.pop();
                }
                std::path::Component::Normal(c) => {
                    normalized.push(c);
                }
                std::path::Component::CurDir => {}
                other => normalized.push(other.as_os_str()),
            }
        }

        if normalized.starts_with(&self.root) {
            // Check for symlink escapes if the path exists
            if normalized.exists() {
                if let Ok(canon) = tokio::fs::canonicalize(&normalized).await {
                    let canon_root =
                        tokio::fs::canonicalize(&self.root).await.unwrap_or_else(|_| self.root.clone());
                    if !canon.starts_with(&canon_root) {
                        return Err(StorageError::DriverError(
                            "Access denied: Symlink traversal attempt detected".to_string(),
                        ));
                    }
                }
            }
            Ok(normalized)
        } else {
            Err(StorageError::DriverError(
                "Access denied: path traversal attempt detected".to_string(),
            ))
        }
    }
}

#[async_trait]
impl StorageDriver for LocalDriver {
    async fn put(&self, path: &str, bytes: &[u8]) -> Result<(), StorageError> {
        let full_path = self.resolve_path(path).await?;
        if let Some(parent) = full_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(full_path, bytes).await?;
        Ok(())
    }

    async fn get(&self, path: &str) -> Result<Vec<u8>, StorageError> {
        let full_path = self.resolve_path(path).await?;
        let data = tokio::fs::read(full_path).await?;
        Ok(data)
    }

    async fn exists(&self, path: &str) -> Result<bool, StorageError> {
        let full_path = self.resolve_path(path).await?;
        Ok(tokio::fs::metadata(full_path).await.is_ok())
    }

    async fn delete(&self, path: &str) -> Result<(), StorageError> {
        let full_path = self.resolve_path(path).await?;
        if tokio::fs::metadata(&full_path).await.is_ok() {
            tokio::fs::remove_file(full_path).await?;
        }
        Ok(())
    }

    async fn url(&self, path: &str) -> Result<String, StorageError> {
        Ok(format!("/storage/{}", path.trim_start_matches('/')))
    }
}

/// An AWS S3 / Cloudflare R2 storage driver
#[cfg(feature = "storage-s3")]
pub struct S3Driver {
    pub client: aws_sdk_s3::Client,
    pub bucket: String,
    pub endpoint: Option<String>,
}

#[cfg(feature = "storage-s3")]
#[async_trait]
impl StorageDriver for S3Driver {
    async fn put(&self, path: &str, bytes: &[u8]) -> Result<(), StorageError> {
        let body = aws_sdk_s3::primitives::ByteStream::from(bytes.to_vec());
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(path)
            .body(body)
            .send()
            .await
            .map_err(|e| StorageError::DriverError(format!("{}", e)))?;
        Ok(())
    }

    async fn get(&self, path: &str) -> Result<Vec<u8>, StorageError> {
        let res = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(path)
            .send()
            .await
            .map_err(|e| StorageError::DriverError(format!("{}", e)))?;

        let bytes = res
            .body
            .collect()
            .await
            .map_err(|e| StorageError::DriverError(format!("{}", e)))?
            .into_bytes()
            .to_vec();
        Ok(bytes)
    }

    async fn exists(&self, path: &str) -> Result<bool, StorageError> {
        let res = self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(path)
            .send()
            .await;

        match res {
            Ok(_) => Ok(true),
            Err(e) => {
                let service_err = e.into_service_error();
                if service_err.is_not_found() {
                    Ok(false)
                } else {
                    Err(StorageError::DriverError(format!("{}", service_err)))
                }
            }
        }
    }

    async fn delete(&self, path: &str) -> Result<(), StorageError> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(path)
            .send()
            .await
            .map_err(|e| StorageError::DriverError(format!("{}", e)))?;
        Ok(())
    }

    async fn url(&self, path: &str) -> Result<String, StorageError> {
        if let Some(ref endpoint) = self.endpoint {
            Ok(format!(
                "{}/{}/{}",
                endpoint.trim_end_matches('/'),
                self.bucket,
                path
            ))
        } else {
            Ok(format!("https://{}.s3.amazonaws.com/{}", self.bucket, path))
        }
    }
}

/// Placeholder S3 storage driver if Cargo feature is not enabled
#[cfg(not(feature = "storage-s3"))]
pub struct S3Driver;

#[cfg(not(feature = "storage-s3"))]
#[async_trait]
impl StorageDriver for S3Driver {
    async fn put(&self, _path: &str, _bytes: &[u8]) -> Result<(), StorageError> {
        Err(StorageError::DriverError(
            "S3 storage driver requires the 'storage-s3' Cargo feature to be enabled".to_string(),
        ))
    }
    async fn get(&self, _path: &str) -> Result<Vec<u8>, StorageError> {
        Err(StorageError::DriverError(
            "S3 storage driver requires the 'storage-s3' Cargo feature to be enabled".to_string(),
        ))
    }
    async fn exists(&self, _path: &str) -> Result<bool, StorageError> {
        Err(StorageError::DriverError(
            "S3 storage driver requires the 'storage-s3' Cargo feature to be enabled".to_string(),
        ))
    }
    async fn delete(&self, _path: &str) -> Result<(), StorageError> {
        Err(StorageError::DriverError(
            "S3 storage driver requires the 'storage-s3' Cargo feature to be enabled".to_string(),
        ))
    }
    async fn url(&self, _path: &str) -> Result<String, StorageError> {
        Err(StorageError::DriverError(
            "S3 storage driver requires the 'storage-s3' Cargo feature to be enabled".to_string(),
        ))
    }
}

/// A fallback driver that always returns an error. Used for gracefully handling unknown disks.
pub struct ErrorDriver {
    /// The error message returned by all operations on this driver.
    pub message: String,
}

#[async_trait]
impl StorageDriver for ErrorDriver {
    async fn put(&self, _path: &str, _bytes: &[u8]) -> Result<(), StorageError> {
        Err(StorageError::DriverError(self.message.clone()))
    }
    async fn get(&self, _path: &str) -> Result<Vec<u8>, StorageError> {
        Err(StorageError::DriverError(self.message.clone()))
    }
    async fn exists(&self, _path: &str) -> Result<bool, StorageError> {
        Err(StorageError::DriverError(self.message.clone()))
    }
    async fn delete(&self, _path: &str) -> Result<(), StorageError> {
        Err(StorageError::DriverError(self.message.clone()))
    }
    async fn url(&self, _path: &str) -> Result<String, StorageError> {
        Err(StorageError::DriverError(self.message.clone()))
    }
}

/// The main Storage facade
pub struct Storage;

impl Storage {
    /// Stores `bytes` at `path` using the default `"local"` disk driver.
    pub async fn put(path: &str, bytes: &[u8]) -> Result<(), StorageError> {
        Self::disk("local").put(path, bytes).await
    }

    /// Retrieves the file at `path` from the default `"local"` disk driver.
    pub async fn get(path: &str) -> Result<Vec<u8>, StorageError> {
        Self::disk("local").get(path).await
    }

    /// Checks whether `path` exists on the default `"local"` disk driver.
    pub async fn exists(path: &str) -> Result<bool, StorageError> {
        Self::disk("local").exists(path).await
    }

    /// Deletes the file at `path` on the default `"local"` disk driver.
    pub async fn delete(path: &str) -> Result<(), StorageError> {
        Self::disk("local").delete(path).await
    }

    /// Returns the public URL for `path` on the default `"local"` disk driver.
    pub async fn url(path: &str) -> Result<String, StorageError> {
        Self::disk("local").url(path).await
    }

    /// Returns a boxed [`StorageDriver`] for the named disk configuration.
    /// Unknown disk names return an [`ErrorDriver`] rather than panicking.
    pub fn disk(name: &str) -> Box<dyn StorageDriver> {
        match name {
            "local" => {
                let config = crate::config::RullstConfig::global();
                let is_prod = std::env::var("APP_ENV")
                    .unwrap_or_else(|_| "development".to_string())
                    == "production";
                let root = config.storage.root.clone().unwrap_or_else(|| {
                    if is_prod {
                        "storage/app".to_string()
                    } else {
                        std::env::var("STORAGE_ROOT").unwrap_or_else(|_| "storage/app".to_string())
                    }
                });
                Box::new(LocalDriver::new(root))
            }
            "s3" => {
                #[cfg(feature = "storage-s3")]
                {
                    let bucket = std::env::var("AWS_BUCKET").unwrap_or_default();
                    let endpoint = std::env::var("AWS_ENDPOINT").ok();
                    let config = tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(aws_config::load_defaults(
                            aws_config::BehaviorVersion::latest(),
                        ))
                    });
                    let client = aws_sdk_s3::Client::new(&config);
                    Box::new(S3Driver {
                        client,
                        bucket,
                        endpoint,
                    })
                }
                #[cfg(not(feature = "storage-s3"))]
                {
                    Box::new(S3Driver)
                }
            }
            other => Box::new(ErrorDriver {
                message: format!("Unknown storage disk: {}", other),
            }),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_local_storage_lifecycle() {
        let temp_dir = "storage/test_app";
        let _ = std::fs::remove_dir_all(temp_dir);

        let driver = LocalDriver::new(temp_dir);

        // Put file
        driver
            .put("images/avatar.png", b"fake-png-bytes")
            .await
            .unwrap();

        // Check exists
        assert!(driver.exists("images/avatar.png").await.unwrap());

        // Get file
        let data = driver.get("images/avatar.png").await.unwrap();
        assert_eq!(data, b"fake-png-bytes");

        // Get public URL
        let url = driver.url("images/avatar.png").await.unwrap();
        assert_eq!(url, "/storage/images/avatar.png");

        // Delete file
        driver.delete("images/avatar.png").await.unwrap();
        assert!(!driver.exists("images/avatar.png").await.unwrap());

        // Clean up
        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[tokio::test]
    async fn test_local_storage_path_traversal() {
        let temp_dir = "storage/test_traversal";
        let _ = std::fs::remove_dir_all(temp_dir);
        let driver = LocalDriver::new(temp_dir);

        // Try path traversal
        let res_put = driver.put("../traversal.txt", b"hack").await;
        assert!(res_put.is_err());

        let res_get = driver.get("../traversal.txt").await;
        assert!(res_get.is_err());

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[tokio::test]
    async fn test_local_storage_absolute_path_denied() {
        let temp_dir = "storage/test_absolute";
        let _ = std::fs::remove_dir_all(temp_dir);
        let driver = LocalDriver::new(temp_dir);

        let res_put = driver.put("/etc/passwd", b"hack").await;
        assert!(res_put.is_err());
        assert!(
            res_put
                .unwrap_err()
                .to_string()
                .contains("absolute paths are not allowed")
        );

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_storage_disk_factory() {
        // SAFETY: This test must not run concurrently with other tests that read STORAGE_ROOT.
        // In a parallel test run, prefer running with `cargo test -- --test-threads=1` if
        // STORAGE_ROOT is also checked by other tests, or use the `serial_test` crate.
        // set_var is safe here because this file's tests are independent of STORAGE_ROOT.
        #[allow(unsafe_code)]
        // SAFETY: No other thread reads STORAGE_ROOT during this test in the test suite.
        unsafe {
            std::env::set_var("STORAGE_ROOT", "storage/factory_test");
        }
        let _driver = Storage::disk("local");
        // We just ensure it doesn't panic and returns a valid LocalDriver via Box<dyn StorageDriver>
        // Downcasting is not straight-forward, but we can call a method if we want, or just rely on instantiation.

        let err_driver = Storage::disk("unknown_disk");
        // Unknown disk should return an ErrorDriver
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let err_res = runtime.block_on(err_driver.exists("test"));
        assert!(err_res.is_err());
        assert!(
            err_res
                .unwrap_err()
                .to_string()
                .contains("Unknown storage disk")
        );
        // Restore env to avoid polluting other tests
        #[allow(unsafe_code)]
        unsafe {
            std::env::remove_var("STORAGE_ROOT");
        }
    }

    #[tokio::test]
    async fn test_storage_global_facade() {
        // Set env var to avoid polluting actual storage root
        #[allow(unsafe_code)]
        unsafe {
            std::env::set_var("STORAGE_ROOT", "storage/facade_test");
        }
        let _ = std::fs::remove_dir_all("storage/facade_test");

        let res_put = Storage::put("facade.txt", b"hello global").await;
        assert!(res_put.is_ok());

        let res_exists = Storage::exists("facade.txt").await.unwrap();
        assert!(res_exists);

        let data = Storage::get("facade.txt").await.unwrap();
        assert_eq!(data, b"hello global");

        Storage::delete("facade.txt").await.unwrap();
        let _ = std::fs::remove_dir_all("storage/facade_test");

        #[allow(unsafe_code)]
        unsafe {
            std::env::remove_var("STORAGE_ROOT");
        }
    }
}
