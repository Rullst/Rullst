//! Explicit private video contracts. Provider and persistence are opt-in.
//!
//! Private course videos: creation, metadata, resumable upload grants,
//! processing refresh, explicit publication, current-entitlement playback,
//! withdrawal, deletion and recovery of interrupted operations.
//!
//! **Maturity: Experimental**
//! ([tiers](https://rullst.github.io/Rullst/book/maturity.html)). The API may
//! change between 13.x releases. Interoperability with real Bunny Stream
//! accounts, transcoding and CDN delivery is unvalidated, and the `s3` adapter
//! is validated against the offline mock only.
//!
//! No feature is enabled by default; the default build holds the validated
//! identities, the [`Authorization`] and [`VideoProvider`] contracts and
//! [`MediaError`], with no HTTP or database. Enable `bunny` for the Bunny Stream
//! adapter and browser upload module, `s3` for S3-compatible object storage
//! (AWS S3, Cloudflare R2, MinIO) without transcoding, and `sqlite` for the
//! shared-local store and `sqlite::MediaService`; a complete service uses
//! `bunny` or `s3` together with `sqlite`. See the
//! [private video guide](https://rullst.github.io/Rullst/book/managed-video.html)
//! and the [README](https://github.com/Rullst/Rullst/blob/main/rullst-media/README.md).
#![forbid(unsafe_code)]

mod contracts;
mod error;
mod provider;

pub use contracts::*;
pub use error::MediaError;
pub use provider::*;

#[cfg(feature = "bunny")]
pub mod bunny;
/// Dependency-free browser TUS module. Serve as an external JavaScript module
/// under the host's CSP; never interpolate this source or credentials into HTML.
#[cfg(feature = "bunny")]
pub const BUNNY_UPLOAD_MODULE: &str = include_str!("../web/bunny-upload.mjs");
#[cfg(feature = "s3")]
pub mod s3;
#[cfg(feature = "sqlite")]
pub mod sqlite;
