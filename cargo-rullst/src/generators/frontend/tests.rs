#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::alpine::{VendorError, integrity, is_pinned, vendored_with};
use super::alpine_release::{self, BANNER, ENTRY, INTEGRITY, MAX_ENTRY_BYTES};
use super::fetch::{FetchError, Fixture};
use super::tailwind::{self, Platform, ToolError};
use super::tailwind_release::{self, ASSETS, Asset, VERSION};
use super::*;
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::path::Path;

fn sha256(bytes: &[u8]) -> String {
    use sha2::Digest;
    hex::encode(sha2::Sha256::digest(bytes))
}

/// `cargo-rullst` release targets (.github/cli-artifact-targets.json) and
/// the other hosts `cargo install` supports, as (os, arch, musl).
const CLI_HOSTS: [(&str, &str, bool, Platform); 8] = [
    ("linux", "x86_64", false, Platform::LinuxX64),
    ("windows", "x86_64", false, Platform::WindowsX64),
    ("macos", "aarch64", false, Platform::MacosArm64),
    ("macos", "x86_64", false, Platform::MacosX64),
    ("linux", "aarch64", false, Platform::LinuxArm64),
    ("linux", "x86_64", true, Platform::LinuxX64Musl),
    ("linux", "aarch64", true, Platform::LinuxArm64Musl),
    ("windows", "aarch64", false, Platform::WindowsX64),
];

#[test]
fn the_pinned_table_has_one_well_formed_asset_per_platform() {
    assert!(
        semver::Version::parse(VERSION).unwrap().major == 4,
        "{VERSION}"
    );
    let mut names = BTreeSet::new();
    for platform in Platform::ALL {
        let matching: Vec<&Asset> = ASSETS
            .iter()
            .filter(|asset| asset.platform == platform)
            .collect();
        assert_eq!(matching.len(), 1, "{platform:?}");
        let asset = matching[0];
        assert_eq!(platform.asset(), Some(asset));
        assert_eq!(asset.sha256.len(), 64, "{platform:?}");
        assert!(
            asset
                .sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        );
        assert!(asset.name.starts_with("tailwindcss-"), "{}", asset.name);
        assert_eq!(
            asset.name.ends_with(".exe"),
            platform == Platform::WindowsX64
        );
        assert!(names.insert(asset.name));
        assert_eq!(
            tailwind_release::download_url(asset),
            format!(
                "https://github.com/tailwindlabs/tailwindcss/releases/download/v{VERSION}/{}",
                asset.name
            )
        );
    }
    assert_eq!(names.len(), ASSETS.len());
}

#[test]
fn every_host_the_cli_supports_has_a_pinned_asset() {
    for (os, arch, musl, expected) in CLI_HOSTS {
        let platform = Platform::for_target(os, arch, musl);
        assert_eq!(platform, Some(expected), "{os}/{arch} musl={musl}");
        assert!(expected.asset().is_some());
    }
    for (os, arch) in [
        ("freebsd", "x86_64"),
        ("linux", "riscv64"),
        ("linux", "x86"),
        ("macos", "powerpc"),
    ] {
        assert_eq!(Platform::for_target(os, arch, false), None, "{os}/{arch}");
    }
    // Every CI host (Linux, macOS, Windows on x64 or arm64) has one.
    assert!(Platform::host().is_some());
    assert!(tailwind::host_asset().is_ok());
}

fn env<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<OsString> + 'a {
    move |name| {
        pairs
            .iter()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| OsString::from(value))
    }
}

#[test]
fn the_cache_lives_in_the_user_cache_under_a_versioned_directory() {
    let (cache, home) = if cfg!(windows) {
        ("C:\\cache", "C:\\home")
    } else {
        ("/cache", "/home/me")
    };
    let both = [("XDG_CACHE_HOME", cache), ("HOME", home)];
    assert_eq!(
        tailwind::cache_base(false, env(&both)),
        Some(Path::new(cache).to_path_buf())
    );
    let relative = [("XDG_CACHE_HOME", "cache"), ("HOME", home)];
    assert_eq!(
        tailwind::cache_base(false, env(&relative)),
        Some(Path::new(home).join(".cache"))
    );
    assert_eq!(tailwind::cache_base(false, env(&[("HOME", "home")])), None);
    assert_eq!(tailwind::cache_base(false, env(&[])), None);
    let windows = [("LOCALAPPDATA", cache), ("XDG_CACHE_HOME", home)];
    assert_eq!(
        tailwind::cache_base(true, env(&windows)),
        Some(Path::new(cache).to_path_buf())
    );
    assert_eq!(tailwind::cache_base(true, env(&[("HOME", home)])), None);

    let asset = Platform::LinuxX64.asset().unwrap();
    assert_eq!(
        tailwind::binary_path(Path::new(cache), asset),
        Path::new(cache)
            .join("rullst")
            .join("tailwindcss")
            .join(format!("v{VERSION}"))
            .join("tailwindcss-linux-x64")
    );
}

fn fixture_asset(sha256: &'static str) -> Asset {
    Asset {
        platform: Platform::LinuxX64,
        name: "tailwindcss-fixture",
        sha256,
    }
}

const FIXTURE_CLI: &[u8] = b"#!/bin/sh\nexit 0\n";
const FIXTURE_SHA256: &str = "306c6ca7407560340797866e077e053627ad409277d1b9da58106fce4cf717cb";

#[test]
fn the_fixture_checksum_is_the_fixture_cli() {
    assert_eq!(sha256(FIXTURE_CLI), FIXTURE_SHA256);
}

#[test]
fn a_download_is_verified_before_it_is_kept_and_reused_without_network() {
    let base = tempfile::tempdir().unwrap();
    let asset = fixture_asset(FIXTURE_SHA256);
    let fetcher = Fixture::serving(FIXTURE_CLI);
    let first = tailwind::ensure(&fetcher, base.path(), &asset).unwrap();
    assert!(first.downloaded);
    assert_eq!(first.path, tailwind::binary_path(base.path(), &asset));
    assert_eq!(std::fs::read(&first.path).unwrap(), FIXTURE_CLI);
    assert_eq!(
        fetcher.urls.borrow().as_slice(),
        [tailwind_release::download_url(&asset)]
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&first.path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o700);
        let directory = std::fs::metadata(first.path.parent().unwrap()).unwrap();
        assert_eq!(directory.permissions().mode() & 0o777, 0o700);
    }
    let second = tailwind::ensure(&fetcher, base.path(), &asset).unwrap();
    assert!(!second.downloaded);
    assert_eq!(second.path, first.path);
    assert_eq!(fetcher.calls.get(), 1);

    // A cached file that changed is refused, kept for inspection and not
    // downloaded again.
    std::fs::write(&first.path, b"#!/bin/sh\necho tampered\n").unwrap();
    match tailwind::ensure(&fetcher, base.path(), &asset) {
        Err(ToolError::ChecksumMismatch {
            path,
            expected,
            actual,
        }) => {
            assert_eq!(path, first.path);
            assert_eq!(expected, FIXTURE_SHA256);
            assert_eq!(actual, sha256(b"#!/bin/sh\necho tampered\n"));
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(fetcher.calls.get(), 1);
    assert!(first.path.is_file());
    let message = tailwind::verify(&first.path, &asset)
        .unwrap_err()
        .to_string();
    assert!(message.contains("it was not run"), "{message}");
    assert!(
        message.contains(&format!("tailwindcss v{VERSION}")),
        "{message}"
    );
}

#[test]
fn a_download_with_other_bytes_or_a_failed_download_leaves_nothing() {
    let base = tempfile::tempdir().unwrap();
    let asset = fixture_asset(FIXTURE_SHA256);
    let wrong = Fixture::serving(b"#!/bin/sh\necho not the pinned cli\n");
    match tailwind::ensure(&wrong, base.path(), &asset) {
        Err(ToolError::ChecksumMismatch { expected, .. }) => assert_eq!(expected, FIXTURE_SHA256),
        other => panic!("{other:?}"),
    }
    let path = tailwind::binary_path(base.path(), &asset);
    assert!(!path.exists());
    assert_eq!(
        std::fs::read_dir(path.parent().unwrap()).unwrap().count(),
        0
    );

    let offline = Fixture::failing(FetchError::Status {
        url: "https://example.invalid".into(),
        status: 404,
    });
    match tailwind::ensure(&offline, base.path(), &asset) {
        Err(ToolError::Download(FetchError::Status { status: 404, .. })) => {}
        other => panic!("{other:?}"),
    }
    assert!(!path.exists());
    assert_eq!(
        std::fs::read_dir(path.parent().unwrap()).unwrap().count(),
        0
    );
}

#[test]
fn build_and_watch_commands_name_the_input_and_output() {
    let root = Path::new("project");
    let binary = Path::new("tailwindcss");
    let args = |command: &std::process::Command| {
        command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
    };
    let build = tailwind::build_command(binary, root, false);
    assert_eq!(
        args(&build),
        ["--input", TAILWIND_INPUT, "--output", TAILWIND_OUTPUT]
    );
    assert_eq!(build.get_current_dir(), Some(root));
    assert_eq!(build.get_program(), "tailwindcss");
    let minified = tailwind::build_command(binary, root, true);
    assert_eq!(args(&minified).last().map(String::as_str), Some("--minify"));
    let watch = tailwind::watch_command(binary, root);
    assert_eq!(
        args(&watch),
        [
            "--input",
            TAILWIND_INPUT,
            "--output",
            TAILWIND_OUTPUT,
            "--watch"
        ]
    );
}

#[cfg(unix)]
#[test]
fn run_build_reports_the_cli_exit_status() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let ok = root.path().join("ok.sh");
    std::fs::write(
        &ok,
        "#!/bin/sh\nmkdir -p static && echo built > static/app.css\n",
    )
    .unwrap();
    let failing = root.path().join("fail.sh");
    std::fs::write(&failing, "#!/bin/sh\nexit 3\n").unwrap();
    for script in [&ok, &failing] {
        std::fs::set_permissions(script, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    tailwind::run_build(&ok, root.path(), false).unwrap();
    assert_eq!(
        std::fs::read_to_string(root.path().join(TAILWIND_OUTPUT)).unwrap(),
        "built\n"
    );
    match tailwind::run_build(&failing, root.path(), true) {
        Err(ToolError::Failed(status)) => assert_eq!(status.code(), Some(3)),
        other => panic!("{other:?}"),
    }
    assert!(matches!(
        tailwind::run_build(&root.path().join("missing"), root.path(), false),
        Err(ToolError::Io { action: "run", .. })
    ));
}

#[test]
fn tailwind_use_is_detected_from_the_input_or_the_projects_own_setup() {
    for css in [
        "@import \"tailwindcss\";",
        "  @import 'tailwindcss' source(none);",
        "/* x */\n@import   \"tailwindcss\" prefix(tw);",
    ] {
        assert!(imports_tailwind(css), "{css}");
    }
    for css in [
        "@import \"tailwind.css\";",
        "/* @import \"tailwindcss\" */",
        "@import url(tailwindcss);",
    ] {
        assert!(!imports_tailwind(css), "{css}");
    }

    let root = tempfile::tempdir().unwrap();
    assert!(!has_managed_tailwind(root.path()));
    assert!(!uses_tailwind(root.path()));
    std::fs::create_dir_all(root.path().join("assets/css")).unwrap();
    std::fs::write(root.path().join(TAILWIND_INPUT), "body{}").unwrap();
    assert!(!uses_tailwind(root.path()));
    std::fs::write(
        root.path().join(TAILWIND_INPUT),
        "@import \"tailwindcss\";\n",
    )
    .unwrap();
    assert!(has_managed_tailwind(root.path()));
    assert!(uses_tailwind(root.path()));

    for (file, text, uses) in [
        ("tailwind.config.js", "module.exports = {}", true),
        ("tailwind.config.ts", "export default {}", true),
        (
            "package.json",
            "{\"devDependencies\":{\"tailwindcss\":\"^3.4.0\"}}",
            true,
        ),
        (
            "package.json",
            "{\"devDependencies\":{\"@tailwindcss/cli\":\"^4\"}}",
            true,
        ),
        (
            "package.json",
            "{\"dependencies\":{\"htmx.org\":\"^2\"}}",
            false,
        ),
    ] {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join(file), text).unwrap();
        assert_eq!(uses_tailwind(root.path()), uses, "{file}: {text}");
        assert!(!has_managed_tailwind(root.path()));
    }
}

/// A gzip tarball with `entries` (path, bytes); a `None` body is a directory.
fn tarball(entries: &[(&str, Option<&[u8]>)]) -> Vec<u8> {
    let encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    let mut builder = tar::Builder::new(encoder);
    for (path, body) in entries {
        let mut header = tar::Header::new_gnu();
        match body {
            Some(body) => {
                header.set_size(body.len() as u64);
                header.set_entry_type(tar::EntryType::Regular);
                header.set_mode(0o600);
                header.set_cksum();
                builder.append_data(&mut header, path, *body).unwrap();
            }
            None => {
                header.set_size(0);
                header.set_entry_type(tar::EntryType::Directory);
                header.set_mode(0o700);
                header.set_cksum();
                builder
                    .append_data(&mut header, path, std::io::empty())
                    .unwrap();
            }
        }
    }
    builder.into_inner().unwrap().finish().unwrap()
}

#[test]
fn the_pinned_alpine_build_is_described_consistently() {
    assert!(alpine_release::TARBALL.ends_with(&format!("/csp-{}.tgz", alpine_release::VERSION)));
    assert!(BANNER.contains(&format!("@alpinejs/csp {} ", alpine_release::VERSION)));
    assert!(BANNER.starts_with("/*!") && BANNER.ends_with("*/") && !BANNER.contains('\n'));
    assert!(INTEGRITY.starts_with("sha512-") && INTEGRITY.len() == 7 + 88);
    assert_eq!(
        integrity(b""),
        "sha512-z4PhNX7vuL3xVChQ1m2AB9Yg5AULVxXcg/SpIdNs6c5H0NE8XYXysP+DGNKHfuwvY7kxvUdBeoGlODJ6+SfaPg=="
    );
    assert!(is_pinned(&format!("{BANNER}\ncode")));
    assert!(!is_pinned("/*! @alpinejs/csp 3.0.0 */\ncode"));
    assert!(!is_pinned(""));
}

#[test]
fn the_browser_build_is_read_only_from_a_verified_tarball() {
    let tgz = tarball(&[
        ("package/package.json", Some(b"{}")),
        ("package/dist", None),
        ("package/dist/cdn.js", Some(b"unminified")),
        (ENTRY, Some(b"window.Alpine=1;")),
    ]);
    let expected = integrity(&tgz);
    assert_eq!(
        vendored_with(&tgz, &expected).unwrap(),
        format!("{BANNER}\nwindow.Alpine=1;")
    );
    match vendored_with(&tgz, INTEGRITY) {
        Err(VendorError::IntegrityMismatch { expected, actual }) => {
            assert_eq!(expected, INTEGRITY);
            assert_eq!(actual, integrity(&tgz));
        }
        other => panic!("{other:?}"),
    }
    let without = tarball(&[("package/dist/cdn.js", Some(b"x"))]);
    assert!(matches!(
        vendored_with(&without, &integrity(&without)),
        Err(VendorError::MissingEntry)
    ));
    let large = vec![b'a'; MAX_ENTRY_BYTES as usize + 1];
    let oversized = tarball(&[(ENTRY, Some(&large))]);
    assert!(matches!(
        vendored_with(&oversized, &integrity(&oversized)),
        Err(VendorError::MissingEntry)
    ));
    let exact = vec![b'a'; MAX_ENTRY_BYTES as usize];
    let at_limit = tarball(&[(ENTRY, Some(&exact))]);
    assert!(vendored_with(&at_limit, &integrity(&at_limit)).is_ok());
    let garbage = b"not a gzip stream".to_vec();
    assert!(matches!(
        vendored_with(&garbage, &integrity(&garbage)),
        Err(VendorError::Archive(_))
    ));

    // The pinned download goes to the pinned URL and is refused when the
    // fixture is not the pinned tarball.
    let fetcher = Fixture::serving(&tgz);
    assert!(matches!(
        alpine::download_with(&fetcher, INTEGRITY),
        Err(VendorError::IntegrityMismatch { .. })
    ));
    assert_eq!(fetcher.urls.borrow().as_slice(), [alpine_release::TARBALL]);
}

#[test]
fn the_dockerfile_stages_pin_both_architectures_and_edit_only_generated_files() {
    let stages = docker::stages();
    for platform in [Platform::LinuxX64, Platform::LinuxArm64] {
        let asset = platform.asset().unwrap();
        assert!(stages.contains(&format!("--checksum=sha256:{}", asset.sha256)));
        assert!(stages.contains(&tailwind_release::download_url(asset)));
    }
    assert!(stages.contains("FROM scratch AS tailwind-cli-amd64\n"));
    assert!(stages.contains("FROM scratch AS tailwind-cli-arm64\n"));
    assert!(stages.contains("FROM tailwind-cli-${BUILDARCH} AS tailwind-cli\n"));
    assert!(
        stages.contains("FROM --platform=$BUILDPLATFORM rust:1.99.1-slim-bookworm AS tailwind\n")
    );
    assert!(
        stages.contains(
            "RUN tailwindcss --input assets/css/app.css --output /css/app.css --minify\n"
        )
    );
    assert!(stages.contains(docker::STAGES_MARKER));

    let generated = "FROM rust AS builder\nRUN build\n\n# Debian 12 glibc runtime (x)\nFROM gcr.io/distroless/cc-debian12:nonroot\nCOPY --chown=10001:10001 static /app/static\nCMD [\"/app/demo\"]\n";
    let edited = docker::add_to(generated).unwrap();
    let runtime = edited.find("# Debian 12 glibc runtime").unwrap();
    assert!(edited.find(docker::STAGES_MARKER).unwrap() < runtime);
    assert!(edited.contains(&format!(
        "COPY --chown=10001:10001 static /app/static\n{}CMD",
        docker::COPY_LINE
    )));
    assert!(edited.starts_with("FROM rust AS builder\nRUN build\n\n# Tailwind CSS"));
    assert_eq!(docker::add_to(&edited), None);
    let without_comment = generated.replace("# Debian 12 glibc runtime (x)\n", "");
    let edited = docker::add_to(&without_comment).unwrap();
    assert!(edited.contains(&format!("{}FROM gcr.io/distroless", "--minify\n\n")));
    assert_eq!(docker::add_to("FROM alpine\nCOPY . /app\n"), None);
    let no_static = generated.replace("COPY --chown=10001:10001 static /app/static\n", "");
    assert_eq!(docker::add_to(&no_static), None);
    // A static copy before the runtime stage is not the generated layout.
    let early =
        "COPY --chown=10001:10001 static /app/static\nFROM gcr.io/distroless/cc-debian12:nonroot\n";
    assert_eq!(docker::add_to(early), None);
}

#[test]
fn generated_dockerfiles_build_the_stylesheet_only_for_managed_tailwind() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("static")).unwrap();
    crate::generators::project::generate_docker_files(root.path(), "shop", None, None).unwrap();
    let plain = std::fs::read_to_string(root.path().join("Dockerfile")).unwrap();
    assert!(!plain.contains(docker::STAGES_MARKER));

    std::fs::create_dir_all(root.path().join("assets/css")).unwrap();
    std::fs::write(
        root.path().join(TAILWIND_INPUT),
        "@import \"tailwindcss\";\n",
    )
    .unwrap();
    crate::generators::project::generate_docker_files(root.path(), "shop", None, None).unwrap();
    let styled = std::fs::read_to_string(root.path().join("Dockerfile")).unwrap();
    assert_eq!(Some(styled.clone()), docker::add_to(&plain));
    // The stylesheet joins static/ in the stage that pre-compresses it
    // (tested with that stage in project/docker_static.rs).
    assert!(styled.contains(docker::STATIC_STAGE_COPY_LINE));
    assert!(!styled.contains(docker::COPY_LINE));
}
