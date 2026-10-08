//! Experimental S3-compatible object storage: AWS S3, Cloudflare R2 and MinIO.
//!
//! The adapter stores and serves the uploaded original; it never transcodes.
//! Browsers upload with a presigned `PUT` bound to a server-generated key, the
//! declared content type and exact length, and play with a presigned `GET`.
//! Both are issued only through the `sqlite` feature's `MediaService` grants and
//! expire within [`MAX_GRANT_SECONDS`]. Validated against the offline mock
//! only; provider interoperability is not yet validated.
mod adapter;
mod client;
mod config;
mod signing;

pub use adapter::MAX_GRANT_SECONDS;
pub use client::S3Storage;
pub use config::{S3Config, S3Credentials};
