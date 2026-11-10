//! The pinned Tailwind CSS standalone CLI. Bumping Tailwind changes this file
//! only: the version and the SHA-256 of each release asset.
//!
//! Cooldown rule: pin the newest v4 release that was published at least 14
//! days before the pin is made, never a release younger than that, so a
//! compromised or broken release has time to be noticed and pulled. On
//! 2026-11-10 that is v4.3.3 (published 2026-07-16); v4.3.4 (2026-11-02) and
//! v4.3.5 (2026-11-06) were younger. The same rule applies to every pinned
//! third-party download (`alpine_release.rs`).
//!
//! Source of the checksums: the release's `sha256sums.txt` asset
//! (`gh release download v4.3.3 --repo tailwindlabs/tailwindcss --pattern sha256sums.txt`),
//! identical to the asset digests GitHub reports
//! (`gh api repos/tailwindlabs/tailwindcss/releases/tags/v4.3.3 --jq '.assets[] | .name + " " + .digest'`),
//! and checked by downloading `tailwindcss-linux-x64` and hashing it
//! (`sha256sum`) on 2026-11-10. The unit tests check that every platform
//! has one well-formed entry.

use super::tailwind::Platform;

/// The Tailwind CSS release, without the tag's `v`.
pub(crate) const VERSION: &str = "4.3.3";

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
        sha256: "dc61b3ac6b8c9ca874c0cc4c57b2409791a64c5540404ca5f5367360babc313a",
    },
    Asset {
        platform: Platform::LinuxArm64,
        name: "tailwindcss-linux-arm64",
        sha256: "55fd0b241214eff3de1e8ee4f22796662f2d2e7a49bcfca7477cfd0bac398195",
    },
    Asset {
        platform: Platform::LinuxX64Musl,
        name: "tailwindcss-linux-x64-musl",
        sha256: "a04d34ceacc8f52cbe8920ad846cdeb61d3d0021dba32db0d1f77c9d9fad7a6c",
    },
    Asset {
        platform: Platform::LinuxArm64Musl,
        name: "tailwindcss-linux-arm64-musl",
        sha256: "71ea4be79c9de9827545682df3e040053fb535d37c71ed2cfdedf9385a0868e0",
    },
    Asset {
        platform: Platform::MacosX64,
        name: "tailwindcss-macos-x64",
        sha256: "7922e0953f2110c05976e3bf58f14e643d90427575e766b7d433f5f80cbee7e1",
    },
    Asset {
        platform: Platform::MacosArm64,
        name: "tailwindcss-macos-arm64",
        sha256: "cdf646702987a743464dff4d9c60fd4480d1c1e73dd819a9a67f1078815dce9d",
    },
    Asset {
        platform: Platform::WindowsX64,
        name: "tailwindcss-windows-x64.exe",
        sha256: "e0e260ce048014e9268f6237ff18f8ccf02cef521cbd0ae04e82c2cdf7aa3955",
    },
];

/// Largest asset accepted while downloading (the v4.3.3 assets are 80 to
/// 113 MB).
pub(crate) const MAX_ASSET_BYTES: u64 = 256 * 1024 * 1024;

/// The download URL of `asset`.
pub(crate) fn download_url(asset: &Asset) -> String {
    format!(
        "https://github.com/tailwindlabs/tailwindcss/releases/download/v{VERSION}/{}",
        asset.name
    )
}
