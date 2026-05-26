use async_trait::async_trait;
use std::path::PathBuf;

#[derive(Debug)]
pub enum StorageError {
    IoError(std::io::Error),
    ConfigError(String),
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

#[async_trait]
pub trait StorageDriver: Send + Sync {
    async fn put(&self, path: &str, bytes: &[u8]) -> Result<(), StorageError>;
    async fn get(&self, path: &str) -> Result<Vec<u8>, StorageError>;
    async fn exists(&self, path: &str) -> Result<bool, StorageError>;
    async fn delete(&self, path: &str) -> Result<(), StorageError>;
    async fn url(&self, path: &str) -> Result<String, StorageError>;
}

/// A local disk storage driver
pub struct LocalDriver {
    pub root: PathBuf,
}

impl LocalDriver {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        LocalDriver { root: root.into() }
    }

    fn resolve_path(&self, path: &str) -> Result<PathBuf, StorageError> {
        let joined = self.root.join(path.trim_start_matches('/'));
        
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
            Ok(normalized)
        } else {
            Err(StorageError::DriverError("Access denied: path traversal attempt detected".to_string()))
        }
    }
}

#[async_trait]
impl StorageDriver for LocalDriver {
    async fn put(&self, path: &str, bytes: &[u8]) -> Result<(), StorageError> {
        let full_path = self.resolve_path(path)?;
        if let Some(parent) = full_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(full_path, bytes).await?;
        Ok(())
    }

    async fn get(&self, path: &str) -> Result<Vec<u8>, StorageError> {
        let full_path = self.resolve_path(path)?;
        let data = tokio::fs::read(full_path).await?;
        Ok(data)
    }

    async fn exists(&self, path: &str) -> Result<bool, StorageError> {
        let full_path = self.resolve_path(path)?;
        Ok(tokio::fs::metadata(full_path).await.is_ok())
    }

    async fn delete(&self, path: &str) -> Result<(), StorageError> {
        let full_path = self.resolve_path(path)?;
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
                if service_err.is_no_such_key() {
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
    pub async fn put(path: &str, bytes: &[u8]) -> Result<(), StorageError> {
        Self::disk("local").put(path, bytes).await
    }

    pub async fn get(path: &str) -> Result<Vec<u8>, StorageError> {
        Self::disk("local").get(path).await
    }

    pub async fn exists(path: &str) -> Result<bool, StorageError> {
        Self::disk("local").exists(path).await
    }

    pub async fn delete(path: &str) -> Result<(), StorageError> {
        Self::disk("local").delete(path).await
    }

    pub async fn url(path: &str) -> Result<String, StorageError> {
        Self::disk("local").url(path).await
    }

    pub fn disk(name: &str) -> Box<dyn StorageDriver> {
        match name {
            "local" => {
                let root =
                    std::env::var("STORAGE_ROOT").unwrap_or_else(|_| "storage/app".to_string());
                Box::new(LocalDriver::new(root))
            }
            "s3" => {
                #[cfg(feature = "storage-s3")]
                {
                    let bucket = std::env::var("AWS_BUCKET").unwrap_or_default();
                    let endpoint = std::env::var("AWS_ENDPOINT").ok();
                    let config = tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(aws_config::load_from_env())
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
}
