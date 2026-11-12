#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::names::{hashable, hashed_name, plain_segment, split_hashed};
use super::*;
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

/// The 16-hex-character hash `url` writes for `content`.
fn hash_of(content: &[u8]) -> String {
    let digest = <sha2::Sha256 as sha2::Digest>::digest(content);
    digest[..8]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn write(root: &Path, relative: &str, content: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn manifest(root: &Path, follow_edits: bool) -> Manifest {
    Manifest::build(root, follow_edits, Limits::DEFAULT)
}

#[test]
fn hashable_names_are_plain_files_with_an_extension() {
    for name in [
        "app.css",
        "css/app.css",
        "dir.v2/app.min.js",
        "vendor/alpine-csp~1_x.min.js",
        "app.js.map",
    ] {
        assert!(hashable(name), "{name}");
    }
    for name in [
        "app",
        "dir.v2/app",
        ".env",
        "css/.hidden.css",
        ".css",
        "app.",
        "app.css.br",
        "app.css.zst",
        "app.3f9a2c1b.css",
        "my file.css",
        "café.css",
        "a//b.css",
        "../x.css",
        "./x.css",
        "a\\b.css",
        "a%20b.css",
        "",
    ] {
        assert!(!hashable(name), "{name}");
    }
    assert!(plain_segment("a-b_c.d~e9"));
    for segment in ["", ".git", "a b", "a+b", "a%b"] {
        assert!(!plain_segment(segment), "{segment}");
    }
}

#[test]
fn hashed_names_insert_the_hash_before_the_last_extension_and_split_back() {
    let hash = "0123456789abcdef";
    assert_eq!(hashed_name("app.css", hash), "app.0123456789abcdef.css");
    assert_eq!(
        hashed_name("dir.v2/app.min.js", hash),
        "dir.v2/app.min.0123456789abcdef.js"
    );
    assert_eq!(hashed_name("LICENSE", hash), "LICENSE");
    for base in ["app.css", "dir.v2/app.min.js", "css/site.css"] {
        let hashed = hashed_name(base, hash);
        assert_eq!(
            split_hashed(&hashed),
            Some((base.to_owned(), hash)),
            "{base}"
        );
        assert!(looks_hashed(&hashed));
    }
    for requested in [
        "app.css",
        "app.0123456789abcde.css",
        "app.0123456789abcdef0.css",
        "app.0123456789ABCDEF.css",
        "app.0123456789abcdeg.css",
        ".0123456789abcdef.css",
        "css/.0123456789abcdef.css",
        "app.0123456789abcdef.",
        "app.0123456789abcdef",
        "0123456789abcdef.css",
    ] {
        assert_eq!(split_hashed(requested), None, "{requested}");
        assert!(!looks_hashed(requested), "{requested}");
    }
}

#[test]
fn the_manifest_hashes_regular_files_and_skips_what_it_must_not_rename() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "app.css", "body{}");
    write(root.path(), "css/site.css", "main{}");
    write(root.path(), "app.css.br", "brotli");
    write(root.path(), "app.css.zst", "zstd");
    write(root.path(), ".hidden.css", "hidden");
    write(root.path(), ".git/config.css", "git");
    write(root.path(), "my dir/inner.css", "spaced");
    write(root.path(), "app.3f9a2c1b.css", "fingerprinted");
    write(root.path(), "LICENSE", "text");
    let manifest = manifest(root.path(), false);
    assert_eq!(manifest.len(), 2);
    assert_eq!(
        manifest.url("app.css"),
        format!("/static/app.{}.css", hash_of(b"body{}"))
    );
    assert_eq!(
        manifest.url("css/site.css"),
        format!("/static/css/site.{}.css", hash_of(b"main{}"))
    );
    for plain in [
        "app.css.br",
        ".hidden.css",
        "my dir/inner.css",
        "app.3f9a2c1b.css",
        "LICENSE",
        "missing.css",
        "css",
    ] {
        assert_eq!(manifest.url(plain), format!("/static/{plain}"), "{plain}");
    }
}

#[cfg(unix)]
#[test]
fn symbolic_links_keep_plain_urls_in_every_build() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    write(outside.path(), "theme.css", "outside{}");
    write(root.path(), "real/app.css", "body{}");
    std::os::unix::fs::symlink(
        outside.path().join("theme.css"),
        root.path().join("theme.css"),
    )
    .unwrap();
    std::os::unix::fs::symlink(root.path().join("real"), root.path().join("linked")).unwrap();
    for follow_edits in [false, true] {
        let manifest = manifest(root.path(), follow_edits);
        assert_eq!(manifest.len(), 1, "follow_edits={follow_edits}");
        assert_eq!(manifest.url("theme.css"), "/static/theme.css");
        assert_eq!(manifest.url("linked/app.css"), "/static/linked/app.css");
        assert!(
            manifest
                .url("real/app.css")
                .ends_with(&format!(".{}.css", hash_of(b"body{}")))
        );
    }
}

#[test]
fn the_start_up_walk_stays_within_its_limits() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "a.css", "a");
    write(root.path(), "b.css", "b");
    write(root.path(), "c.css", "c");
    assert_eq!(Limits::DEFAULT.file_bytes, 32 * 1024 * 1024);
    let two = Limits {
        files: 2,
        ..Limits::DEFAULT
    };
    let manifest = Manifest::build(root.path(), false, two);
    // Name order decides which files fit.
    assert_eq!(manifest.len(), 2);
    assert!(manifest.url("b.css").contains(&hash_of(b"b")));
    assert_eq!(manifest.url("c.css"), "/static/c.css");

    let root = tempfile::tempdir().unwrap();
    write(root.path(), "exact.css", "1234");
    write(root.path(), "large.css", "12345");
    let four_bytes = Limits {
        file_bytes: 4,
        ..Limits::DEFAULT
    };
    let manifest = Manifest::build(root.path(), false, four_bytes);
    assert!(manifest.url("exact.css").contains(&hash_of(b"1234")));
    assert_eq!(manifest.url("large.css"), "/static/large.css");

    let root = tempfile::tempdir().unwrap();
    write(root.path(), "top.css", "0");
    write(root.path(), "one/level.css", "1");
    write(root.path(), "one/two/levels.css", "2");
    let one_level = Limits {
        depth: 1,
        ..Limits::DEFAULT
    };
    let manifest = Manifest::build(root.path(), false, one_level);
    assert_eq!(manifest.len(), 2);
    assert!(manifest.url("one/level.css").contains(&hash_of(b"1")));
    assert_eq!(
        manifest.url("one/two/levels.css"),
        "/static/one/two/levels.css"
    );
    let flat = Limits {
        depth: 0,
        ..Limits::DEFAULT
    };
    assert_eq!(Manifest::build(root.path(), false, flat).len(), 1);
}

#[test]
fn debug_manifests_follow_edits_new_files_and_removals() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "app.css", "body{}");
    let manifest = manifest(root.path(), true);
    write(root.path(), "app.css", "body{color:red}");
    assert_eq!(
        manifest.url("app.css"),
        format!("/static/app.{}.css", hash_of(b"body{color:red}"))
    );
    // Same size, new modification time.
    write(root.path(), "app.css", "body{color:tan}");
    let later = SystemTime::now() + Duration::from_secs(60);
    fs::File::options()
        .write(true)
        .open(root.path().join("app.css"))
        .unwrap()
        .set_modified(later)
        .unwrap();
    assert_eq!(
        manifest.url("app.css"),
        format!("/static/app.{}.css", hash_of(b"body{color:tan}"))
    );
    write(root.path(), "js/new.js", "1");
    assert_eq!(
        manifest.url("js/new.js"),
        format!("/static/js/new.{}.js", hash_of(b"1"))
    );
    fs::remove_file(root.path().join("app.css")).unwrap();
    assert_eq!(manifest.url("app.css"), "/static/app.css");
    assert_eq!(manifest.len(), 1);
}

#[test]
fn release_manifests_keep_start_up_hashes_for_links() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "app.css", "body{}");
    let manifest = manifest(root.path(), false);
    write(root.path(), "app.css", "body{color:red}");
    write(root.path(), "new.css", "new");
    assert_eq!(
        manifest.url("app.css"),
        format!("/static/app.{}.css", hash_of(b"body{}"))
    );
    assert_eq!(manifest.url("new.css"), "/static/new.css");
}

#[test]
fn a_full_debug_manifest_records_no_new_file_but_still_rehashes_known_ones() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "app.css", "body{}");
    let one = Limits {
        files: 1,
        ..Limits::DEFAULT
    };
    let manifest = Manifest::build(root.path(), true, one);
    write(root.path(), "extra.css", "extra");
    assert_eq!(manifest.url("extra.css"), "/static/extra.css");
    write(root.path(), "app.css", "body{color:red}");
    assert!(
        manifest
            .url("app.css")
            .contains(&hash_of(b"body{color:red}"))
    );
}

#[test]
fn requests_resolve_current_outdated_and_unknown_hashes() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "css/app.css", "body{}");
    write(root.path(), "app.0123456789abcdef.css", "a real file");
    // Release: links keep the start-up hash, but requests check the file.
    let manifest = manifest(root.path(), false);
    let old = hash_of(b"body{}");
    let requested = format!("css/app.{old}.css");
    assert_eq!(
        manifest.resolve(&requested),
        Resolution::Current("css/app.css".to_owned())
    );
    assert_eq!(manifest.resolve("css/app.css"), Resolution::PassThrough);
    assert_eq!(
        manifest.resolve("app.0123456789abcdef.css"),
        Resolution::PassThrough
    );
    assert_eq!(
        manifest.resolve("css/missing.0123456789abcdef.css"),
        Resolution::PassThrough
    );

    write(root.path(), "css/app.css", "body{color:red}");
    let new = hash_of(b"body{color:red}");
    assert_eq!(
        manifest.resolve(&requested),
        Resolution::Outdated(format!("/static/css/app.{new}.css"))
    );
    assert_eq!(
        manifest.resolve(&format!("css/app.{new}.css")),
        Resolution::Current("css/app.css".to_owned())
    );
    fs::remove_file(root.path().join("css/app.css")).unwrap();
    assert_eq!(
        manifest.resolve(&format!("css/app.{new}.css")),
        Resolution::PassThrough
    );
}

#[test]
fn a_real_file_with_a_hash_like_name_is_served_as_it_is() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "app.css", "body{}");
    let manifest = manifest(root.path(), false);
    // Named like an outdated hash of `app.css`, but a file of its own.
    write(root.path(), "app.0123456789abcdef.css", "own");
    assert_eq!(
        manifest.resolve("app.0123456789abcdef.css"),
        Resolution::PassThrough
    );
    fs::remove_file(root.path().join("app.0123456789abcdef.css")).unwrap();
    assert!(matches!(
        manifest.resolve("app.0123456789abcdef.css"),
        Resolution::Outdated(_)
    ));
}

/// Unchanged size and modification time: the recorded hash is used without
/// reading the file again (an unreadable file keeps its hashed URL).
#[cfg(unix)]
#[test]
fn unchanged_metadata_reuses_the_recorded_hash() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "app.css", "body{}");
    let manifest = manifest(root.path(), true);
    let path = root.path().join("app.css");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).unwrap();
    let hashed = format!("/static/app.{}.css", hash_of(b"body{}"));
    let url = manifest.url("app.css");
    let resolved = manifest.resolve(&format!("app.{}.css", hash_of(b"body{}")));
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(url, hashed);
    assert_eq!(resolved, Resolution::Current("app.css".to_owned()));
}

#[test]
fn url_reads_the_active_manifest_and_is_plain_without_one() {
    let _guard = TEST_MANIFEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    deactivate();
    assert_eq!(url("app.css"), "/static/app.css");
    assert_eq!(url("//css/app.css"), "/static/css/app.css");
    assert_eq!(url(String::from("js/app.js")), "/static/js/app.js");

    let root = tempfile::tempdir().unwrap();
    write(root.path(), "app.css", "body{}");
    activate(Arc::new(manifest(root.path(), false)));
    let hashed = format!("/static/app.{}.css", hash_of(b"body{}"));
    assert_eq!(url("app.css"), hashed);
    assert_eq!(url("/app.css"), hashed);
    assert_eq!(
        crate::html! { <link rel="stylesheet" href={url("app.css")} /> },
        format!("<link rel=\"stylesheet\" href=\"{hashed}\" />")
    );
    deactivate();
    assert_eq!(url("app.css"), "/static/app.css");
}

#[test]
fn file_hashes_are_sha256_known_answers() {
    // The first eight bytes of the FIPS 180-2 appendix B.1 SHA-256 vector.
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "abc.css", "abc");
    let manifest = manifest(root.path(), false);
    assert_eq!(manifest.url("abc.css"), "/static/abc.ba7816bf8f01cfea.css");
}
