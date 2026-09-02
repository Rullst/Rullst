//! Bounded and policy-bound image source loading for guarded vision prompts.

use super::{EgressFetchError, EgressFetcher, EgressResolver};
use std::fmt;
use std::path::{Path, PathBuf};
use tokio::io::AsyncReadExt;

/// Maximum image size accepted by the high-level file and URL helpers.
pub const MAX_VISION_IMAGE_BYTES: u64 = 10 * 1_024 * 1_024;
const MAX_LOCAL_ROOTS: usize = 32;

/// Typed failures raised before an image reaches an AI provider.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum VisionInputError {
    /// No local root was supplied to the deny-by-default policy.
    #[error("at least one local image root must be explicitly allowed")]
    MissingLocalRoots,
    /// The local root count exceeds the bounded policy surface.
    #[error("local image policy accepts at most {MAX_LOCAL_ROOTS} roots")]
    TooManyLocalRoots,
    /// A configured byte limit is zero or exceeds the framework ceiling.
    #[error("local image byte limit must be between 1 byte and 10 MiB")]
    InvalidByteLimit,
    /// An allowlisted root is unavailable, is not a directory, or is too broad.
    #[error("local image root is unavailable, is not a directory, or is too broad")]
    InvalidLocalRoot,
    /// The requested file could not be resolved or opened.
    #[error("local image is unavailable")]
    LocalFileUnavailable,
    /// The canonical file path did not remain within an exact allowlisted root.
    #[error("local image is outside every allowed root")]
    LocalFileNotAllowed,
    /// The resolved source is not a regular file.
    #[error("local image source must be a regular file")]
    NotARegularFile,
    /// The image exceeded the configured byte budget.
    #[error("vision image exceeds the configured byte limit")]
    ImageTooLarge,
    /// The image magic bytes are outside the portable supported set.
    #[error("vision input must be JPEG, PNG, WebP, or GIF")]
    UnsupportedImageFormat,
    /// A remote response advertised a non-image or contradictory media type.
    #[error("remote vision content type does not match the supported image bytes")]
    ContentTypeMismatch,
    /// The policy-bound HTTPS fetch failed.
    #[error(transparent)]
    RemoteFetch(#[from] EgressFetchError),
}

/// Exact-root and byte-budget policy for local vision input.
///
/// Roots are canonicalized when the policy is created. Each requested file is
/// canonicalized again and must remain below one of those roots. The host must
/// still protect an allowlisted directory from adversarial rename races while
/// a file is being opened.
#[derive(Clone)]
pub struct LocalImagePolicy {
    allowed_roots: Vec<PathBuf>,
    max_bytes: u64,
}

impl fmt::Debug for LocalImagePolicy {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalImagePolicy")
            .field("allowed_root_count", &self.allowed_roots.len())
            .field("max_bytes", &self.max_bytes)
            .finish()
    }
}

impl LocalImagePolicy {
    /// Creates an exact-root policy with the 10 MiB framework ceiling.
    pub fn new<P, I>(roots: I) -> Result<Self, VisionInputError>
    where
        P: AsRef<Path>,
        I: IntoIterator<Item = P>,
    {
        Self::with_max_bytes(roots, MAX_VISION_IMAGE_BYTES)
    }

    /// Creates an exact-root policy with a stricter application byte budget.
    pub fn with_max_bytes<P, I>(roots: I, max_bytes: u64) -> Result<Self, VisionInputError>
    where
        P: AsRef<Path>,
        I: IntoIterator<Item = P>,
    {
        if max_bytes == 0 || max_bytes > MAX_VISION_IMAGE_BYTES {
            return Err(VisionInputError::InvalidByteLimit);
        }
        let roots = roots.into_iter().collect::<Vec<_>>();
        if roots.is_empty() {
            return Err(VisionInputError::MissingLocalRoots);
        }
        if roots.len() > MAX_LOCAL_ROOTS {
            return Err(VisionInputError::TooManyLocalRoots);
        }
        let mut allowed_roots = Vec::with_capacity(roots.len());
        for root in roots {
            let canonical = std::fs::canonicalize(root.as_ref())
                .map_err(|_| VisionInputError::InvalidLocalRoot)?;
            let metadata =
                std::fs::metadata(&canonical).map_err(|_| VisionInputError::InvalidLocalRoot)?;
            if !metadata.is_dir() || canonical.parent().is_none() {
                return Err(VisionInputError::InvalidLocalRoot);
            }
            allowed_roots.push(canonical);
        }
        allowed_roots.sort();
        allowed_roots.dedup();
        Ok(Self {
            allowed_roots,
            max_bytes,
        })
    }

    pub(crate) async fn read(&self, path: &Path) -> Result<Vec<u8>, VisionInputError> {
        let canonical = tokio::fs::canonicalize(path)
            .await
            .map_err(|_| VisionInputError::LocalFileUnavailable)?;
        if !self
            .allowed_roots
            .iter()
            .any(|root| canonical.starts_with(root))
        {
            return Err(VisionInputError::LocalFileNotAllowed);
        }
        let file = tokio::fs::File::open(canonical)
            .await
            .map_err(|_| VisionInputError::LocalFileUnavailable)?;
        let metadata = file
            .metadata()
            .await
            .map_err(|_| VisionInputError::LocalFileUnavailable)?;
        if !metadata.is_file() {
            return Err(VisionInputError::NotARegularFile);
        }
        if metadata.len() > self.max_bytes {
            return Err(VisionInputError::ImageTooLarge);
        }

        let read_limit = self
            .max_bytes
            .checked_add(1)
            .ok_or(VisionInputError::InvalidByteLimit)?;
        let mut image = Vec::new();
        let initial_capacity = usize::try_from(metadata.len())
            .unwrap_or(usize::MAX)
            .min(usize::try_from(self.max_bytes).unwrap_or(usize::MAX));
        image
            .try_reserve(initial_capacity)
            .map_err(|_| VisionInputError::ImageTooLarge)?;
        file.take(read_limit)
            .read_to_end(&mut image)
            .await
            .map_err(|_| VisionInputError::LocalFileUnavailable)?;
        validate_image(&image, self.max_bytes, None)?;
        Ok(image)
    }
}

pub(crate) async fn fetch_image<R>(
    fetcher: &EgressFetcher<R>,
    url: &str,
) -> Result<Vec<u8>, VisionInputError>
where
    R: EgressResolver,
{
    let resource = fetcher.fetch_bytes(url).await?;
    validate_image(
        &resource.body,
        MAX_VISION_IMAGE_BYTES,
        resource.content_type.as_deref(),
    )?;
    Ok(resource.body)
}

pub(crate) fn image_mime_type(image: &[u8]) -> Option<&'static str> {
    if image.starts_with(&[0xff, 0xd8, 0xff]) {
        Some("image/jpeg")
    } else if image.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if image.starts_with(b"RIFF") && image.get(8..12) == Some(b"WEBP".as_slice()) {
        Some("image/webp")
    } else if image.starts_with(b"GIF87a") || image.starts_with(b"GIF89a") {
        Some("image/gif")
    } else {
        None
    }
}

fn validate_image(
    image: &[u8],
    max_bytes: u64,
    advertised_content_type: Option<&str>,
) -> Result<(), VisionInputError> {
    let length = u64::try_from(image.len()).map_err(|_| VisionInputError::ImageTooLarge)?;
    if length == 0 || length > max_bytes {
        return Err(if length == 0 {
            VisionInputError::UnsupportedImageFormat
        } else {
            VisionInputError::ImageTooLarge
        });
    }
    let detected = image_mime_type(image).ok_or(VisionInputError::UnsupportedImageFormat)?;
    if let Some(advertised) = advertised_content_type {
        let advertised = advertised
            .split(';')
            .next()
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        if advertised != detected {
            return Err(VisionInputError::ContentTypeMismatch);
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_directory(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("rullst-ai-{label}-{}-{nonce}", std::process::id()))
    }

    #[tokio::test]
    async fn local_policy_reads_only_bounded_supported_images() {
        let root = test_directory("vision-root");
        std::fs::create_dir_all(&root).expect("create root");
        let image_path = root.join("pixel.png");
        std::fs::write(&image_path, b"\x89PNG\r\n\x1a\n\x00").expect("write image");

        let policy = LocalImagePolicy::with_max_bytes([&root], 16).expect("valid policy");
        assert_eq!(
            policy.read(&image_path).await.expect("allowed image"),
            b"\x89PNG\r\n\x1a\n\x00"
        );

        let mut oversized = b"\x89PNG\r\n\x1a\n".to_vec();
        oversized.resize(17, 0);
        std::fs::write(&image_path, oversized).expect("write oversized image");
        assert!(matches!(
            policy.read(&image_path).await,
            Err(VisionInputError::ImageTooLarge)
        ));
        std::fs::remove_dir_all(root).expect("remove root");
    }

    #[tokio::test]
    async fn local_policy_rejects_escape_and_unsupported_content() {
        let root = test_directory("vision-allowed");
        let outside = test_directory("vision-outside");
        std::fs::create_dir_all(&root).expect("create root");
        std::fs::create_dir_all(&outside).expect("create outside");
        let outside_image = outside.join("outside.png");
        std::fs::write(&outside_image, b"\x89PNG\r\n\x1a\n").expect("write image");
        let invalid = root.join("invalid.png");
        std::fs::write(&invalid, b"not an image").expect("write invalid image");
        let policy = LocalImagePolicy::new([&root]).expect("valid policy");

        assert!(matches!(
            policy.read(&outside_image).await,
            Err(VisionInputError::LocalFileNotAllowed)
        ));
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&outside_image, root.join("escaped.png"))
                .expect("create escape symlink");
            assert!(matches!(
                policy.read(&root.join("escaped.png")).await,
                Err(VisionInputError::LocalFileNotAllowed)
            ));
        }
        assert!(matches!(
            policy.read(&invalid).await,
            Err(VisionInputError::UnsupportedImageFormat)
        ));

        std::fs::remove_dir_all(root).expect("remove root");
        std::fs::remove_dir_all(outside).expect("remove outside");
    }

    #[test]
    fn remote_content_type_must_match_magic_bytes() {
        let png = *b"\x89PNG\r\n\x1a\n\x00";
        assert!(validate_image(&png, 16, Some("image/png; charset=binary")).is_ok());
        assert!(matches!(
            validate_image(&png, 16, Some("text/html")),
            Err(VisionInputError::ContentTypeMismatch)
        ));
    }

    #[test]
    fn configuration_is_fail_closed_and_debug_hides_roots() {
        assert!(matches!(
            LocalImagePolicy::new::<&Path, _>([]),
            Err(VisionInputError::MissingLocalRoots)
        ));
        assert!(matches!(
            LocalImagePolicy::with_max_bytes([std::env::temp_dir()], 0),
            Err(VisionInputError::InvalidByteLimit)
        ));
        let policy = LocalImagePolicy::new([std::env::temp_dir()]).expect("temp root");
        let debug = format!("{policy:?}");
        assert!(debug.contains("allowed_root_count"));
        assert!(!debug.contains(std::env::temp_dir().to_string_lossy().as_ref()));
    }
}
