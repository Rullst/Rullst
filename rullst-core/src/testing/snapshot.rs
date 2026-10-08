//! File-backed snapshot assertions for rendered HTML (v13).
//!
//! [`assert_html_snapshot!`](crate::assert_html_snapshot) compares the
//! [normalised](super::normalize_html) HTML with
//! `<crate>/tests/snapshots/<name>.html`. This is test-only code: it panics
//! like `assert_eq!` and is never called on a production path.

use std::io::ErrorKind;
use std::path::{Path, PathBuf};

/// The environment variable that writes missing or changed snapshots.
pub const UPDATE_SNAPSHOTS_ENV: &str = "RULLST_UPDATE_SNAPSHOTS";
/// At most this many changed lines are printed for one mismatch.
const DIFF_LINE_LIMIT: usize = 40;
/// Largest line-diff table (expected lines × actual lines) computed.
const DIFF_CELL_LIMIT: usize = 4_000_000;

/// Which volatile values the HTML snapshot normaliser replaces with
/// documented placeholders. Nothing is masked by default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SnapshotOptions {
    nonce: bool,
    csrf_token: bool,
}

impl SnapshotOptions {
    /// Masks nothing; whitespace is still normalised.
    pub const fn new() -> Self {
        Self {
            nonce: false,
            csrf_token: false,
        }
    }

    /// Replaces CSP nonces (`nonce="…"` attributes and `'nonce-…'` sources)
    /// with `{NONCE}`.
    pub const fn mask_nonce(mut self) -> Self {
        self.nonce = true;
        self
    }

    /// Replaces CSRF tokens (the `value` of a `name="_token"` field, the
    /// `content` of `<meta name="csrf-token">` and `X-CSRF-Token` values in
    /// attributes such as `hx-headers`) with `{CSRF_TOKEN}`.
    pub const fn mask_csrf_token(mut self) -> Self {
        self.csrf_token = true;
        self
    }

    /// Whether CSP nonces are masked.
    pub const fn masks_nonce(self) -> bool {
        self.nonce
    }

    /// Whether CSRF tokens are masked.
    pub const fn masks_csrf_token(self) -> bool {
        self.csrf_token
    }
}

/// Asserts that rendered HTML matches `tests/snapshots/<name>.html` in the
/// crate that calls it.
///
/// Both sides go through [`normalize_html`](crate::testing::normalize_html),
/// so insignificant whitespace between tags does not matter. A missing
/// snapshot fails with the path to create; a mismatch fails with a compact
/// line diff. Run the tests with `RULLST_UPDATE_SNAPSHOTS=1` to write missing
/// snapshots and overwrite changed ones, then review and commit them.
///
/// `name` may contain `/` to group snapshots in folders; each segment uses
/// only ASCII letters, digits, `_`, `-` and `.`. The optional third argument
/// is a [`SnapshotOptions`](crate::testing::SnapshotOptions).
///
/// ```no_run
/// use rullst_core::testing::{SnapshotOptions, assert_html_snapshot};
///
/// let page = r#"<form method="post"><input type="hidden" name="_token" value="abc123"></form>"#;
/// assert_html_snapshot!("signup/form", page, SnapshotOptions::new().mask_csrf_token());
/// ```
#[macro_export]
macro_rules! assert_html_snapshot {
    ($name:expr, $html:expr $(,)?) => {
        $crate::testing::__assert_html_snapshot(
            ::core::env!("CARGO_MANIFEST_DIR"),
            &$name,
            &$html,
            $crate::testing::SnapshotOptions::new(),
        )
    };
    ($name:expr, $html:expr, $options:expr $(,)?) => {
        $crate::testing::__assert_html_snapshot(
            ::core::env!("CARGO_MANIFEST_DIR"),
            &$name,
            &$html,
            $options,
        )
    };
}

/// The implementation behind [`assert_html_snapshot!`](crate::assert_html_snapshot).
#[doc(hidden)]
#[track_caller]
pub fn __assert_html_snapshot(
    manifest_dir: &str,
    name: &str,
    html: &str,
    options: SnapshotOptions,
) {
    let path = match snapshot_path(Path::new(manifest_dir), name) {
        Ok(path) => path,
        Err(message) => panic!("{message}"),
    };
    let actual = super::normalize_html(html, options);
    let update = update_requested(std::env::var(UPDATE_SNAPSHOTS_ENV).ok().as_deref());
    match check_snapshot(&path, name, &actual, options, update) {
        Ok(Outcome::Matched) => {}
        Ok(Outcome::Written) => eprintln!("wrote HTML snapshot {}", path.display()),
        Err(message) => panic!("{message}"),
    }
}

/// What a passing snapshot check did.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    Matched,
    Written,
}

/// `1`, `true` or `yes` (any letter case) asks for snapshots to be written.
pub(crate) fn update_requested(value: Option<&str>) -> bool {
    value.is_some_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes"
        )
    })
}

/// `<manifest_dir>/tests/snapshots/<name>.html`, refusing names that could
/// leave that folder.
pub(crate) fn snapshot_path(manifest_dir: &Path, name: &str) -> Result<PathBuf, String> {
    let valid = !name.is_empty()
        && name.split('/').all(|segment| {
            !segment.is_empty()
                && segment != "."
                && segment != ".."
                && segment
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
        });
    if !valid {
        return Err(format!(
            "invalid HTML snapshot name {name:?}: use ASCII letters, digits, '_', '-' and '.', \
             with '/' between folder segments"
        ));
    }
    let mut path = manifest_dir.join("tests").join("snapshots");
    path.push(format!("{name}.html"));
    Ok(path)
}

/// Compares `actual` (already normalised) with the stored snapshot, writing
/// it when `update` is set.
pub(crate) fn check_snapshot(
    path: &Path,
    name: &str,
    actual: &str,
    options: SnapshotOptions,
    update: bool,
) -> Result<Outcome, String> {
    let stored = match std::fs::read_to_string(path) {
        Ok(stored) => Some(super::normalize_html(&stored, options)),
        Err(error) if error.kind() == ErrorKind::NotFound => None,
        Err(error) => {
            return Err(format!(
                "could not read HTML snapshot {}: {error}",
                path.display()
            ));
        }
    };
    if stored.as_deref() == Some(actual) {
        return Ok(Outcome::Matched);
    }
    if update {
        write_snapshot(path, actual)?;
        return Ok(Outcome::Written);
    }
    Err(match stored {
        None => format!(
            "HTML snapshot `{name}` is missing: {} does not exist.\n\
             Run the test with {UPDATE_SNAPSHOTS_ENV}=1 to create it, then review and commit it.",
            path.display()
        ),
        Some(expected) => format!(
            "HTML snapshot `{name}` does not match {}\n{}\
             If the change is intended, rerun the test with {UPDATE_SNAPSHOTS_ENV}=1 and review the file.",
            path.display(),
            line_diff(&expected, actual)
        ),
    })
}

fn write_snapshot(path: &Path, contents: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "could not create snapshot folder {}: {error}",
                parent.display()
            )
        })?;
    }
    std::fs::write(path, contents)
        .map_err(|error| format!("could not write HTML snapshot {}: {error}", path.display()))
}

/// Changed lines only, numbered, `-` for the snapshot and `+` for the
/// rendered HTML, capped at [`DIFF_LINE_LIMIT`] lines.
pub(crate) fn line_diff(expected: &str, actual: &str) -> String {
    let old: Vec<&str> = expected.lines().collect();
    let new: Vec<&str> = actual.lines().collect();
    let mut changes = Vec::new();
    if (old.len() + 1).saturating_mul(new.len() + 1) <= DIFF_CELL_LIMIT {
        let width = new.len() + 1;
        let mut table = vec![0u32; (old.len() + 1) * width];
        for i in (0..old.len()).rev() {
            for j in (0..new.len()).rev() {
                table[i * width + j] = if old[i] == new[j] {
                    table[(i + 1) * width + j + 1] + 1
                } else {
                    table[(i + 1) * width + j].max(table[i * width + j + 1])
                };
            }
        }
        let (mut i, mut j) = (0, 0);
        while i < old.len() || j < new.len() {
            if i < old.len() && j < new.len() && old[i] == new[j] {
                i += 1;
                j += 1;
            } else if i < old.len()
                && (j == new.len() || table[(i + 1) * width + j] >= table[i * width + j + 1])
            {
                changes.push(format!("-{:>4} | {}", i + 1, old[i]));
                i += 1;
            } else {
                changes.push(format!("+{:>4} | {}", j + 1, new[j]));
                j += 1;
            }
        }
    } else {
        let first = old.iter().zip(&new).take_while(|(a, b)| a == b).count();
        if let Some(line) = old.get(first) {
            changes.push(format!("-{:>4} | {line}", first + 1));
        }
        if let Some(line) = new.get(first) {
            changes.push(format!("+{:>4} | {line}", first + 1));
        }
    }
    let total = changes.len();
    let mut out = String::from("  (- snapshot, + rendered)\n");
    for change in changes.iter().take(DIFF_LINE_LIMIT) {
        out.push_str("  ");
        out.push_str(change);
        out.push('\n');
    }
    if total > DIFF_LINE_LIMIT {
        out.push_str(&format!(
            "  … {} more changed lines\n",
            total - DIFF_LINE_LIMIT
        ));
    }
    out
}
