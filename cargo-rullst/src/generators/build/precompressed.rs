//! Brotli/Zstandard siblings (`static/<file>.br`, `static/<file>.zst`) that
//! `cargo rullst build` writes next to static assets.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Asset extensions `cargo rullst build` pre-compresses.
pub(crate) const COMPRESSED_EXTENSIONS: [&str; 8] =
    ["html", "css", "js", "json", "svg", "wasm", "xml", "txt"];

/// Sibling extensions, one per pre-compressed encoding.
const ENCODINGS: [&str; 2] = ["br", "zst"];

/// Returns the asset a pre-compressed sibling encodes, such as `app.css` for
/// `app.css.br`, or `None` for any other path.
pub(crate) fn sibling_source(path: &Path) -> Option<PathBuf> {
    let encoding = path.extension()?.to_str()?;
    if !ENCODINGS.contains(&encoding) {
        return None;
    }
    let source = path.with_extension("");
    let asset_extension = source.extension()?.to_str()?.to_ascii_lowercase();
    COMPRESSED_EXTENSIONS
        .contains(&asset_extension.as_str())
        .then_some(source)
}

/// Removes pre-compressed siblings under `static_dir` that are not newer than
/// the asset they encode, and returns how many were removed.
///
/// The server prefers an existing `.br`/`.zst` sibling without comparing
/// modification times, so a sibling left by an earlier build would keep
/// serving the old asset after an edit. A sibling whose asset does not exist
/// is kept, because it may be hand-made. Symbolic links to siblings are left
/// alone.
pub(crate) fn remove_stale_siblings(static_dir: &Path) -> io::Result<usize> {
    let mut removed = 0;
    for entry in walkdir::WalkDir::new(static_dir)
        .into_iter()
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let Some(source) = sibling_source(entry.path()) else {
            continue;
        };
        let Ok(source_modified) = fs::metadata(&source).and_then(|meta| meta.modified()) else {
            continue;
        };
        let sibling_modified = entry.metadata().map_err(io::Error::other)?.modified()?;
        // Equal times are ambiguous on coarse file systems: prefer freshness.
        if sibling_modified <= source_modified {
            fs::remove_file(entry.path())?;
            removed += 1;
        }
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime};

    fn write_at(path: &Path, contents: &str, modified: SystemTime) {
        fs::write(path, contents).expect("write fixture");
        fs::File::options()
            .write(true)
            .open(path)
            .and_then(|file| file.set_modified(modified))
            .expect("set fixture modification time");
    }

    #[test]
    fn siblings_map_only_to_compressed_asset_types() {
        assert_eq!(
            sibling_source(Path::new("static/app.css.br")),
            Some(PathBuf::from("static/app.css"))
        );
        assert_eq!(
            sibling_source(Path::new("static/js/app.JS.zst")),
            Some(PathBuf::from("static/js/app.JS"))
        );
        assert_eq!(sibling_source(Path::new("static/app.css")), None);
        assert_eq!(sibling_source(Path::new("static/photo.png.br")), None);
        assert_eq!(sibling_source(Path::new("static/archive.zst")), None);
    }

    #[test]
    fn stale_siblings_are_removed_and_fresh_or_hand_made_ones_kept() {
        let root = tempfile::tempdir().expect("temporary static directory");
        let static_dir = root.path();
        let earlier = SystemTime::now() - Duration::from_secs(120);
        let later = SystemTime::now() - Duration::from_secs(60);

        // Edited after the last build: the old siblings would shadow it.
        write_at(&static_dir.join("app.css"), "body{}", later);
        write_at(&static_dir.join("app.css.br"), "old", earlier);
        write_at(&static_dir.join("app.css.zst"), "old", earlier);
        // Built after the last edit.
        write_at(&static_dir.join("app.js"), "1", earlier);
        write_at(&static_dir.join("app.js.br"), "fresh", later);
        // No source asset: possibly hand-made, so never deleted.
        write_at(&static_dir.join("data.json.zst"), "manual", earlier);

        assert_eq!(remove_stale_siblings(static_dir).expect("prune"), 2);
        assert!(!static_dir.join("app.css.br").exists());
        assert!(!static_dir.join("app.css.zst").exists());
        assert!(static_dir.join("app.css").exists());
        assert!(static_dir.join("app.js.br").exists());
        assert!(static_dir.join("data.json.zst").exists());
        assert_eq!(remove_stale_siblings(static_dir).expect("prune again"), 0);
    }
}
