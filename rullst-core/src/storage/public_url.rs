//! Percent-encoded object paths for public storage URLs.

use std::fmt::Write;

/// Percent-encodes every byte of each `/`-separated key segment except the
/// RFC 3986 unreserved set `A-Z a-z 0-9 - . _ ~` (uppercase hex).
///
/// `key` must already be normalized, so `/` only separates segments. A `#`,
/// `?`, `%`, space or non-ASCII byte therefore stays part of the object key
/// instead of starting a fragment or query or being re-decoded.
pub(super) fn encode_key_path(key: &str) -> String {
    encode(key, true)
}

/// Unsigned S3 object URL for an already encoded key path, following the
/// endpoint rules `CloudClient` uses: `cn-*` regions live in the
/// `amazonaws.com.cn` partition, and a bucket name containing `.` is addressed
/// path-style because the `*.s3.<region>.amazonaws.com` wildcard certificate
/// does not cover a multi-label virtual host.
pub(super) fn s3_object_url(bucket: &str, region: &str, path: &str) -> String {
    let suffix = if region.starts_with("cn-") {
        "amazonaws.com.cn"
    } else {
        "amazonaws.com"
    };
    if bucket.contains('.') {
        let bucket = encode(bucket, false);
        format!("https://s3.{region}.{suffix}/{bucket}/{path}")
    } else {
        format!("https://{bucket}.s3.{region}.{suffix}/{path}")
    }
}

fn encode(value: &str, keep_separators: bool) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if (keep_separators && byte == b'/')
            || byte.is_ascii_alphanumeric()
            || matches!(byte, b'-' | b'.' | b'_' | b'~')
        {
            encoded.push(char::from(byte));
        } else {
            // Writing into a `String` cannot fail.
            let _ = write!(encoded, "%{byte:02X}");
        }
    }
    encoded
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::encode_key_path;
    use crate::storage::{LocalDriver, Storage};

    #[test]
    fn key_segments_are_percent_encoded_but_separators_are_kept() {
        assert_eq!(
            encode_key_path("courses/1/lesson.txt"),
            "courses/1/lesson.txt"
        );
        assert_eq!(encode_key_path("a-b_c.d~e"), "a-b_c.d~e");
        assert_eq!(
            encode_key_path("reports/Q1 #2?.pdf"),
            "reports/Q1%20%232%3F.pdf"
        );
        assert_eq!(encode_key_path("100%/ação"), "100%25/a%C3%A7%C3%A3o");
        assert_eq!(encode_key_path("a+b=c&d:e"), "a%2Bb%3Dc%26d%3Ae");
    }

    #[test]
    fn cloud_urls_keep_reserved_characters_inside_the_object_key() {
        let s3 = Storage::s3("assets", "us-east-1");
        let r2 = Storage::r2("assets", "account");

        assert_eq!(
            s3.url("reports/Q1 #2.pdf").unwrap(),
            "https://assets.s3.us-east-1.amazonaws.com/reports/Q1%20%232.pdf"
        );
        assert_eq!(
            r2.url("reports/ação?.pdf").unwrap(),
            "https://account.r2.cloudflarestorage.com/assets/reports/a%C3%A7%C3%A3o%3F.pdf"
        );
    }

    #[test]
    fn s3_urls_follow_the_partition_and_dotted_bucket_endpoint_rules() {
        assert_eq!(
            Storage::s3("assets", "cn-north-1").url("a.png").unwrap(),
            "https://assets.s3.cn-north-1.amazonaws.com.cn/a.png"
        );
        // A dotted virtual host fails the provider's wildcard certificate.
        assert_eq!(
            Storage::s3("assets.example.com", "us-east-1")
                .url("a b.png")
                .unwrap(),
            "https://s3.us-east-1.amazonaws.com/assets.example.com/a%20b.png"
        );
        assert_eq!(
            Storage::s3("assets.example.com", "cn-northwest-1")
                .url("a.png")
                .unwrap(),
            "https://s3.cn-northwest-1.amazonaws.com.cn/assets.example.com/a.png"
        );
        assert_eq!(
            Storage::s3("assets", "eu-west-1").url("a.png").unwrap(),
            "https://assets.s3.eu-west-1.amazonaws.com/a.png"
        );
    }

    #[tokio::test]
    async fn local_urls_are_public_paths_not_filesystem_paths() {
        let absolute = Storage::local("/srv/app/storage");
        let relative = Storage::local("storage");
        let driver = LocalDriver::new("/srv/app/storage");

        for storage in [&absolute, &relative] {
            assert_eq!(storage.url("a.png").unwrap(), "/storage/a.png");
            assert_eq!(
                storage.url("avatars/my photo#1.png").unwrap(),
                "/storage/avatars/my%20photo%231.png"
            );
        }
        // Both local URL helpers resolve the same object to the same URL.
        assert_eq!(
            driver.url("avatars/my photo#1.png").await.unwrap(),
            absolute.url("avatars/my photo#1.png").unwrap()
        );
        assert!(!absolute.url("a.png").unwrap().contains("/srv/app"));
    }
}
