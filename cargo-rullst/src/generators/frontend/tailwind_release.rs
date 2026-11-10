//! The pinned Tailwind CSS standalone CLI. Bumping Tailwind changes this file
//! only: the version and the SHA-256 of each release asset.
//!
//! Source of the checksums (v4.3.5, published 2026-11-06): the release's
//! `sha256sums.txt` asset
//! (`gh release download v4.3.5 --repo tailwindlabs/tailwindcss --pattern sha256sums.txt`),
//! identical to the asset digests GitHub reports
//! (`gh api repos/tailwindlabs/tailwindcss/releases/tags/v4.3.5 --jq '.assets[] | .name + " " + .digest'`),
//! and checked by downloading `tailwindcss-linux-x64` and hashing it
//! (`sha256sum`) on 2026-11-10. The unit tests check that every platform
//! has one well-formed entry.

use super::tailwind::Platform;

/// The Tailwind CSS release, without the tag's `v`.
pub(crate) const VERSION: &str = "4.3.5";

/// One downloadable standalone CLI.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Asset {
    pub platform: Platform,
    /// The release asset's file name.
    pub name: &'static str,
    /// Lowercase hexadecimal SHA-256 of the asset.
    pub sha256: &'static str,
}

/// Every asset of the release, one per [`Platform`].
pub(crate) const ASSETS: [Asset; 7] = [
    Asset {
        platform: Platform::LinuxX64,
        name: "tailwindcss-linux-x64",
        sha256: "e47695cee3f0d90623a91f3939b08bdee610eb5ad99340adff9d4145b284d6cc",
    },
    Asset {
        platform: Platform::LinuxArm64,
        name: "tailwindcss-linux-arm64",
        sha256: "ffcd0d21dbd7c605dc12743ff0c0b47debc963c86b399674084c1c8f2c96ecad",
    },
    Asset {
        platform: Platform::LinuxX64Musl,
        name: "tailwindcss-linux-x64-musl",
        sha256: "bb050edd13f15006d872a9bb1772b908618a8df6806d30bf095cba26a09e640b",
    },
    Asset {
        platform: Platform::LinuxArm64Musl,
        name: "tailwindcss-linux-arm64-musl",
        sha256: "564dad3028c029b0c69042c66a7a79873ed7289b73fcb7b37973fbf79cb9a32f",
    },
    Asset {
        platform: Platform::MacosX64,
        name: "tailwindcss-macos-x64",
        sha256: "a63702686a3ed048b3135d84a73be49ea9d47f880c9b20e54b3ef60934ad8cdf",
    },
    Asset {
        platform: Platform::MacosArm64,
        name: "tailwindcss-macos-arm64",
        sha256: "f3f4f5d1727e2b693b8f1c9a1492b5dd87008c86ea286bee1589cb458e6235fc",
    },
    Asset {
        platform: Platform::WindowsX64,
        name: "tailwindcss-windows-x64.exe",
        sha256: "2803583268c53f4713e89943e211081fddb186c0a4a7d07b5c342c6ae853ec95",
    },
];

/// Largest asset accepted while downloading (the v4.3.5 assets are 80 to
/// 112 MB).
pub(crate) const MAX_ASSET_BYTES: u64 = 256 * 1024 * 1024;

/// The download URL of `asset`.
pub(crate) fn download_url(asset: &Asset) -> String {
    format!(
        "https://github.com/tailwindlabs/tailwindcss/releases/download/v{VERSION}/{}",
        asset.name
    )
}
