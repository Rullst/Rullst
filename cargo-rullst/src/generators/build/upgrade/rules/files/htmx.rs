//! `V13-HTMX-2`: since 13.0.0-alpha.5 `cargo rullst new` writes htmx 2.0.11
//! and its SSE extension 2.2.4 to `static/`. An application created earlier
//! vendors `static/htmx-1.9.12.min.js`, which Core's `render_page*` keep
//! loading while it is the only htmx client there, so nothing breaks; the
//! finding says how to move to htmx 2 when the application is ready.

use std::path::Path;

/// The htmx 1.9.12 client of an application created before 13.0.0-alpha.5.
pub(in super::super) const LEGACY_CLIENT: &str = "static/htmx-1.9.12.min.js";
/// The htmx 2 client new applications ship.
const CURRENT_CLIENT: &str = "static/htmx-2.0.11.min.js";

/// [`LEGACY_CLIENT`] when the project at `root` has it as a regular file
/// and does not have the htmx 2 client yet.
pub(in super::super) fn legacy_client(root: &Path) -> Option<&'static str> {
    (is_file(&root.join(LEGACY_CLIENT)) && !is_file(&root.join(CURRENT_CLIENT)))
        .then_some(LEGACY_CLIENT)
}

/// A regular file, not followed through a symbolic link.
fn is_file(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|metadata| metadata.is_file())
}
