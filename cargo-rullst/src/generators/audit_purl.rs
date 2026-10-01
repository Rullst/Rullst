//! Package URLs for Cargo.lock entries in the CycloneDX SBOM.
//!
//! A plain `pkg:cargo/<name>@<version>` denotes the crates.io package, so it is
//! emitted only for crates.io sources. Other registries and git sources keep
//! their origin in a qualifier, and path or workspace packages get no purl.

/// `source` values Cargo records for crates.io.
const CRATES_IO_SOURCES: [&str; 2] = [
    "registry+https://github.com/rust-lang/crates.io-index",
    "sparse+https://index.crates.io/",
];

/// Where a Cargo.lock entry comes from, as far as the SBOM is concerned.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum CargoOrigin {
    /// Published on crates.io.
    CratesIo(String),
    /// Another registry or a git repository, with a qualified purl.
    Qualified(String),
    /// A path dependency, workspace member or the application itself.
    Local,
    /// A `source` this generator does not recognize; no purl is claimed.
    Unrecognized,
}

impl CargoOrigin {
    pub(crate) fn purl(&self) -> Option<&str> {
        match self {
            Self::CratesIo(purl) | Self::Qualified(purl) => Some(purl),
            Self::Local | Self::Unrecognized => None,
        }
    }
}

/// Classifies one lock entry from its `name`, `version` and optional `source`.
pub(crate) fn cargo_origin(name: &str, version: &str, source: Option<&str>) -> CargoOrigin {
    let base = format!("pkg:cargo/{}@{}", encode(name), encode(version));
    let Some(source) = source else {
        return CargoOrigin::Local;
    };
    if CRATES_IO_SOURCES.contains(&source) {
        return CargoOrigin::CratesIo(base);
    }
    if let Some(registry) = source
        .strip_prefix("registry+")
        .or_else(|| source.strip_prefix("sparse+"))
    {
        return CargoOrigin::Qualified(format!("{base}?repository_url={}", encode(registry)));
    }
    if source.starts_with("git+") {
        // Cargo records `git+<url>[?branch=..|tag=..|rev=..]#<commit>`; the purl
        // `vcs_url` form is `git+<url>@<commit>`.
        let (location, commit) = source
            .split_once('#')
            .map_or((source, None), |(location, commit)| {
                (location, Some(commit))
            });
        let repository = location
            .split_once('?')
            .map_or(location, |(repository, _)| repository);
        let vcs_url = commit.map_or_else(
            || repository.to_string(),
            |commit| format!("{repository}@{commit}"),
        );
        return CargoOrigin::Qualified(format!("{base}?vcs_url={}", encode(&vcs_url)));
    }
    CargoOrigin::Unrecognized
}

/// Percent-encodes everything except unreserved characters, `:` and `/`.
fn encode(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~' | b':' | b'/') {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_crates_io_sources_receive_the_plain_registry_purl() {
        assert_eq!(
            cargo_origin(
                "serde",
                "1.0.228",
                Some("registry+https://github.com/rust-lang/crates.io-index")
            ),
            CargoOrigin::CratesIo("pkg:cargo/serde@1.0.228".to_string())
        );
        assert_eq!(
            cargo_origin("serde", "1.0.228", Some("sparse+https://index.crates.io/")).purl(),
            Some("pkg:cargo/serde@1.0.228")
        );
        assert_eq!(cargo_origin("demo", "0.1.0", None), CargoOrigin::Local);
        assert_eq!(cargo_origin("demo", "0.1.0", None).purl(), None);
        assert_eq!(
            cargo_origin("demo", "0.1.0", Some("path+file:///work/demo")),
            CargoOrigin::Unrecognized
        );
    }

    #[test]
    fn git_and_alternative_registry_sources_keep_their_origin() {
        assert_eq!(
            cargo_origin(
                "openssl",
                "0.10.55",
                Some("git+https://github.com/example/rust-openssl?branch=fix#0123abcd")
            )
            .purl(),
            Some(
                "pkg:cargo/openssl@0.10.55?vcs_url=git%2Bhttps://github.com/example/rust-openssl%400123abcd"
            )
        );
        assert_eq!(
            cargo_origin(
                "internal",
                "2.0.0+build.5",
                Some("sparse+https://cargo.example.com/index/")
            )
            .purl(),
            Some(
                "pkg:cargo/internal@2.0.0%2Bbuild.5?repository_url=https://cargo.example.com/index/"
            )
        );
    }
}
