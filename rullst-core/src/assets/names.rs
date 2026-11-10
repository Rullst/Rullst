//! Spelling of hashed static file names: `app.css` ↔ `app.<hash>.css`.

/// Hexadecimal characters of the content hash in a hashed name.
pub(crate) const HASH_LEN: usize = 16;

/// `/static/<path>`, unchanged.
pub(crate) fn plain_url(path: &str) -> String {
    format!("/static/{path}")
}

/// A path segment that needs no percent-encoding in a URL and is not hidden:
/// ASCII letters, digits, `-`, `_`, `.` and `~`, not starting with `.`.
pub(crate) fn plain_segment(segment: &str) -> bool {
    !segment.is_empty()
        && !segment.starts_with('.')
        && segment
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~'))
}

/// `(directory prefix with its trailing `/`, file name)` of `relative`.
fn split_file(relative: &str) -> (&str, &str) {
    match relative.rfind('/') {
        Some(index) => relative.split_at(index + 1),
        None => ("", relative),
    }
}

/// Whether `relative` (below `static/`) may get a hashed name: plain
/// segments, a file name with a stem and an extension, not a `.br`/`.zst`
/// sibling and not already fingerprinted.
pub(crate) fn hashable(relative: &str) -> bool {
    if !relative.split('/').all(plain_segment) {
        return false;
    }
    let (_, file) = split_file(relative);
    let Some((stem, extension)) = file.rsplit_once('.') else {
        return false;
    };
    !stem.is_empty()
        && !extension.is_empty()
        && !matches!(extension, "br" | "zst")
        && !crate::server::is_fingerprinted(file)
}

/// `css/app.css` with `hash` → `css/app.<hash>.css`; `relative` is
/// [`hashable`].
pub(crate) fn hashed_name(relative: &str, hash: &str) -> String {
    let (directory, file) = split_file(relative);
    match file.rsplit_once('.') {
        Some((stem, extension)) => format!("{directory}{stem}.{hash}.{extension}"),
        None => relative.to_owned(),
    }
}

/// Whether `hash` is spelled like a content hash: [`HASH_LEN`] lowercase
/// hexadecimal characters.
fn is_hash(hash: &str) -> bool {
    hash.len() == HASH_LEN
        && hash
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// `css/app.<hash>.css` → `("css/app.css", "<hash>")`, or `None` when
/// `requested` is not spelled like a hashed name.
pub(crate) fn split_hashed(requested: &str) -> Option<(String, &str)> {
    let (directory, file) = split_file(requested);
    let (rest, extension) = file.rsplit_once('.')?;
    let (stem, hash) = rest.rsplit_once('.')?;
    if stem.is_empty() || extension.is_empty() || !is_hash(hash) {
        return None;
    }
    Some((format!("{directory}{stem}.{extension}"), hash))
}

/// A cheap syntactic check before the manifest is consulted.
pub(crate) fn looks_hashed(requested: &str) -> bool {
    split_hashed(requested).is_some()
}
