#![cfg(all(feature = "storage-s3", feature = "security"))]
#![allow(clippy::unwrap_used, clippy::expect_used)]

include!("../../.github/fixtures/storage-facade.rs");
